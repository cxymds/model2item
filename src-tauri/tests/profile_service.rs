mod support;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        profile::{CreateProfileInput, UpdateProfileInput},
        comparison_run::CreateComparisonRunInput,
        evaluation_case::CreateEvaluationCaseInput,
        window_binding::CreateWindowBindingInput,
    },
    services::{
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService,
        profile_service::ProfileService,
        secret_store::{profile_secret_locator, MemorySecretStore, SecretStore},
        window_binding_service::WindowBindingService,
    },
};

#[derive(Debug, Default)]
struct WriteOnlySecretStore {
    secrets: Mutex<HashMap<String, String>>,
}

impl SecretStore for WriteOnlySecretStore {
    fn set_secret(&self, locator: &str, secret: &str) -> Result<(), AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStore("write-only secret store lock poisoned".to_string()))?
            .insert(locator.to_string(), secret.to_string());
        Ok(())
    }

    fn get_secret(&self, locator: &str) -> Result<String, AppError> {
        if self
            .secrets
            .lock()
            .map_err(|_| AppError::SecretStore("write-only secret store lock poisoned".to_string()))?
            .contains_key(locator)
        {
            Err(AppError::SecretStore(
                "No matching entry found in secure storage".to_string(),
            ))
        } else {
            Err(AppError::MissingDependency(format!(
                "secret not found for locator {locator}"
            )))
        }
    }

    fn delete_secret(&self, locator: &str) -> Result<(), AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::SecretStore("write-only secret store lock poisoned".to_string()))?
            .remove(locator);
        Ok(())
    }
}

#[tokio::test]
async fn creates_core_tables() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let tables = support::table_names(&pool).await?;
    let foreign_keys_on = support::foreign_keys_enabled(&pool).await?;

    let required = vec![
        iterm_mcp_tools_lib::db::schema::MODEL_PROFILES,
        iterm_mcp_tools_lib::db::schema::WINDOW_BINDINGS,
        iterm_mcp_tools_lib::db::schema::EVALUATION_CASES,
        iterm_mcp_tools_lib::db::schema::COMPARISON_RUNS,
        iterm_mcp_tools_lib::db::schema::COMPARISON_TARGETS,
        iterm_mcp_tools_lib::db::schema::MESSAGES,
        iterm_mcp_tools_lib::db::schema::ANALYSIS_RESULTS,
        iterm_mcp_tools_lib::db::schema::TARGET_EVALUATIONS,
    ];

    for table in required {
        assert!(
            tables.iter().any(|name| name == table),
            "expected table `{table}` to exist"
        );
    }
    assert!(
        foreign_keys_on,
        "expected SQLite foreign_keys pragma to be ON"
    );

    Ok(())
}

#[tokio::test]
async fn creates_and_lists_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let secret = "plain-text-for-now";

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: secret.to_string(),
        })
        .await?;

    assert_eq!(created.provider, "openai");
    assert_eq!(created.execution_mode, "openai_chat");
    assert_eq!(created.model_name, "gpt-5.4");
    assert_ne!(created.api_key_encrypted, secret);
    assert!(
        created.api_key_encrypted.starts_with("secret://"),
        "expected secret locator, got {}",
        created.api_key_encrypted
    );

    let stored_locator = sqlx::query_scalar::<_, String>(
        "SELECT api_key_encrypted FROM model_profiles WHERE id = ?",
    )
    .bind(&created.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored_locator, created.api_key_encrypted);
    assert_ne!(stored_locator, secret);
    assert_eq!(
        secret_store
            .get_secret(&profile_secret_locator(&created.id))
            .as_deref(),
        Some(secret)
    );

    let listed = service.list_profiles().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let fetched = service.get_profile(&created.id).await?;
    assert_eq!(fetched.name, "GPT 5.4");

    Ok(())
}

