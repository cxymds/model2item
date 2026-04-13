mod support;

use iterm_mcp_tools_lib::models::custom_provider::CustomProviderRecord;
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, SqlitePool};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use iterm_mcp_tools_lib::{
    error::AppError,
    models::custom_provider::{CreateCustomProviderInput, UpdateCustomProviderInput},
    services::{
        custom_provider_service::CustomProviderService,
        secret_store::{MemorySecretStore, SecretStore},
    },
};

async fn create_legacy_pool() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(&pool)
        .await?;
    Ok(pool)
}

async fn create_legacy_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE model_profiles (
          id TEXT PRIMARY KEY NOT NULL,
          name TEXT NOT NULL,
          provider TEXT NOT NULL,
          model_name TEXT NOT NULL,
          base_url TEXT NOT NULL,
          api_key_encrypted TEXT NOT NULL,
          system_prompt TEXT NOT NULL DEFAULT '',
          temperature REAL,
          max_tokens INTEGER,
          extra_params_json TEXT NOT NULL DEFAULT '{}',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          execution_mode TEXT NOT NULL DEFAULT 'claude_cli',
          enabled INTEGER NOT NULL DEFAULT 1
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE window_bindings (
          id TEXT PRIMARY KEY NOT NULL,
          iterm_session_id TEXT NOT NULL,
          display_name TEXT NOT NULL,
          profile_id TEXT NOT NULL REFERENCES model_profiles(id),
          enabled INTEGER NOT NULL DEFAULT 1,
          last_seen_at TEXT,
          metadata_json TEXT NOT NULL DEFAULT '{}'
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn run_task1_migration(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let migration_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("db")
        .join("migrations")
        .join("0004_add_custom_providers.sql");
    let migration_dir = unique_temp_migration_dir()?;
    fs::create_dir_all(&migration_dir)?;
    fs::copy(
        &migration_source,
        migration_dir.join("0004_add_custom_providers.sql"),
    )?;

    let migrator = Migrator::new(migration_dir.clone()).await?;
    migrator.run(pool).await?;

    let _ = fs::remove_dir_all(&migration_dir);
    Ok(())
}

fn unique_temp_migration_dir() -> Result<PathBuf, std::time::SystemTimeError> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "iterm-mcp-task1-migration-{stamp}"
    )))
}

#[tokio::test]
async fn lists_backfilled_custom_providers_from_existing_profiles(
) -> Result<(), Box<dyn std::error::Error>> {
    let _typecheck: Option<CustomProviderRecord> = None;
    let pool = create_legacy_pool().await?;
    create_legacy_schema(&pool).await?;

    sqlx::query(
        r#"
        INSERT INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, system_prompt, temperature, max_tokens, extra_params_json, created_at, updated_at)
        VALUES
          ('profile-1', 'GLM via Claude CLI', 'glm', 'claude_cli', 'glm-5.1', 'https://glm.example.com/v1', 'secret://glm', '', NULL, NULL, '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z')
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id)
        VALUES ('binding-1', 'session-1', 'Window 1', 'profile-1')
        "#,
    )
    .execute(&pool)
    .await?;

    run_task1_migration(&pool).await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);

    let provider = sqlx::query_as::<_, CustomProviderRecord>(
        "SELECT * FROM custom_providers WHERE id = 'provider-profile-1' LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(provider.name, "GLM via Claude CLI");
    assert_eq!(provider.provider_key, "glm");
    assert_eq!(provider.client_type, "claude_cli");
    assert_eq!(provider.default_model, "glm-5.1");
    assert_eq!(provider.enabled, 1);

    let bound_provider_id =
        sqlx::query_scalar::<_, Option<String>>("SELECT custom_provider_id FROM window_bindings WHERE id = 'binding-1' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    assert_eq!(bound_provider_id.as_deref(), Some("provider-profile-1"));

    let invalid_set_result = sqlx::query(
        "UPDATE window_bindings SET custom_provider_id = 'provider-does-not-exist' WHERE id = 'binding-1'",
    )
    .execute(&pool)
    .await;
    assert!(
        invalid_set_result.is_err(),
        "expected foreign key constraint on window_bindings.custom_provider_id"
    );

    Ok(())
}

#[tokio::test]
async fn backfills_disabled_profile_state_into_custom_providers(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_legacy_pool().await?;
    create_legacy_schema(&pool).await?;

    sqlx::query(
        r#"
        INSERT INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, system_prompt, temperature, max_tokens, extra_params_json, created_at, updated_at, enabled)
        VALUES
          ('profile-disabled', 'Disabled Provider', 'glm', 'claude_cli', 'glm-5.1', 'https://glm.example.com/v1', 'secret://glm-disabled', '', NULL, NULL, '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z', 0)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id)
        VALUES ('binding-disabled', 'session-disabled', 'Window Disabled', 'profile-disabled')
        "#,
    )
    .execute(&pool)
    .await?;

    run_task1_migration(&pool).await?;

    let backfilled_enabled = sqlx::query_scalar::<_, i64>(
        "SELECT enabled FROM custom_providers WHERE id = 'provider-profile-disabled' LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(backfilled_enabled, 0);

    let bound_provider_id = sqlx::query_scalar::<_, Option<String>>(
        "SELECT custom_provider_id FROM window_bindings WHERE id = 'binding-disabled' LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        bound_provider_id.as_deref(),
        Some("provider-profile-disabled")
    );

    Ok(())
}

