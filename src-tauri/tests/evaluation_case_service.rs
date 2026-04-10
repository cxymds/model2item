mod support;

use iterm_mcp_tools_lib::error::AppError;
use iterm_mcp_tools_lib::{
    models::evaluation_case::CreateEvaluationCaseInput,
    services::evaluation_case_service::EvaluationCaseService,
};

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
async fn rejects_evaluation_case_with_invalid_context_json()
-> Result<(), Box<dyn std::error::Error>> {
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
