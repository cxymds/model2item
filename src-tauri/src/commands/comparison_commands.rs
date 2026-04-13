use tauri::{Manager, State};

use crate::{
    app_state::AppState,
    error::AppError,
    models::comparison_run::{
        ComparisonMessageResponse, ComparisonRunResponse, ComparisonSummaryResponse,
        ComparisonTargetResponse, CreateComparisonRunInput,
    },
    services::{
        analysis_service::AnalysisService, comparison_orchestrator::ComparisonOrchestrator,
        comparison_run_service::ComparisonRunService, iterm_mcp_adapter::ItermMcpAdapter,
        iterm_session_service::ItermSessionService,
    },
};

async fn reconcile_runtime_state(pool: sqlx::SqlitePool) -> Result<(), AppError> {
    let session_service = ItermSessionService::new();
    reconcile_runtime_state_with_session_service(pool, &session_service).await
}

async fn reconcile_runtime_state_with_session_service<A: ItermMcpAdapter>(
    pool: sqlx::SqlitePool,
    session_service: &ItermSessionService<A>,
) -> Result<(), AppError> {
    let sessions = session_service.list_sessions().await?;
    let online_session_ids = sessions
        .into_iter()
        .map(|session| session.session_id)
        .collect::<Vec<_>>();

    ComparisonRunService::new(pool)
        .reconcile_closed_sessions(&online_session_ids)
        .await
}

async fn reconcile_runtime_state_best_effort(pool: sqlx::SqlitePool) {
    if let Err(error) = reconcile_runtime_state(pool).await {
        eprintln!("skipping runtime reconcile because iTerm session listing failed: {error}");
    }
}

#[tauri::command]
pub async fn create_comparison_run(
    state: State<'_, AppState>,
    input: CreateComparisonRunInput,
) -> Result<ComparisonRunResponse, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let run = service
        .create_comparison_run(input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(run.into())
}

