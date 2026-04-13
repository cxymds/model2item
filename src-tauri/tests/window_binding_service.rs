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
        secret_store::{MemorySecretStore, SecretStore},
        window_binding_service::WindowBindingService,
        window_binding_sync_service::{
            create_window_binding_and_sync, update_window_binding_and_sync,
            WindowBindingSyncService,
        },
    },
};
use sqlx::SqlitePool;
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

async fn insert_custom_provider(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    provider_key: &str,
    client_type: &str,
    base_url: &str,
    api_key_locator: &str,
    default_model: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO custom_providers
          (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, 1, '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z')
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(provider_key)
    .bind(client_type)
    .bind(base_url)
    .bind(api_key_locator)
    .bind(default_model)
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::test]
async fn creates_and_lists_window_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let binding_service = WindowBindingService::new(pool.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    insert_custom_provider(
        &pool,
        "provider-window-a",
        "GLM via Claude CLI",
        "glm",
        "claude_cli",
        "https://glm.example.com/v1",
        "secret://provider/window-a",
        "glm-5.1",
    )
    .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
            custom_provider_id: Some("provider-window-a".to_string()),
        })
        .await?;

    assert_eq!(created.profile_id, profile.id);
    assert_eq!(
        created.custom_provider_id.as_deref(),
        Some("provider-window-a")
    );
    assert_eq!(created.iterm_session_id, "session-a");

    let listed = binding_service.list_window_bindings().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(
        listed[0].custom_provider_id.as_deref(),
        Some("provider-window-a")
    );

    Ok(())
}

#[tokio::test]
async fn creates_binding_with_custom_provider_and_without_usable_profile_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let binding_service = WindowBindingService::new(pool.clone());

    insert_custom_provider(
        &pool,
        "provider-provider-only-create",
        "GLM Create",
        "openai-compatible",
        "claude_cli",
        "https://glm-create.example.com/v1",
        "secret://provider/create-only",
        "glm-create-model",
    )
    .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-provider-only-create".to_string(),
            display_name: "Window Provider Only Create".to_string(),
            profile_id: String::new(),
            custom_provider_id: Some("provider-provider-only-create".to_string()),
        })
        .await?;

    assert_eq!(
        created.custom_provider_id.as_deref(),
        Some("provider-provider-only-create")
    );
    assert!(!created.profile_id.is_empty());

    let profile_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM model_profiles WHERE id = ?",
    )
    .bind(&created.profile_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(profile_exists, 1);

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
            custom_provider_id: None,
        })
        .await;

    assert!(result.is_err());
    assert!(matches!(result, Err(AppError::Database(_))));

    Ok(())
}

#[tokio::test]
async fn syncs_last_seen_for_online_window_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let binding_service = WindowBindingService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
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
            custom_provider_id: None,
        })
        .await?;

    let offline_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-offline".to_string(),
            display_name: "Window Offline".to_string(),
            profile_id: profile.id.clone(),
            custom_provider_id: None,
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
    let binding_service = WindowBindingService::new(pool.clone());

    let profile_a = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let profile_b = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-2".to_string(),
        })
        .await?;

    insert_custom_provider(
        &pool,
        "provider-a",
        "Anthropic via Claude CLI",
        "anthropic",
        "claude_cli",
        "https://anthropic.example.com",
        "secret://provider/a",
        "claude-sonnet",
    )
    .await?;
    insert_custom_provider(
        &pool,
        "provider-b",
        "GPT via OpenAI Chat",
        "openai",
        "openai_chat",
        "https://openai.example.com/v1",
        "secret://provider/b",
        "gpt-5.4",
    )
    .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile_a.id.clone(),
            custom_provider_id: Some("provider-a".to_string()),
        })
        .await?;

    let updated = binding_service
        .update_window_binding(
            &created.id,
            UpdateWindowBindingInput {
                iterm_session_id: "session-b".to_string(),
                display_name: "Window B".to_string(),
                profile_id: profile_b.id.clone(),
                custom_provider_id: Some("provider-b".to_string()),
            },
        )
        .await?;

    assert_eq!(updated.iterm_session_id, "session-b");
    assert_eq!(updated.display_name, "Window B");
    assert_eq!(updated.profile_id, profile_b.id);
    assert_eq!(updated.custom_provider_id.as_deref(), Some("provider-b"));

    Ok(())
}

