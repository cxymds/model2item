use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::task::JoinSet;

use crate::{
    error::AppError,
    services::{
        comparison_run_service::{ComparisonRunService, ComparisonTargetExecutionRecord},
        iterm_mcp_adapter::{
            build_claude_cli_launch_command, classify_adapter_error, ItermExecutionRequest,
            ItermMcpAdapter, PythonItermMcpAdapter,
        },
        openai_chat_executor::OpenaiChatExecutor,
        secret_store::{SecretStore, SystemSecretStore},
    },
};

fn build_target_prompt(run_prompt: &str, context_snapshot: &str) -> String {
    format!("## Evaluation Prompt\n{run_prompt}\n\n## Context Payload\n{context_snapshot}\n")
}

#[derive(Clone)]
pub struct ComparisonOrchestrator<A: ItermMcpAdapter> {
    run_service: ComparisonRunService,
    secret_store: Arc<dyn SecretStore>,
    adapter: A,
    openai_executor: Arc<dyn OpenaiChatCompletionExecutor>,
}

#[async_trait]
pub trait OpenaiChatCompletionExecutor: Send + Sync {
    async fn execute_chat_completion(
        &self,
        request: &ItermExecutionRequest,
    ) -> Result<String, String>;
}

#[async_trait]
impl OpenaiChatCompletionExecutor for OpenaiChatExecutor {
    async fn execute_chat_completion(
        &self,
        request: &ItermExecutionRequest,
    ) -> Result<String, String> {
        OpenaiChatExecutor::execute_chat_completion(self, request).await
    }
}

impl ComparisonOrchestrator<PythonItermMcpAdapter> {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            run_service: ComparisonRunService::new(pool),
            secret_store: Arc::new(SystemSecretStore),
            adapter: PythonItermMcpAdapter::default(),
            openai_executor: Arc::new(OpenaiChatExecutor::default()),
        }
    }
}

impl<A: ItermMcpAdapter> ComparisonOrchestrator<A> {
    fn requires_online_session(target: &ComparisonTargetExecutionRecord) -> bool {
        target.execution_mode == "claude_cli"
    }

    fn map_secret_lookup_error(
        target_display_name: &str,
        profile_name: &str,
        error: AppError,
    ) -> AppError {
        match error {
            AppError::SecretStore(message)
                if message.contains("No matching entry found in secure storage") =>
            {
                AppError::InvalidInput(format!(
                    "cannot start target `{target_display_name}` because profile `{profile_name}` is missing its saved API key in secure storage. Open that profile and re-save the API key, then try again."
                ))
            }
            AppError::MissingDependency(message)
                if message.contains("secret not found for locator") =>
            {
                AppError::InvalidInput(format!(
                    "cannot start target `{target_display_name}` because profile `{profile_name}` is missing its saved API key in secure storage. Open that profile and re-save the API key, then try again."
                ))
            }
            other => other,
        }
    }

    async fn read_screen_text(&self, session_id: &str) -> Result<String, AppError> {
        self.adapter
            .get_screen_text(session_id)
            .await
            .map_err(classify_adapter_error)
    }

    fn extract_incremental_output(previous_screen: &str, current_screen: &str) -> String {
        let current_screen = current_screen.trim();
        let previous_screen = previous_screen.trim();

        if current_screen.is_empty() || current_screen == previous_screen {
            return String::new();
        }

        if previous_screen.is_empty() {
            return current_screen.to_string();
        }

        if let Some(suffix) = current_screen.strip_prefix(previous_screen) {
            return suffix.trim().to_string();
        }

        let mut prefix_bytes = 0usize;
        let mut previous_chars = previous_screen.chars();
        for (byte_index, current_char) in current_screen.char_indices() {
            match previous_chars.next() {
                Some(previous_char) if previous_char == current_char => {
                    prefix_bytes = byte_index + current_char.len_utf8();
                }
                _ => break,
            }
        }

        current_screen[prefix_bytes..].trim().to_string()
    }

