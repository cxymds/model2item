mod support;

use std::sync::Arc;

use async_trait::async_trait;
use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        comparison_run::CreateComparisonRunInput,
        evaluation_case::CreateEvaluationCaseInput,
        profile::CreateProfileInput,
        window_binding::CreateWindowBindingInput,
    },
    services::{
        comparison_orchestrator::ComparisonOrchestrator,
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService,
        iterm_mcp_adapter::{
            ItermExecutionRequest, ItermExecutionResult, ItermMcpAdapter, ItermSessionInfo,
        },
        profile_service::ProfileService,
        secret_store::MemorySecretStore,
        window_binding_service::WindowBindingService,
    },
};

#[derive(Clone)]
struct FakeAdapter;

#[async_trait]
impl ItermMcpAdapter for FakeAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(vec![
            ItermSessionInfo {
                session_id: "session-ok".to_string(),
                window_id: "window-1".to_string(),
                window_title: "Window 1".to_string(),
                tab_id: "tab-1".to_string(),
                tab_title: "Tab 1".to_string(),
                session_title: "Pane OK".to_string(),
            },
            ItermSessionInfo {
                session_id: "session-fail".to_string(),
                window_id: "window-2".to_string(),
                window_title: "Window 2".to_string(),
                tab_id: "tab-2".to_string(),
                tab_title: "Tab 2".to_string(),
                session_title: "Pane Fail".to_string(),
            },
        ])
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        if request.session_id == "session-fail" {
            return Err("simulated adapter failure".to_string());
        }

        Ok(ItermExecutionResult {
            output_text: format!(
                "session={} provider={} model={}",
                request.session_id, request.provider, request.model_name
            ),
        })
    }
}

#[tokio::test]
async fn executes_run_and_persists_target_outcomes() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let orchestrator =
        ComparisonOrchestrator::with_dependencies(pool.clone(), secret_store.clone(), FakeAdapter);

    let success_profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "success-secret".to_string(),
        })
        .await?;
    let fail_profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "fail-secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Old code understanding".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let success_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-ok".to_string(),
            display_name: "Window A".to_string(),
            profile_id: success_profile.id.clone(),
        })
        .await?;
    let fail_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-fail".to_string(),
            display_name: "Window B".to_string(),
            profile_id: fail_profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Legacy compare".to_string(),
            target_ids: vec![success_binding.id.clone(), fail_binding.id.clone()],
            notes: Some("Execute both windows".to_string()),
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "failed");
    assert!(updated_run.started_at.is_some());
    assert!(updated_run.finished_at.is_some());

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets.len(), 2);

    let success_target = targets
        .iter()
        .find(|target| target.window_binding_id == success_binding.id)
        .expect("success target should exist");
    assert_eq!(success_target.status, "done");
    assert_eq!(success_target.success_status.as_deref(), Some("completed"));
    assert!(success_target.sent_at.is_some());
    assert!(success_target.first_response_at.is_some());
    assert!(success_target.finished_at.is_some());
    assert!(success_target.duration_ms.is_some());
    assert!(success_target.response_chars > 0);
    assert!(success_target.response_lines > 0);
    assert_eq!(success_target.error_category, None);

    let fail_target = targets
        .iter()
        .find(|target| target.window_binding_id == fail_binding.id)
        .expect("failed target should exist");
    assert_eq!(fail_target.status, "failed");
    assert_eq!(fail_target.error_category.as_deref(), Some("adapter_error"));
    assert_eq!(
        fail_target.error_detail.as_deref(),
        Some("simulated adapter failure")
    );
    assert!(fail_target.sent_at.is_some());
    assert!(fail_target.finished_at.is_some());
    assert_eq!(fail_target.success_status.as_deref(), Some("failed"));

    let stored_messages = sqlx::query_scalar::<_, String>(
        "SELECT content FROM messages ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(stored_messages.len(), 4);
    let combined_messages = stored_messages.join("\n---\n");
    assert!(combined_messages.contains("Summarize the core control flow"));
    assert!(combined_messages.contains("session=session-ok"));
    assert!(combined_messages.contains("simulated adapter failure"));

    Ok(())
}

#[tokio::test]
async fn rejects_run_when_target_session_is_offline() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let orchestrator =
        ComparisonOrchestrator::with_dependencies(pool.clone(), secret_store.clone(), FakeAdapter);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "success-secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Old code understanding".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-offline".to_string(),
            display_name: "Window Offline".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Offline compare".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let result = orchestrator.execute_run(&run.id).await;
    assert!(matches!(result, Err(AppError::InvalidInput(_))));
    assert!(result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
        .contains("offline"));

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "queued");
    assert!(updated_run.started_at.is_none());

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets[0].status, "queued");

    Ok(())
}
