use std::sync::Arc;
use std::time::Duration;

use crate::{
    error::AppError,
    services::{
        comparison_run_service::{ComparisonRunService, ComparisonTargetExecutionRecord},
        iterm_mcp_adapter::{
            build_interactive_claude_launch_command, classify_adapter_error, ItermExecutionRequest,
            ItermMcpAdapter, PythonItermMcpAdapter,
        },
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
}

impl ComparisonOrchestrator<PythonItermMcpAdapter> {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            run_service: ComparisonRunService::new(pool),
            secret_store: Arc::new(SystemSecretStore),
            adapter: PythonItermMcpAdapter::default(),
        }
    }
}

impl<A: ItermMcpAdapter> ComparisonOrchestrator<A> {
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

    async fn capture_incremental_interactive_output(
        &self,
        session_id: &str,
        baseline_screen: &str,
    ) -> Result<(String, String), AppError> {
        const MAX_SCREEN_READ_ATTEMPTS: usize = 20;
        const SCREEN_READ_INTERVAL_MS: u64 = 500;

        for attempt in 0..MAX_SCREEN_READ_ATTEMPTS {
            let screen_text = self.read_screen_text(session_id).await?;
            let delta = Self::extract_incremental_output(baseline_screen, &screen_text);
            if !delta.is_empty() {
                return Ok((screen_text, delta));
            }

            if attempt + 1 < MAX_SCREEN_READ_ATTEMPTS {
                tokio::time::sleep(Duration::from_millis(SCREEN_READ_INTERVAL_MS)).await;
            }
        }

        Err(AppError::Adapter(format!(
            "timed out waiting for new interactive output from session {session_id}"
        )))
    }

    pub fn with_dependencies(
        pool: sqlx::SqlitePool,
        secret_store: Arc<dyn SecretStore>,
        adapter: A,
    ) -> Self {
        Self {
            run_service: ComparisonRunService::new(pool),
            secret_store,
            adapter,
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
            let api_key = self.secret_store.get_secret(&target.api_key_locator)?;
            let request = ItermExecutionRequest {
                request_id: target.target_id,
                session_id: target.iterm_session_id,
                prompt: build_target_prompt(&run.prompt_snapshot, &run.context_snapshot),
                provider: target.provider,
                model_name: target.model_name,
                base_url: target.base_url,
                api_key,
                system_prompt: target.system_prompt,
                extra_params_json: target.extra_params_json,
            };

            build_interactive_claude_launch_command(&request).map_err(AppError::Adapter)?;
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

        let mut failed_count = 0usize;
        for target in targets {
            if self
                .execute_target(&run.prompt_snapshot, &run.context_snapshot, &target)
                .await
                .is_err()
            {
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

        for target in targets {
            self.run_service
                .store_target_message(&target.target_id, "user", prompt, "follow_up")
                .await?;
            let baseline_screen = self.read_screen_text(&target.iterm_session_id).await?;
            self.adapter
                .send_text(&target.iterm_session_id, &format!("{prompt}\n"))
                .await
                .map_err(classify_adapter_error)?;
            tokio::time::sleep(Duration::from_millis(250)).await;
            let baseline_after_prompt = self.read_screen_text(&target.iterm_session_id).await?;
            let baseline = if baseline_after_prompt.trim().is_empty() {
                baseline_screen.as_str()
            } else {
                baseline_after_prompt.as_str()
            };
            let (_, delta) = self
                .capture_incremental_interactive_output(&target.iterm_session_id, baseline)
                .await?;
            self.run_service
                .record_target_interactive_output(&target.target_id, &delta)
                .await?;
        }

        Ok(())
    }

    async fn ensure_targets_online(
        &self,
        targets: &[ComparisonTargetExecutionRecord],
    ) -> Result<(), AppError> {
        let online_session_ids = self
            .adapter
            .list_sessions()
            .await
            .map_err(classify_adapter_error)?
            .into_iter()
            .map(|session| session.session_id)
            .collect::<std::collections::HashSet<_>>();

        let offline_targets = targets
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

        let api_key = self.secret_store.get_secret(&target.api_key_locator)?;
        let request = ItermExecutionRequest {
            request_id: target.target_id.clone(),
            session_id: target.iterm_session_id.clone(),
            prompt: request_prompt.clone(),
            provider: target.provider.clone(),
            model_name: target.model_name.clone(),
            base_url: target.base_url.clone(),
            api_key,
            system_prompt: target.system_prompt.clone(),
            extra_params_json: target.extra_params_json.clone(),
        };

        let launch_command = build_interactive_claude_launch_command(&request);
        let result = match launch_command {
            Ok(command) => {
                if let Err(error) = self
                    .adapter
                    .send_text(&target.iterm_session_id, &command)
                    .await
                {
                    Err(error)
                } else {
                    tokio::time::sleep(Duration::from_millis(1200)).await;
                    if let Err(error) = self
                        .adapter
                        .send_text(&target.iterm_session_id, &format!("{request_prompt}\n"))
                        .await
                    {
                        Err(error)
                    } else {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        match self.read_screen_text(&target.iterm_session_id).await {
                            Ok(baseline_after_prompt) => self
                                .capture_incremental_interactive_output(
                                    &target.iterm_session_id,
                                    &baseline_after_prompt,
                                )
                                .await
                                .map(|(_, delta)| delta)
                                .map_err(|error| error.to_string()),
                            Err(error) => Err(error.to_string()),
                        }
                    }
                }
            }
            Err(error) => Err(error),
        };

        match result {
            Ok(output_text) => {
                self.run_service
                    .record_target_interactive_output(&target.target_id, &output_text)
                    .await
            }
            Err(error) => {
                self.run_service
                    .mark_target_failed(&target.target_id, "adapter_error", &error)
                    .await?;
                Err(classify_adapter_error(error))
            }
        }
    }
}
