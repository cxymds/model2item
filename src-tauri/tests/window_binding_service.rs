mod support;

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
        evaluation_case_service::EvaluationCaseService, profile_service::ProfileService,
        secret_store::MemorySecretStore, window_binding_service::WindowBindingService,
    },
};
use std::sync::Arc;

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
    let offline = updated
        .iter()
        .find(|binding| binding.id == offline_binding.id)
        .expect("offline binding should exist");

    assert!(online.last_seen_at.is_some());
    assert_eq!(offline.last_seen_at, None);

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
