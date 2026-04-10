mod support;

use async_trait::async_trait;
use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        comparison_run::CreateComparisonRunInput,
        evaluation_case::CreateEvaluationCaseInput,
        profile::CreateProfileInput,
        window_binding::{CreateWindowBindingInput, UpdateWindowBindingInput},
    },
    services::{
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService,
        iterm_mcp_adapter::{
            ItermExecutionRequest, ItermExecutionResult, ItermMcpAdapter, ItermSessionInfo,
        },
        profile_service::ProfileService,
        secret_store::MemorySecretStore,
        window_binding_service::WindowBindingService,
        window_binding_sync_service::{
            create_window_binding_and_sync, WindowBindingSyncService,
        },
    },
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct RecordingAdapter {
    texts: Arc<Mutex<Vec<(String, String)>>>,
    screens: Arc<Mutex<HashMap<String, String>>>,
}

#[derive(Clone, Default)]
struct FailingAdapter;

#[async_trait]
impl ItermMcpAdapter for FailingAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(Vec::new())
    }

    async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), String> {
        Err("simulated iTerm sync failure".to_string())
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
impl ItermMcpAdapter for RecordingAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(Vec::new())
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.texts
            .lock()
            .expect("texts mutex")
            .push((session_id.to_string(), text.to_string()));
        self.screens
            .lock()
            .expect("screens mutex")
            .insert(session_id.to_string(), text.to_string());
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        Ok(self
            .screens
            .lock()
            .expect("screens mutex")
            .get(session_id)
            .cloned()
            .unwrap_or_default())
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
async fn creates_and_lists_window_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service = ProfileService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    assert_eq!(created.profile_id, profile.id);
    assert_eq!(created.iterm_session_id, "session-a");

    let listed = binding_service.list_window_bindings().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    Ok(())
}

#[tokio::test]
async fn rejects_window_binding_when_profile_does_not_exist(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let binding_service = WindowBindingService::new(pool);

    let result = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-missing".to_string(),
            display_name: "Missing Profile Window".to_string(),
            profile_id: "does-not-exist".to_string(),
        })
        .await;

    assert!(result.is_err());
    assert!(matches!(result, Err(AppError::Database(_))));

    Ok(())
}

#[tokio::test]
async fn syncs_last_seen_for_online_window_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service = ProfileService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let online_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-online".to_string(),
            display_name: "Window Online".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let offline_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-offline".to_string(),
            display_name: "Window Offline".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let updated = binding_service
        .refresh_presence(&["session-online".to_string()])
        .await?;

    let online = updated
        .iter()
        .find(|binding| binding.id == online_binding.id)
        .expect("online binding should exist");
    assert!(online.last_seen_at.is_some());
    assert!(!updated
        .iter()
        .any(|binding| binding.id == offline_binding.id));

    Ok(())
}

#[tokio::test]
async fn updates_window_binding_details() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let binding_service = WindowBindingService::new(pool);

    let profile_a = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let profile_b = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-2".to_string(),
        })
        .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile_a.id.clone(),
        })
        .await?;

    let updated = binding_service
        .update_window_binding(
            &created.id,
            UpdateWindowBindingInput {
                iterm_session_id: "session-b".to_string(),
                display_name: "Window B".to_string(),
                profile_id: profile_b.id.clone(),
            },
        )
        .await?;

    assert_eq!(updated.iterm_session_id, "session-b");
    assert_eq!(updated.display_name, "Window B");
    assert_eq!(updated.profile_id, profile_b.id);

    Ok(())
}

#[tokio::test]
async fn rejects_deleting_window_binding_when_it_is_referenced_by_run(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
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
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let result = binding_service.delete_window_binding(&binding.id).await;

    assert!(matches!(result, Err(AppError::InvalidInput(_))));

    Ok(())
}

#[tokio::test]
async fn allows_deleting_window_binding_when_only_finished_runs_reference_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-5.4".to_string(),
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
            iterm_session_id: "session-finished".to_string(),
            display_name: "Window Finished".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Finished Benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    run_service.finalize_run(&run.id, "done").await?;

    binding_service.delete_window_binding(&binding.id).await?;
    let bindings = binding_service.list_window_bindings().await?;
    assert!(bindings.is_empty());

    Ok(())
}

#[tokio::test]
async fn applies_binding_to_window_session_and_writes_visible_notice(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let adapter = RecordingAdapter::default();
    let sync_service =
        WindowBindingSyncService::with_dependencies(pool.clone(), adapter.clone(), secret_store);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://gateway.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-sync".to_string(),
            display_name: "Window Sync".to_string(),
            profile_id: profile.id,
        })
        .await?;

    sync_service.apply_binding(&binding.id).await?;

    let texts = adapter.texts.lock().expect("texts mutex");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].0, "session-sync");
    assert!(texts[0]
        .1
        .contains("export ANTHROPIC_MODEL='claude-sonnet-4'"));
    assert!(texts[0]
        .1
        .contains("export ANTHROPIC_BASE_URL='https://gateway.example.com'"));
    assert!(texts[0].1.contains("Bound profile"));
    assert!(texts[0].1.contains("Next run will use claude-sonnet-4"));

    Ok(())
}

#[tokio::test]
async fn rolls_back_created_binding_when_window_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let result = create_window_binding_and_sync(
        pool.clone(),
        FailingAdapter,
        secret_store,
        CreateWindowBindingInput {
            iterm_session_id: "session-sync-fail".to_string(),
            display_name: "Window Sync Fail".to_string(),
            profile_id: profile.id,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Adapter(_))));

    let binding_service = WindowBindingService::new(pool);
    let bindings = binding_service.list_window_bindings().await?;
    assert!(bindings.is_empty());

    Ok(())
}

#[tokio::test]
async fn sync_removes_unreferenced_bindings_for_closed_sessions(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service = ProfileService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let online_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-online".to_string(),
            display_name: "Window Online".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-closed".to_string(),
            display_name: "Window Closed".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let synced = binding_service
        .sync_with_online_sessions(&["session-online".to_string()])
        .await?;

    assert_eq!(synced.len(), 1);
    assert_eq!(synced[0].id, online_binding.id);

    Ok(())
}

#[tokio::test]
async fn sync_keeps_referenced_bindings_even_if_session_is_closed(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
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
        })
        .await?;

    run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let synced = binding_service.sync_with_online_sessions(&[]).await?;

    assert_eq!(synced.len(), 1);
    assert_eq!(synced[0].id, binding.id);

    Ok(())
}
