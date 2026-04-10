mod support;

use iterm_mcp_tools_lib::{
    error::AppError,
    models::{profile::CreateProfileInput, window_binding::CreateWindowBindingInput},
    services::{
        profile_service::ProfileService, window_binding_service::WindowBindingService,
    },
};

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
async fn rejects_window_binding_when_profile_does_not_exist() -> Result<(), Box<dyn std::error::Error>>
{
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
