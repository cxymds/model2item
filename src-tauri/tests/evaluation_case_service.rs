mod support;

use iterm_mcp_tools_lib::error::AppError;
use iterm_mcp_tools_lib::{
    models::{
        comparison_run::CreateComparisonRunInput,
        evaluation_case::{CreateEvaluationCaseInput, UpdateEvaluationCaseInput},
        profile::CreateProfileInput,
        window_binding::CreateWindowBindingInput,
    },
    services::{
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService, profile_service::ProfileService,
        secret_store::MemorySecretStore, window_binding_service::WindowBindingService,
    },
};
use std::sync::Arc;

#[tokio::test]
async fn creates_and_lists_evaluation_cases() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service = EvaluationCaseService::new(pool);

    let created = service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Legacy Parser Review".to_string(),
            prompt: "Explain the parsing flow and risks.".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: Some("Focus on old parser behavior".to_string()),
        })
        .await?;

    assert_eq!(created.title, "Legacy Parser Review");
    assert_eq!(created.expected_checkpoints_json, "[]");
    assert_eq!(created.validation_rules_json, "{}");

    let listed = service.list_evaluation_cases().await?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let fetched = service.get_evaluation_case(&created.id).await?;
    assert_eq!(fetched.prompt, "Explain the parsing flow and risks.");

    Ok(())
}

#[tokio::test]
async fn rejects_evaluation_case_with_invalid_context_json(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service = EvaluationCaseService::new(pool);

    let result = service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Broken payload".to_string(),
            prompt: "Explain the parsing flow and risks.".to_string(),
            context_payload: "{\"files\":[".to_string(),
            notes: None,
        })
        .await;

    assert!(matches!(result, Err(AppError::InvalidJsonInput(_))));

    Ok(())
}

#[tokio::test]
async fn updates_evaluation_case_fields() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service = EvaluationCaseService::new(pool);

    let created = service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Legacy Parser Review".to_string(),
            prompt: "Explain the parsing flow and risks.".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: Some("Focus on old parser behavior".to_string()),
        })
        .await?;

    let updated = service
        .update_evaluation_case(
            &created.id,
            UpdateEvaluationCaseInput {
                title: "Updated Review".to_string(),
                prompt: "Summarize the code path and hidden risks.".to_string(),
                context_payload: "{\"files\":[\"parser.rs\",\"lexer.rs\"]}".to_string(),
                notes: Some("Focus on migration cost".to_string()),
            },
        )
        .await?;

    assert_eq!(updated.title, "Updated Review");
    assert!(updated.prompt.contains("hidden risks"));
    assert!(updated.context_payload.contains("lexer.rs"));
    assert_eq!(updated.notes, "Focus on migration cost");

    Ok(())
}

#[tokio::test]
async fn rejects_deleting_evaluation_case_when_it_is_referenced_by_run(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let profile_service =
        ProfileService::with_secret_store(pool.clone(), Arc::new(MemorySecretStore::default()));
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool);

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

    let evaluation_case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Legacy Parser Review".to_string(),
            prompt: "Explain the parsing flow and risks.".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
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
            evaluation_case_id: evaluation_case.id.clone(),
            title: "Parser benchmark".to_string(),
            target_ids: vec![binding.id],
            notes: None,
        })
        .await?;

    let result = case_service
        .delete_evaluation_case(&evaluation_case.id)
        .await;

    assert!(matches!(result, Err(AppError::InvalidInput(_))));

    Ok(())
}