#[tokio::test]
async fn updates_profile_fields_and_secret() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let updated = service
        .update_profile(
            &created.id,
            UpdateProfileInput {
                name: "Claude Sonnet".to_string(),
                provider: "anthropic".to_string(),
                execution_mode: "claude_cli".to_string(),
                model_name: "claude-sonnet-4".to_string(),
                base_url: "https://api.anthropic.example.com".to_string(),
                api_key: "secret-2".to_string(),
            },
        )
        .await?;

    assert_eq!(updated.name, "Claude Sonnet");
    assert_eq!(updated.provider, "anthropic");
    assert_eq!(updated.model_name, "claude-sonnet-4");
    assert_eq!(updated.base_url, "https://api.anthropic.example.com");
    assert_eq!(
        secret_store
            .get_secret(&profile_secret_locator(&created.id))
            .as_deref(),
        Some("secret-2")
    );

    Ok(())
}

#[tokio::test]
async fn keeps_existing_secret_when_updating_profile_without_new_api_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let updated = service
        .update_profile(
            &created.id,
            UpdateProfileInput {
                name: "GPT 5.4 updated".to_string(),
                provider: "openai".to_string(),
                execution_mode: "openai_chat".to_string(),
                model_name: "gpt-5.4-mini".to_string(),
                base_url: "https://api.example.com/v2".to_string(),
                api_key: "".to_string(),
            },
        )
        .await?;

    assert_eq!(updated.name, "GPT 5.4 updated");
    assert_eq!(updated.model_name, "gpt-5.4-mini");
    assert_eq!(updated.base_url, "https://api.example.com/v2");
    assert_eq!(updated.api_key_encrypted, created.api_key_encrypted);
    assert_eq!(
        secret_store
            .get_secret(&profile_secret_locator(&created.id))
            .as_deref(),
        Some("secret-1")
    );

    Ok(())
}

#[tokio::test]
async fn returns_saved_profile_api_key_for_editing() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store);

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let api_key = service.get_profile_api_key(&created.id).await?;
    assert_eq!(api_key.as_deref(), Some("secret-1"));

    Ok(())
}

#[tokio::test]
async fn returns_none_when_profile_secret_is_missing() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    secret_store.delete_secret(&profile_secret_locator(&created.id))?;

    let api_key = service.get_profile_api_key(&created.id).await?;
    assert_eq!(api_key, None);

    Ok(())
}

#[tokio::test]
async fn rejects_creating_profile_when_secret_cannot_be_read_back(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service =
        ProfileService::with_secret_store(pool, Arc::new(WriteOnlySecretStore::default()));

    let result = service
        .create_profile(CreateProfileInput {
            name: "Broken Secure Store".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await;

    assert!(matches!(result, Err(AppError::SecretStore(_))));

    Ok(())
}

#[tokio::test]
async fn rejects_updating_profile_when_secret_cannot_be_read_back(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(WriteOnlySecretStore::default()));

    sqlx::query(
        r#"
        INSERT INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind("profile-1")
    .bind("Broken Secure Store")
    .bind("openai")
    .bind("openai_chat")
    .bind("gpt-5.4")
    .bind("https://api.example.com/v1")
    .bind(profile_secret_locator("profile-1"))
    .bind("2026-04-10T00:00:00Z")
    .bind("2026-04-10T00:00:00Z")
    .execute(&pool)
    .await?;

    let result = service
        .update_profile(
            "profile-1",
            UpdateProfileInput {
                name: "Broken Secure Store Updated".to_string(),
                provider: "openai".to_string(),
                execution_mode: "openai_chat".to_string(),
                model_name: "gpt-5.4-mini".to_string(),
                base_url: "https://api.example.com/v2".to_string(),
                api_key: "secret-2".to_string(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::SecretStore(_))));

    Ok(())
}

#[tokio::test]
async fn deletes_profile_when_not_bound() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    service.delete_profile(&created.id).await?;
    let listed = service.list_profiles().await?;
    assert!(listed.is_empty());
    assert!(secret_store
        .get_secret(&profile_secret_locator(&created.id))
        .is_none());

    Ok(())
}

#[tokio::test]
async fn deletes_profile_and_unreferenced_bindings_together(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let binding_service = WindowBindingService::new(pool.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-1".to_string(),
            display_name: "Window A".to_string(),
            profile_id: created.id.clone(),
        })
        .await?;

    service.delete_profile(&created.id).await?;

    let listed_profiles = service.list_profiles().await?;
    assert!(listed_profiles.is_empty());
    let listed_bindings = binding_service.list_window_bindings().await?;
    assert!(listed_bindings.is_empty());
    assert!(secret_store
        .get_secret(&profile_secret_locator(&created.id))
        .is_none());

    Ok(())
}