#[tokio::test]
async fn creates_and_lists_custom_providers() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store);

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "GLM via Claude CLI".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "glm-secret".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    assert_eq!(created.name, "GLM via Claude CLI");
    assert_eq!(created.provider_key, "glm");
    assert_eq!(created.client_type, "claude_cli");
    assert_eq!(created.base_url, "https://glm.example.com/v1");
    assert_eq!(created.default_model, "glm-5.1");
    assert_eq!(created.extra_params_json, "{}");

    let listed = service.list_custom_providers().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    Ok(())
}

#[tokio::test]
async fn updates_custom_provider_fields_and_secret() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool, secret_store);

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "Provider A".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    let updated = service
        .update_custom_provider(
            &created.id,
            UpdateCustomProviderInput {
                name: "Provider B".to_string(),
                provider_key: "openai".to_string(),
                client_type: "openai_chat".to_string(),
                base_url: "https://api.openai.example.com/v1".to_string(),
                api_key: "secret-2".to_string(),
                default_model: "gpt-5.4".to_string(),
                extra_params_json: r#"{"tier":"prod"}"#.to_string(),
            },
        )
        .await?;

    assert_eq!(updated.name, "Provider B");
    assert_eq!(updated.provider_key, "openai");
    assert_eq!(updated.client_type, "openai_chat");
    assert_eq!(updated.base_url, "https://api.openai.example.com/v1");
    assert_eq!(updated.default_model, "gpt-5.4");
    assert_eq!(updated.extra_params_json, r#"{"tier":"prod"}"#);

    let secret = service.get_custom_provider_api_key(&created.id).await?;
    assert_eq!(secret.as_deref(), Some("secret-2"));

    Ok(())
}

#[tokio::test]
async fn keeps_existing_secret_when_updating_without_new_api_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool, secret_store);

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "Provider A".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    let _ = service
        .update_custom_provider(
            &created.id,
            UpdateCustomProviderInput {
                name: "Provider A2".to_string(),
                provider_key: "glm".to_string(),
                client_type: "claude_cli".to_string(),
                base_url: "https://glm.example.com/v2".to_string(),
                api_key: "".to_string(),
                default_model: "glm-5.1-plus".to_string(),
                extra_params_json: "{}".to_string(),
            },
        )
        .await?;

    let secret = service.get_custom_provider_api_key(&created.id).await?;
    assert_eq!(secret.as_deref(), Some("secret-1"));

    Ok(())
}

