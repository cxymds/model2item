mod support;

use std::sync::Arc;

use iterm_mcp_tools_lib::{
    models::profile::CreateProfileInput,
    services::{
        profile_service::ProfileService,
        secret_store::{profile_secret_locator, MemorySecretStore},
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
    assert!(foreign_keys_on, "expected SQLite foreign_keys pragma to be ON");

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
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: secret.to_string(),
        })
        .await?;

    assert_eq!(created.provider, "openai");
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
        secret_store.get_secret(&profile_secret_locator(&created.id)).as_deref(),
        Some(secret)
    );

    let listed = service.list_profiles().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let fetched = service.get_profile(&created.id).await?;
    assert_eq!(fetched.name, "GPT 5.4");

    Ok(())
}