#[tokio::test]
async fn updates_binding_with_custom_provider_and_without_usable_profile_id(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let binding_service = WindowBindingService::new(pool.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Legacy Seed".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://legacy.example.com".to_string(),
            api_key: "legacy-secret".to_string(),
        })
        .await?;

    insert_custom_provider(
        &pool,
        "provider-provider-only-update",
        "GLM Update",
        "openai-compatible",
        "claude_cli",
        "https://glm-update.example.com/v1",
        "secret://provider/update-only",
        "glm-update-model",
    )
    .await?;

    let created = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-provider-only-update".to_string(),
            display_name: "Window Provider Only Update".to_string(),
            profile_id: profile.id,
            custom_provider_id: None,
        })
        .await?;

    let updated = binding_service
        .update_window_binding(
            &created.id,
            UpdateWindowBindingInput {
                iterm_session_id: "session-provider-only-update-next".to_string(),
                display_name: "Window Provider Only Update Next".to_string(),
                profile_id: "does-not-exist".to_string(),
                custom_provider_id: Some("provider-provider-only-update".to_string()),
            },
        )
        .await?;

    assert_eq!(
        updated.custom_provider_id.as_deref(),
        Some("provider-provider-only-update")
    );
    assert_ne!(updated.profile_id, "does-not-exist");

    let profile_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM model_profiles WHERE id = ?",
    )
    .bind(&updated.profile_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(profile_exists, 1);

    Ok(())
}

#[tokio::test]
async fn update_nonexistent_binding_returns_not_found(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let binding_service = WindowBindingService::new(pool);

    let result = binding_service
        .update_window_binding(
            "missing-binding",
            UpdateWindowBindingInput {
                iterm_session_id: "session-missing".to_string(),
                display_name: "Window Missing".to_string(),
                profile_id: "missing-profile".to_string(),
                custom_provider_id: None,
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::MissingDependency(_))));
    Ok(())
}

#[tokio::test]
async fn delete_nonexistent_binding_returns_not_found(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let binding_service = WindowBindingService::new(pool);

    let result = binding_service.delete_window_binding("missing-binding").await;

    assert!(matches!(result, Err(AppError::MissingDependency(_))));
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
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
            custom_provider_id: None,
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
            execution_mode: "openai_chat".to_string(),
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
            custom_provider_id: None,
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
    let stored = sqlx::query_as::<_, (i64, String)>(
        "SELECT enabled, metadata_json FROM window_bindings WHERE id = ? LIMIT 1",
    )
    .bind(&binding.id)
    .fetch_optional(&pool)
    .await?;
    assert!(stored.is_some());
    let (enabled, metadata_json) = stored.expect("historically referenced binding row");
    assert_eq!(enabled, 0);
    assert!(metadata_json.contains("\"deleted\":true"));

    Ok(())
}

#[tokio::test]
async fn allows_deleting_closed_window_binding_after_reconciling_active_runs(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
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

    let result = binding_service
        .delete_window_binding_after_reconciling_sessions(&binding.id, &[])
        .await;

    assert!(result.is_ok());

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "failed");

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].status, "failed");
    assert_eq!(targets[0].error_category.as_deref(), Some("session_closed"));

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
            execution_mode: "claude_cli".to_string(),
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
            custom_provider_id: None,
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
async fn applies_openai_compatible_claude_cli_binding_without_exporting_provider_env(
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
            name: "GLM via Claude CLI".to_string(),
            provider: "openai-compatible".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "glm-4.5".to_string(),
            base_url: "https://gateway.example.com/v1".to_string(),
            api_key: "glm-secret".to_string(),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-sync-openai".to_string(),
            display_name: "Window Sync OpenAI".to_string(),
            profile_id: profile.id,
            custom_provider_id: None,
        })
        .await?;

    sync_service.apply_binding(&binding.id).await?;

    let texts = adapter.texts.lock().expect("texts mutex");
    assert_eq!(texts.len(), 1);
    assert!(!texts[0].1.contains("export OPENAI_API_KEY='glm-secret'"));
    assert!(!texts[0]
        .1
        .contains("export OPENAI_BASE_URL='https://gateway.example.com/v1'"));
    assert!(!texts[0].1.contains("export OPENAI_MODEL='glm-4.5'"));
    assert!(!texts[0].1.contains("export ANTHROPIC_API_KEY"));
    assert!(texts[0].1.contains("via Provider API"));

    Ok(())
}