#[tokio::test]
async fn returns_none_when_custom_provider_secret_is_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "Provider A".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    let locator = sqlx::query_scalar::<_, String>(
        "SELECT api_key_encrypted FROM custom_providers WHERE id = ? LIMIT 1",
    )
    .bind(&created.id)
    .fetch_one(&pool)
    .await?;
    secret_store.delete_secret(&locator)?;

    let secret = service.get_custom_provider_api_key(&created.id).await?;
    assert_eq!(secret, None);

    Ok(())
}

#[tokio::test]
async fn deletes_custom_provider_and_removes_secret() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "Provider A".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "secret-1".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    let locator = sqlx::query_scalar::<_, String>(
        "SELECT api_key_encrypted FROM custom_providers WHERE id = ? LIMIT 1",
    )
    .bind(&created.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(secret_store.get_secret(&locator).as_deref(), Some("secret-1"));

    service.delete_custom_provider(&created.id).await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers WHERE id = ?")
        .bind(&created.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    assert_eq!(secret_store.get_secret(&locator), None);

    Ok(())
}

#[tokio::test]
async fn preserves_backfilled_locator_when_updating_without_new_api_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());
    let provider_id = "provider-legacy";
    let legacy_locator = "secret://profile/legacy-profile";

    secret_store.set_secret(legacy_locator, "legacy-secret")?;
    sqlx::query(
        r#"
        INSERT INTO custom_providers
          (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
        "#,
    )
    .bind(provider_id)
    .bind("Legacy Provider")
    .bind("glm")
    .bind("claude_cli")
    .bind("https://legacy.example.com/v1")
    .bind(legacy_locator)
    .bind("glm-5.1")
    .bind("{}")
    .bind("2026-04-13T00:00:00Z")
    .bind("2026-04-13T00:00:00Z")
    .execute(&pool)
    .await?;

    let _ = service
        .update_custom_provider(
            provider_id,
            UpdateCustomProviderInput {
                name: "Legacy Provider Updated".to_string(),
                provider_key: "glm".to_string(),
                client_type: "claude_cli".to_string(),
                base_url: "https://legacy.example.com/v2".to_string(),
                api_key: "".to_string(),
                default_model: "glm-5.2".to_string(),
                extra_params_json: "{}".to_string(),
            },
        )
        .await?;

    let stored_locator = sqlx::query_scalar::<_, String>(
        "SELECT api_key_encrypted FROM custom_providers WHERE id = ? LIMIT 1",
    )
    .bind(provider_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored_locator, legacy_locator);
    assert_eq!(
        secret_store.get_secret(legacy_locator).as_deref(),
        Some("legacy-secret")
    );
    assert_eq!(
        secret_store.get_secret("secret://custom-provider/provider-legacy"),
        None
    );

    Ok(())
}