    async fn try_record_cli_preview(
        &self,
        target_id: &str,
        session_id: &str,
        baseline_screen: &str,
    ) -> Result<(), AppError> {
        const PREVIEW_MAX_ATTEMPTS: usize = 3;
        const PREVIEW_INTERVAL_MS: u64 = 250;

        let mut previous_screen = baseline_screen.to_string();
        for attempt in 0..PREVIEW_MAX_ATTEMPTS {
            let screen_text = match self.read_screen_text(session_id).await {
                Ok(screen_text) => screen_text,
                Err(_) => return Ok(()),
            };
            let delta = Self::extract_incremental_output(&previous_screen, &screen_text);
            if !delta.is_empty() {
                self.run_service
                    .record_target_interactive_output(target_id, &delta)
                    .await?;
                return Ok(());
            }

            previous_screen = screen_text;
            if attempt + 1 < PREVIEW_MAX_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(PREVIEW_INTERVAL_MS)).await;
            }
        }

        Ok(())
    }

    async fn enrich_cli_error_with_screen(
        &self,
        session_id: &str,
        baseline_screen: &str,
        error: String,
    ) -> String {
        match self.read_screen_text(session_id).await {
            Ok(screen_text) => {
                let delta = Self::extract_incremental_output(baseline_screen, &screen_text);
                if delta.is_empty() || error.contains(&delta) {
                    error
                } else {
                    format!("{error}\n\nTerminal output:\n{delta}")
                }
            }
            Err(_) => error,
        }
    }

    pub fn with_dependencies(
        pool: sqlx::SqlitePool,
        secret_store: Arc<dyn SecretStore>,
        adapter: A,
    ) -> Self {
        Self::with_dependencies_and_openai_executor(
            pool,
            secret_store,
            adapter,
            Arc::new(OpenaiChatExecutor::default()),
        )
    }

    pub fn with_dependencies_and_openai_executor(
        pool: sqlx::SqlitePool,
        secret_store: Arc<dyn SecretStore>,
        adapter: A,
        openai_executor: Arc<dyn OpenaiChatCompletionExecutor>,
    ) -> Self {
        Self {
            run_service: ComparisonRunService::new(pool),
            secret_store,
            adapter,
            openai_executor,
        }
    }

    pub async fn validate_run_startup(&self, run_id: &str) -> Result<(), AppError> {
        let run = self.run_service.get_comparison_run(run_id).await?;
        let targets = self
            .run_service
            .list_target_execution_records(run_id)
            .await?;
        if targets.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "comparison run {run_id} has no targets"
            )));
        }

        self.ensure_targets_online(&targets).await?;

        for target in targets {
            let api_key = self
                .secret_store
                .get_secret(&target.api_key_locator)
                .map_err(|error| {
                    Self::map_secret_lookup_error(&target.display_name, &target.profile_name, error)
                })?;
            let request = ItermExecutionRequest {
                request_id: target.target_id,
                session_id: target.iterm_session_id,
                prompt: build_target_prompt(&run.prompt_snapshot, &run.context_snapshot),
                execution_mode: target.execution_mode.clone(),
                provider: target.provider,
                model_name: target.model_name,
                base_url: target.base_url,
                api_key,
                system_prompt: target.system_prompt,
                extra_params_json: target.extra_params_json,
            };

            if target.execution_mode == "claude_cli" {
                build_claude_cli_launch_command(&request).map_err(AppError::Adapter)?;
            }
        }

        Ok(())
    }

    pub async fn execute_run(&self, run_id: &str) -> Result<(), AppError> {
        let run = self.run_service.get_comparison_run(run_id).await?;
        let targets = self
            .run_service
            .list_target_execution_records(run_id)
            .await?;
        if targets.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "comparison run {run_id} has no targets"
            )));
        }

        self.ensure_targets_online(&targets).await?;

        self.run_service.mark_run_started(run_id).await?;

        let mut join_set = JoinSet::new();
        for target in targets {
            let orchestrator = self.clone();
            let run_prompt = run.prompt_snapshot.clone();
            let context_snapshot = run.context_snapshot.clone();
            join_set.spawn(async move {
                orchestrator
                    .execute_target(&run_prompt, &context_snapshot, &target)
                    .await
            });
        }

        let mut failed_count = 0usize;
        while let Some(result) = join_set.join_next().await {
            let result = result
                .map_err(|error| AppError::Adapter(format!("target execution task failed: {error}")))?;
            if result.is_err() {
                failed_count += 1;
            }
        }

        if failed_count > 0 {
            self.run_service.finalize_run(run_id, "failed").await?;
        } else {
            self.run_service.finalize_run_if_terminal(run_id).await?;
        }

        Ok(())
    }

    pub async fn broadcast_message(&self, run_id: &str, prompt: &str) -> Result<(), AppError> {
        let run = self.run_service.get_comparison_run(run_id).await?;
        if run.status != "running" {
            return Err(AppError::InvalidInput(format!(
                "comparison run {run_id} is not running"
            )));
        }

        let targets = self
            .run_service
            .list_running_target_execution_records(run_id)
            .await?;
        if targets.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "comparison run {run_id} has no active targets"
            )));
        }

        let mut join_set = JoinSet::new();
        for target in targets {
            let orchestrator = self.clone();
            let prompt = prompt.to_string();
            join_set.spawn(async move { orchestrator.broadcast_to_target(target, prompt).await });
        }

        let mut first_error = None;
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(AppError::Adapter(format!(
                            "target broadcast task failed: {error}"
                        )));
                    }
                }
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }

    async fn broadcast_to_target(
        &self,
        target: ComparisonTargetExecutionRecord,
        prompt: String,
    ) -> Result<(), AppError> {
        self.run_service
            .store_target_message(&target.target_id, "user", &prompt, "follow_up")
            .await?;

        if target.execution_mode == "openai_chat" {
            let api_key = self
                .secret_store
                .get_secret(&target.api_key_locator)
                .map_err(|error| {
                    Self::map_secret_lookup_error(&target.display_name, &target.profile_name, error)
                })?;
            let request = ItermExecutionRequest {
                request_id: target.target_id.clone(),
                session_id: target.iterm_session_id.clone(),
                prompt,
                execution_mode: target.execution_mode.clone(),
                provider: target.provider.clone(),
                model_name: target.model_name.clone(),
                base_url: target.base_url.clone(),
                api_key,
                system_prompt: target.system_prompt.clone(),
                extra_params_json: target.extra_params_json.clone(),
            };

            match self.openai_executor.execute_chat_completion(&request).await {
                Ok(output_text) => {
                    self.run_service
                        .record_target_interactive_output(&target.target_id, &output_text)
                        .await?;
                    Ok(())
                }
                Err(error) => {
                    self.run_service
                        .mark_target_failed(&target.target_id, "execution_error", &error)
                        .await?;
                    Err(AppError::Adapter(error))
                }
            }
        } else {
            let baseline_screen = self
                .read_screen_text(&target.iterm_session_id)
                .await
                .unwrap_or_default();
            match self
                .adapter
                .send_text(&target.iterm_session_id, &format!("{prompt}\n"))
                .await
            {
                Ok(()) => {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let baseline_after_prompt = self
                        .read_screen_text(&target.iterm_session_id)
                        .await
                        .unwrap_or_else(|_| baseline_screen.clone());
                    let baseline = if baseline_after_prompt.trim().is_empty() {
                        baseline_screen
                    } else {
                        baseline_after_prompt
                    };
                    self.try_record_cli_preview(
                        &target.target_id,
                        &target.iterm_session_id,
                        &baseline,
                    )
                    .await?;
                    Ok(())
                }
                Err(error) => {
                    let error = classify_adapter_error(error);
                    self.run_service
                        .mark_target_failed(&target.target_id, "adapter_error", &error.to_string())
                        .await?;
                    Err(error)
                }
            }
        }
    }

    async fn ensure_targets_online(
        &self,
        targets: &[ComparisonTargetExecutionRecord],
    ) -> Result<(), AppError> {
        let terminal_targets = targets
            .iter()
            .filter(|target| Self::requires_online_session(target))
            .collect::<Vec<_>>();

        if terminal_targets.is_empty() {
            return Ok(());
        }

        let online_session_ids = self
            .adapter
            .list_sessions()
            .await
            .map_err(classify_adapter_error)?
            .into_iter()
            .map(|session| session.session_id)
            .collect::<std::collections::HashSet<_>>();

        let offline_targets = terminal_targets
            .iter()
            .filter(|target| !online_session_ids.contains(&target.iterm_session_id))
            .map(|target| format!("{} ({})", target.display_name, target.iterm_session_id))
            .collect::<Vec<_>>();

        if offline_targets.is_empty() {
            Ok(())
        } else {
            Err(AppError::InvalidInput(format!(
                "cannot start comparison run because these target windows are offline: {}",
                offline_targets.join(", ")
            )))
        }
    }

    async fn execute_target(
        &self,
        run_prompt: &str,
        context_snapshot: &str,
        target: &ComparisonTargetExecutionRecord,
    ) -> Result<(), AppError> {
        self.run_service
            .mark_target_running(&target.target_id)
            .await?;

        let request_prompt = build_target_prompt(run_prompt, context_snapshot);
        self.run_service
            .store_target_message(&target.target_id, "user", &request_prompt, "prompt")
            .await?;

        let api_key = self
            .secret_store
            .get_secret(&target.api_key_locator)
            .map_err(|error| {
                Self::map_secret_lookup_error(&target.display_name, &target.profile_name, error)
            })?;
        let request = ItermExecutionRequest {
            request_id: target.target_id.clone(),
            session_id: target.iterm_session_id.clone(),
            prompt: request_prompt.clone(),
            execution_mode: target.execution_mode.clone(),
            provider: target.provider.clone(),
            model_name: target.model_name.clone(),
            base_url: target.base_url.clone(),
            api_key,
            system_prompt: target.system_prompt.clone(),
            extra_params_json: target.extra_params_json.clone(),
        };

        if target.execution_mode == "openai_chat" {
            match self.openai_executor.execute_chat_completion(&request).await {
                Ok(output_text) => {
                    self.run_service
                        .mark_target_completed(&target.target_id, &output_text)
                        .await
                }
                Err(error) => {
                    self.run_service
                        .mark_target_failed(&target.target_id, "execution_error", &error)
                        .await?;
                    Err(AppError::Adapter(error))
                }
            }
        } else {
            let baseline_before_launch = String::new();
            let launch_command = build_claude_cli_launch_command(&request);
            let result = match launch_command {
                Ok(command) => {
                    if let Err(error) = self
                        .adapter
                        .send_text(&target.iterm_session_id, &command)
                        .await
                    {
                        Err(
                            self.enrich_cli_error_with_screen(
                                &target.iterm_session_id,
                                &baseline_before_launch,
                                error,
                            )
                            .await,
                        )
                    } else {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        match self
                            .adapter
                            .send_text(&target.iterm_session_id, &format!("{request_prompt}\n"))
                            .await
                        {
                            Ok(()) => {
                                tokio::time::sleep(Duration::from_millis(250)).await;
                                let baseline_after_prompt = self
                                    .read_screen_text(&target.iterm_session_id)
                                    .await
                                    .unwrap_or_default();
                                self.try_record_cli_preview(
                                    &target.target_id,
                                    &target.iterm_session_id,
                                    &baseline_after_prompt,
                                )
                                .await?;
                                Ok(())
                            }
                            Err(error) => Err(
                                self.enrich_cli_error_with_screen(
                                    &target.iterm_session_id,
                                    &baseline_before_launch,
                                    error,
                                )
                                .await,
                            ),
                        }
                    }
                }
                Err(error) => Err(error),
            };

            match result {
                Ok(()) => Ok(()),
                Err(error) => {
                    self.run_service
                        .mark_target_failed(&target.target_id, "adapter_error", &error)
                        .await?;
                    Err(classify_adapter_error(error))
                }
            }
        }
    }
}