#[tokio::test]
async fn applies_binding_using_custom_provider_values_when_present(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let adapter = RecordingAdapter::default();
    let sync_service =
        WindowBindingSyncService::with_dependencies(pool.clone(), adapter.clone(), secret_store.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Profile Fallback".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://anthropic.example.com".to_string(),
            api_key: "profile-secret".to_string(),
        })
        .await?;

    secret_store.delete_secret(&profile.api_key_encrypted)?;

    insert_custom_provider(
        &pool,
        "provider-glm",
        "GLM via Claude CLI",
        "openai-compatible",
        "claude_cli",
        "https://glm.example.com/v1",
        "secret://provider/glm",
        "glm-5.1",
    )
    .await?;
    secret_store.set_secret("secret://provider/glm", "provider-secret")?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-provider-first".to_string(),
            display_name: "Window Provider First".to_string(),
            profile_id: profile.id,
            custom_provider_id: Some("provider-glm".to_string()),
        })
        .await?;

    sync_service.apply_binding(&binding.id).await?;

    let texts = adapter.texts.lock().expect("texts mutex");
    assert_eq!(texts.len(), 1);
    assert!(!texts[0].1.contains("export OPENAI_API_KEY='provider-secret'"));
    assert!(!texts[0]
        .1
        .contains("export OPENAI_BASE_URL='https://glm.example.com/v1'"));
    assert!(!texts[0].1.contains("export OPENAI_MODEL='glm-5.1'"));
    assert!(!texts[0].1.contains("claude-sonnet-4"));
    assert!(texts[0].1.contains("via Provider API"));

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
            execution_mode: "openai_chat".to_string(),
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
            custom_provider_id: None,
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
async fn surfaces_binding_provider_name_when_binding_sync_secret_is_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Missing Secret".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://gateway.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    secret_store.delete_secret(&profile.api_key_encrypted)?;

    let result = create_window_binding_and_sync(
        pool.clone(),
        RecordingAdapter::default(),
        secret_store,
        CreateWindowBindingInput {
            iterm_session_id: "session-missing-secret".to_string(),
            display_name: "Window Missing Secret".to_string(),
            profile_id: profile.id,
            custom_provider_id: None,
        },
    )
    .await;

    let error_message = result.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error_message.contains("binding/provider `Claude Missing Secret`"));
    assert!(error_message.contains("Window Missing Secret"));
    assert!(error_message.contains("re-save the API key"));

    let binding_service = WindowBindingService::new(pool);
    let bindings = binding_service.list_window_bindings().await?;
    assert!(bindings.is_empty());

    Ok(())
}

#[tokio::test]
async fn rolls_back_updated_binding_when_window_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let binding_service = WindowBindingService::new(pool.clone());

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Rollback Profile".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://rollback.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-rollback-initial".to_string(),
            display_name: "Window Rollback Initial".to_string(),
            profile_id: profile.id.clone(),
            custom_provider_id: None,
        })
        .await?;

    let result = update_window_binding_and_sync(
        pool.clone(),
        FailingAdapter,
        secret_store,
        &binding.id,
        UpdateWindowBindingInput {
            iterm_session_id: "session-rollback-updated".to_string(),
            display_name: "Window Rollback Updated".to_string(),
            profile_id: profile.id,
            custom_provider_id: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Adapter(_))));

    let current = binding_service.get_window_binding(&binding.id).await?;
    assert_eq!(current.iterm_session_id, "session-rollback-initial");
    assert_eq!(current.display_name, "Window Rollback Initial");

    Ok(())
}

#[tokio::test]
async fn sync_removes_unreferenced_bindings_for_closed_sessions(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let binding_service = WindowBindingService::new(pool);

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Sonnet".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
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
            custom_provider_id: None,
        })
        .await?;

    binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-closed".to_string(),
            display_name: "Window Closed".to_string(),
            profile_id: profile.id.clone(),
            custom_provider_id: None,
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