#[tokio::test]
async fn deletes_backfilled_provider_using_stored_legacy_locator(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());
    let provider_id = "provider-legacy-delete";
    let legacy_locator = "secret://profile/legacy-profile-delete";

    secret_store.set_secret(legacy_locator, "legacy-secret-delete")?;
    sqlx::query(
        r#"
        INSERT INTO custom_providers
          (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
        "#,
    )
    .bind(provider_id)
    .bind("Legacy Provider Delete")
    .bind("glm")
    .bind("claude_cli")
    .bind("https://legacy.example.com/v1")
    .bind(legacy_locator)
    .bind("glm-5.1")
    .bind("{}")
    .bind("2026-04-13T00:00:00Z")
    .bind("2026-04-13T00:00:00Z")
    .execute(&pool)
    .await?;

    service.delete_custom_provider(provider_id).await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers WHERE id = ?")
        .bind(provider_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    assert_eq!(secret_store.get_secret(legacy_locator), None);

    Ok(())
}

#[tokio::test]
async fn deletes_provider_with_historical_bindings_by_disabling_bindings_and_clearing_reference(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "Provider With History".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "secret-history".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    let placeholder_profile_id = "__provider_binding_profile__";
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(placeholder_profile_id)
    .bind("Provider Binding Placeholder")
    .bind("openai-compatible")
    .bind("claude_cli")
    .bind("provider-binding-placeholder")
    .bind("")
    .bind("secret://profile/__provider_binding_profile__")
    .bind("2026-04-13T00:00:00Z")
    .bind("2026-04-13T00:00:00Z")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO evaluation_cases
          (id, title, prompt, context_payload, expected_checkpoints_json, validation_rules_json, notes, created_at, updated_at)
        VALUES
          ('case-1', 'Case 1', 'Prompt', '{}', '[]', '{}', '', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z')
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO comparison_runs
          (id, evaluation_case_id, title, status, prompt_snapshot, context_snapshot, created_at, started_at, finished_at, notes)
        VALUES
          ('run-1', 'case-1', 'Run 1', 'completed', 'Prompt', '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z', '2026-04-13T00:01:00Z', '')
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO window_bindings
          (id, iterm_session_id, display_name, profile_id, custom_provider_id, enabled, metadata_json)
        VALUES
          ('binding-history', 'session-1', 'Window History', ?, ?, 1, '{}')
        "#,
    )
    .bind(placeholder_profile_id)
    .bind(&created.id)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO comparison_targets
          (id, run_id, window_binding_id, profile_snapshot_json, status, response_chars, response_lines)
        VALUES
          ('target-1', 'run-1', 'binding-history', '{}', 'completed', 10, 2)
        "#,
    )
    .execute(&pool)
    .await?;

    service.delete_custom_provider(&created.id).await?;

    let provider_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers WHERE id = ?")
            .bind(&created.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(provider_count, 0);

    let binding_state = sqlx::query_as::<_, (i64, Option<String>, String)>(
        "SELECT enabled, custom_provider_id, metadata_json FROM window_bindings WHERE id = 'binding-history' LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(binding_state.0, 0);
    assert_eq!(binding_state.1, None);
    assert_eq!(binding_state.2, r#"{"deleted":true}"#);

    Ok(())
}

#[tokio::test]
async fn deletes_backfilled_provider_and_matching_legacy_profile(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool.clone(), secret_store.clone());
    let legacy_profile_id = "legacy-profile-1";
    let provider_id = format!("provider-{legacy_profile_id}");
    let legacy_locator = format!("secret://profile/{legacy_profile_id}");

    secret_store.set_secret(&legacy_locator, "legacy-secret-delete")?;
    sqlx::query(
        r#"
        INSERT INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(legacy_profile_id)
    .bind("Legacy Profile")
    .bind("glm")
    .bind("claude_cli")
    .bind("glm-5.1")
    .bind("https://legacy.example.com/v1")
    .bind(&legacy_locator)
    .bind("2026-04-13T00:00:00Z")
    .bind("2026-04-13T00:00:00Z")
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO custom_providers
          (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at)
        VALUES
          (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
        "#,
    )
    .bind(&provider_id)
    .bind("Legacy Provider")
    .bind("glm")
    .bind("claude_cli")
    .bind("https://legacy.example.com/v1")
    .bind(&legacy_locator)
    .bind("glm-5.1")
    .bind("{}")
    .bind("2026-04-13T00:00:00Z")
    .bind("2026-04-13T00:00:00Z")
    .execute(&pool)
    .await?;

    service.delete_custom_provider(&provider_id).await?;

    let provider_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers WHERE id = ?")
            .bind(&provider_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(provider_count, 0);

    let profile_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM model_profiles WHERE id = ?")
            .bind(legacy_profile_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(profile_count, 0);

    Ok(())
}

#[tokio::test]
async fn does_not_store_secret_when_updating_missing_provider(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool, secret_store.clone());
    let missing_id = "provider-missing";

    let result = service
        .update_custom_provider(
            missing_id,
            UpdateCustomProviderInput {
                name: "Missing".to_string(),
                provider_key: "glm".to_string(),
                client_type: "claude_cli".to_string(),
                base_url: "https://missing.example.com/v1".to_string(),
                api_key: "should-not-store".to_string(),
                default_model: "glm-5.1".to_string(),
                extra_params_json: "{}".to_string(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::Database(_))));
    assert_eq!(
        secret_store.get_secret("secret://custom-provider/provider-missing"),
        None
    );

    Ok(())
}