#[tokio::test]
async fn rejects_deleting_profile_when_its_binding_is_referenced_by_active_run(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store);
    let binding_service = WindowBindingService::new(pool.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-1".to_string(),
            display_name: "Window A".to_string(),
            profile_id: created.id.clone(),
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

    run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let result = service.delete_profile(&created.id).await;
    assert!(matches!(result, Err(AppError::InvalidInput(message)) if message.contains("active window bindings")));

    Ok(())
}

#[tokio::test]
async fn soft_deletes_profile_when_finished_run_history_still_references_its_bindings(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());

    let created = service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-history".to_string(),
            display_name: "Window History".to_string(),
            profile_id: created.id.clone(),
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

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Finished Benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    run_service.finalize_run(&run.id, "done").await?;

    service.delete_profile(&created.id).await?;

    let listed_profiles = service.list_profiles().await?;
    assert!(listed_profiles.is_empty());

    let raw_enabled = sqlx::query_scalar::<_, i64>(
        "SELECT enabled FROM model_profiles WHERE id = ?",
    )
    .bind(&created.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(raw_enabled, 0);

    let bindings = binding_service.list_window_bindings().await?;
    assert!(bindings.is_empty());
    assert!(secret_store
        .get_secret(&profile_secret_locator(&created.id))
        .is_none());

    Ok(())
}

#[tokio::test]
async fn creates_profile_with_execution_mode_and_normalizes_provider(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool, secret_store);

    let created = service
        .create_profile(CreateProfileInput {
            name: "OpenAI via chat".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    assert_eq!(created.execution_mode, "openai_chat");
    assert_eq!(created.provider, "anthropic");
    Ok(())
}

#[tokio::test]
async fn preserves_custom_provider_for_claude_cli_profiles(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool, secret_store);

    let created = service
        .create_profile(CreateProfileInput {
            name: "GLM via Claude CLI".to_string(),
            provider: "openai-compatible".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "glm-4.5".to_string(),
            base_url: "https://gateway.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    assert_eq!(created.execution_mode, "claude_cli");
    assert_eq!(created.provider, "openai-compatible");
    Ok(())
}

#[tokio::test]
async fn updates_execution_mode_and_normalizes_provider() -> Result<(), Box<dyn std::error::Error>>
{
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool, secret_store);

    let created = service
        .create_profile(CreateProfileInput {
            name: "Claude CLI".to_string(),
            provider: "openai".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;
    assert_eq!(created.execution_mode, "claude_cli");
    assert_eq!(created.provider, "openai");

    let updated = service
        .update_profile(
            &created.id,
            UpdateProfileInput {
                name: "OpenAI Chat".to_string(),
                provider: "anthropic".to_string(),
                execution_mode: "openai_chat".to_string(),
                model_name: "gpt-5.4".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "secret-2".to_string(),
            },
        )
        .await?;

    assert_eq!(updated.execution_mode, "openai_chat");
    assert_eq!(updated.provider, "anthropic");

    Ok(())
}

#[tokio::test]
async fn updates_claude_cli_profile_without_overwriting_custom_provider(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool, secret_store);

    let created = service
        .create_profile(CreateProfileInput {
            name: "GLM via Claude CLI".to_string(),
            provider: "openai-compatible".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "glm-4.5".to_string(),
            base_url: "https://gateway.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
        })
        .await?;

    let updated = service
        .update_profile(
            &created.id,
            UpdateProfileInput {
                name: "GLM via Claude CLI Updated".to_string(),
                provider: "openai-compatible".to_string(),
                execution_mode: "claude_cli".to_string(),
                model_name: "glm-4.5-air".to_string(),
                base_url: "https://gateway.example.com/v2".to_string(),
                api_key: "".to_string(),
            },
        )
        .await?;

    assert_eq!(updated.execution_mode, "claude_cli");
    assert_eq!(updated.provider, "openai-compatible");
    assert_eq!(updated.model_name, "glm-4.5-air");

    Ok(())
}
