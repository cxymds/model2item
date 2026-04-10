mod support;

use std::sync::Arc;

use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        comparison_run::CreateComparisonRunInput, evaluation_case::CreateEvaluationCaseInput,
        profile::CreateProfileInput, window_binding::CreateWindowBindingInput,
    },
    services::{
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService, profile_service::ProfileService,
        secret_store::MemorySecretStore, window_binding_service::WindowBindingService,
    },
};

#[tokio::test]
async fn creates_comparison_run_and_target_snapshots() -> Result<(), Box<dyn std::error::Error>> {
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
            title: "Legacy parser review".to_string(),
            prompt: "Explain the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: Some("Focus on control flow".to_string()),
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-1".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id.clone(),
            title: "Parser benchmark".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: Some("Run against current baseline".to_string()),
        })
        .await?;

    assert_eq!(run.evaluation_case_id, case.id);
    assert_eq!(run.status, "queued");

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].status, "queued");
    assert!(targets[0]
        .profile_snapshot_json
        .contains("\"provider\":\"openai\""));
    assert!(targets[0]
        .profile_snapshot_json
        .contains("\"model_name\":\"gpt-5.4\""));
    assert!(targets[0]
        .profile_snapshot_json
        .contains("\"base_url\":\"https://api.example.com/v1\""));

    let fetched = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(fetched.id, run.id);

    Ok(())
}

#[tokio::test]
async fn rejects_comparison_run_when_binding_does_not_exist(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let case_service = EvaluationCaseService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool);

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Legacy parser review".to_string(),
            prompt: "Explain the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: None,
        })
        .await?;

    let result = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Invalid binding run".to_string(),
            target_ids: vec!["missing-binding".to_string()],
            notes: None,
        })
        .await;

    assert!(matches!(result, Err(AppError::MissingDependency(_))));

    Ok(())
}

#[tokio::test]
async fn rejects_comparison_run_with_empty_target_ids() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let case_service = EvaluationCaseService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool);

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Legacy parser review".to_string(),
            prompt: "Explain the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: None,
        })
        .await?;

    let result = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "No targets".to_string(),
            target_ids: vec![],
            notes: None,
        })
        .await;

    assert!(matches!(result, Err(AppError::InvalidInput(_))));

    Ok(())
}

#[tokio::test]
async fn deduplicates_duplicate_target_ids_and_keeps_input_order(
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
            title: "Legacy parser review".to_string(),
            prompt: "Explain the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding_a = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-a".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let binding_b = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-b".to_string(),
            display_name: "Window B".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Duplicate targets".to_string(),
            target_ids: vec![
                binding_b.id.clone(),
                binding_a.id.clone(),
                binding_b.id.clone(),
                binding_a.id.clone(),
            ],
            notes: None,
        })
        .await?;

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].window_binding_id, binding_b.id);
    assert_eq!(targets[0].position, 0);
    assert_eq!(targets[1].window_binding_id, binding_a.id);
    assert_eq!(targets[1].position, 1);
    assert!(targets[0]
        .profile_snapshot_json
        .contains("\"display_name\":\"Window B\""));

    Ok(())
}

#[tokio::test]
async fn lists_recent_comparison_runs_in_descending_order_with_limit(
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
            title: "Legacy parser review".to_string(),
            prompt: "Explain the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-1".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let mut created_titles = Vec::new();
    for index in 0..22 {
        let title = format!("Run {index:02}");
        run_service
            .create_comparison_run(CreateComparisonRunInput {
                evaluation_case_id: case.id.clone(),
                title: title.clone(),
                target_ids: vec![binding.id.clone()],
                notes: None,
            })
            .await?;
        created_titles.push(title);
    }

    let listed_runs = run_service.list_comparison_runs(20).await?;

    assert_eq!(listed_runs.len(), 20);
    assert_eq!(listed_runs[0].title, "Run 21");
    assert_eq!(listed_runs[19].title, "Run 02");
    assert_eq!(listed_runs[0].status, "queued");

    Ok(())
}
