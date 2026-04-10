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
            self.adapter
                .send_text(&target.iterm_session_id, &format!("{prompt}\n"))
                .await
                .map_err(classify_adapter_error)?;
            tokio::time::sleep(Duration::from_millis(1200)).await;
            let screen_text = self
                .adapter
                .get_screen_text(&target.iterm_session_id)
                .await
                .map_err(classify_adapter_error)?;
            if !screen_text.trim().is_empty() {
                self.run_service
                    .record_target_interactive_output(&target.target_id, &screen_text)
                    .await?;
            }
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
                        tokio::time::sleep(Duration::from_millis(1200)).await;
                        self.adapter.get_screen_text(&target.iterm_session_id).await
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
