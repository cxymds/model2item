use tauri::State;

use crate::{
    app_state::AppState,
    models::evaluation_case::{CreateEvaluationCaseInput, EvaluationCaseResponse},
    services::evaluation_case_service::EvaluationCaseService,
};

#[tauri::command]
pub async fn create_evaluation_case(
    state: State<'_, AppState>,
    input: CreateEvaluationCaseInput,
) -> Result<EvaluationCaseResponse, String> {
    let service = EvaluationCaseService::new(state.pool.clone());
    let evaluation_case = service
        .create_evaluation_case(input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(evaluation_case.into())
}

#[tauri::command]
pub async fn list_evaluation_cases(
    state: State<'_, AppState>,
) -> Result<Vec<EvaluationCaseResponse>, String> {
    let service = EvaluationCaseService::new(state.pool.clone());
    let cases = service
        .list_evaluation_cases()
        .await
        .map_err(|error| error.to_string())?;
    Ok(cases.into_iter().map(Into::into).collect())
}