#[tauri::command]
pub async fn get_comparison_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<ComparisonRunResponse, String> {
    reconcile_runtime_state_best_effort(state.pool.clone()).await;
    let service = ComparisonRunService::new(state.pool.clone());
    let run = service
        .get_comparison_run(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(run.into())
}

#[tauri::command]
pub async fn list_comparison_runs(
    state: State<'_, AppState>,
) -> Result<Vec<ComparisonRunResponse>, String> {
    reconcile_runtime_state_best_effort(state.pool.clone()).await;
    let service = ComparisonRunService::new(state.pool.clone());
    let runs = service
        .list_comparison_runs(20)
        .await
        .map_err(|error| error.to_string())?;
    Ok(runs.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn list_comparison_targets(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<ComparisonTargetResponse>, String> {
    reconcile_runtime_state_best_effort(state.pool.clone()).await;
    let service = ComparisonRunService::new(state.pool.clone());
    let targets = service
        .list_comparison_targets(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(targets.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn list_target_messages(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<Vec<ComparisonMessageResponse>, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let messages = service
        .list_target_messages(&target_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(messages.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn get_comparison_summary(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<ComparisonSummaryResponse, String> {
    let service = AnalysisService::new(state.pool.clone());
    service
        .get_comparison_summary(&run_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn export_comparison_run_report(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<String, String> {
    let service = AnalysisService::new(state.pool.clone());
    let export_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("exports");
    let path = service
        .export_comparison_run_report(&run_id, &export_dir)
        .await
        .map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn start_comparison_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let pool = state.pool.clone();
    let run_service = ComparisonRunService::new(pool.clone());
    let run = run_service
        .get_comparison_run(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(active_run) = run_service
        .get_active_comparison_run()
        .await
        .map_err(|error| error.to_string())?
    {
        if active_run.id != run_id {
            return Err(format!(
                "invalid input: an active comparison run already exists: {}",
                active_run.title
            ));
        }
    }
    if run.status == "running" {
        return Err(format!(
            "invalid input: comparison run {} is already running",
            run.title
        ));
    }

    let orchestrator = ComparisonOrchestrator::new(pool.clone());
    orchestrator
        .validate_run_startup(&run_id)
        .await
        .map_err(|error| error.to_string())?;

    tauri::async_runtime::spawn(async move {
        let orchestrator = ComparisonOrchestrator::new(pool);
        if let Err(error) = orchestrator.execute_run(&run_id).await {
            eprintln!("failed to execute comparison run {}: {}", run_id, error);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn send_comparison_run_message(
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let orchestrator = ComparisonOrchestrator::new(state.pool.clone());
    orchestrator
        .broadcast_message(&run_id, &prompt)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::reconcile_runtime_state_with_session_service;

    use crate::{
        db,
        models::{
            comparison_run::CreateComparisonRunInput, evaluation_case::CreateEvaluationCaseInput,
            profile::CreateProfileInput, window_binding::CreateWindowBindingInput,
        },
        services::{
            comparison_run_service::ComparisonRunService,
            evaluation_case_service::EvaluationCaseService,
            iterm_mcp_adapter::{ItermExecutionRequest, ItermExecutionResult, ItermMcpAdapter, ItermSessionInfo},
            iterm_session_service::ItermSessionService,
            profile_service::ProfileService,
            secret_store::MemorySecretStore,
            window_binding_service::WindowBindingService,
        },
    };

    #[derive(Clone, Default)]
    struct EmptySessionAdapter;

    #[derive(Clone)]
    struct FailingSessionAdapter {
        message: String,
    }

    #[async_trait]
    impl ItermMcpAdapter for EmptySessionAdapter {
        async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
            Ok(Vec::new())
        }

        async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), String> {
            Ok(())
        }

        async fn get_screen_text(&self, _session_id: &str) -> Result<String, String> {
            Ok(String::new())
        }

        async fn execute_prompt(
            &self,
            _request: ItermExecutionRequest,
        ) -> Result<ItermExecutionResult, String> {
            Ok(ItermExecutionResult {
                output_text: String::new(),
            })
        }
    }

    #[async_trait]
    impl ItermMcpAdapter for FailingSessionAdapter {
        async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
            Err(self.message.clone())
        }

        async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), String> {
            Ok(())
        }

        async fn get_screen_text(&self, _session_id: &str) -> Result<String, String> {
            Ok(String::new())
        }

        async fn execute_prompt(
            &self,
            _request: ItermExecutionRequest,
        ) -> Result<ItermExecutionResult, String> {
            Ok(ItermExecutionResult {
                output_text: String::new(),
            })
        }
    }

    #[tokio::test]
    async fn query_side_reconcile_marks_closed_running_targets_failed(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = db::connect("sqlite::memory:").await?;
        let secret_store = Arc::new(MemorySecretStore::default());
        let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store);
        let case_service = EvaluationCaseService::new(pool.clone());
        let binding_service = WindowBindingService::new(pool.clone());
        let run_service = ComparisonRunService::new(pool.clone());

        let profile = profile_service
            .create_profile(CreateProfileInput {
                name: "Claude".to_string(),
                provider: "anthropic".to_string(),
                execution_mode: "claude_cli".to_string(),
                model_name: "claude-sonnet".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "secret".to_string(),
            })
            .await?;

        let case = case_service
            .create_evaluation_case(CreateEvaluationCaseInput {
                title: "Legacy parser".to_string(),
                prompt: "Explain parser".to_string(),
                context_payload: "{}".to_string(),
                notes: None,
            })
            .await?;

        let binding = binding_service
            .create_window_binding(CreateWindowBindingInput {
                iterm_session_id: "session-closed".to_string(),
                display_name: "Window Closed".to_string(),
                profile_id: profile.id.clone(),
                custom_provider_id: None,
            })
            .await?;

        let run = run_service
            .create_comparison_run(CreateComparisonRunInput {
                evaluation_case_id: case.id,
                title: "Queued Benchmark".to_string(),
                target_ids: vec![binding.id.clone()],
                notes: None,
            })
            .await?;

        run_service.mark_run_started(&run.id).await?;
        let targets = run_service.list_comparison_targets(&run.id).await?;
        run_service.mark_target_running(&targets[0].id).await?;

        let session_service = ItermSessionService::with_adapter(EmptySessionAdapter);
        reconcile_runtime_state_with_session_service(pool.clone(), &session_service).await?;

        let updated_run = run_service.get_comparison_run(&run.id).await?;
        assert_eq!(updated_run.status, "failed");

        let updated_targets = run_service.list_comparison_targets(&run.id).await?;
        assert_eq!(updated_targets[0].status, "failed");
        assert_eq!(
            updated_targets[0].error_category.as_deref(),
            Some("session_closed")
        );

        Ok(())
    }

    #[tokio::test]
    async fn query_side_reconcile_tolerates_iterm_connection_errors(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pool = db::connect("sqlite::memory:").await?;
        let secret_store = Arc::new(MemorySecretStore::default());
        let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store);
        let case_service = EvaluationCaseService::new(pool.clone());
        let binding_service = WindowBindingService::new(pool.clone());
        let run_service = ComparisonRunService::new(pool.clone());

        let profile = profile_service
            .create_profile(CreateProfileInput {
                name: "Claude".to_string(),
                provider: "anthropic".to_string(),
                execution_mode: "claude_cli".to_string(),
                model_name: "claude-sonnet".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "secret".to_string(),
            })
            .await?;

        let case = case_service
            .create_evaluation_case(CreateEvaluationCaseInput {
                title: "Legacy parser".to_string(),
                prompt: "Explain parser".to_string(),
                context_payload: "{}".to_string(),
                notes: None,
            })
            .await?;

        let binding = binding_service
            .create_window_binding(CreateWindowBindingInput {
                iterm_session_id: "session-stale".to_string(),
                display_name: "Window Stale".to_string(),
                profile_id: profile.id.clone(),
                custom_provider_id: None,
            })
            .await?;

        let run = run_service
            .create_comparison_run(CreateComparisonRunInput {
                evaluation_case_id: case.id,
                title: "Running Benchmark".to_string(),
                target_ids: vec![binding.id.clone()],
                notes: None,
            })
            .await?;

        run_service.mark_run_started(&run.id).await?;
        let targets = run_service.list_comparison_targets(&run.id).await?;
        run_service.mark_target_running(&targets[0].id).await?;

        let session_service = ItermSessionService::with_adapter(FailingSessionAdapter {
            message: "There was a problem connecting to iTerm2".to_string(),
        });
        let reconcile_result =
            reconcile_runtime_state_with_session_service(pool.clone(), &session_service).await;

        assert!(reconcile_result.is_err());

        let persisted_run = run_service.get_comparison_run(&run.id).await?;
        assert_eq!(persisted_run.status, "running");

        let persisted_targets = run_service.list_comparison_targets(&run.id).await?;
        assert_eq!(persisted_targets.len(), 1);
        assert_eq!(persisted_targets[0].status, "running");

        Ok(())
    }
}
