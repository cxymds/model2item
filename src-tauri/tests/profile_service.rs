mod support;

use std::sync::Arc;

use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        profile::{CreateProfileInput, UpdateProfileInput},
        window_binding::CreateWindowBindingInput,
    },
    services::{
        profile_service::ProfileService,
        secret_store::{profile_secret_locator, MemorySecretStore},
        window_binding_service::WindowBindingService,
    },
};

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
async fn rejects_deleting_profile_when_bound_to_window() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = ProfileService::with_secret_store(pool.clone(), secret_store);
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

    let result = service.delete_profile(&created.id).await;
    assert!(matches!(result, Err(AppError::InvalidInput(_))));

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
    assert_eq!(created.provider, "openai");
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
    assert_eq!(created.provider, "anthropic");

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
    assert_eq!(updated.provider, "openai");

    Ok(())
}
