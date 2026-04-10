use tauri::State;

use crate::{
    app_state::AppState,
    models::evaluation_case::{
        CreateEvaluationCaseInput, EvaluationCaseResponse, UpdateEvaluationCaseInput,
    },
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

#[tauri::command]
pub async fn update_evaluation_case(
    state: State<'_, AppState>,
    id: String,
    input: UpdateEvaluationCaseInput,
) -> Result<EvaluationCaseResponse, String> {
    let service = EvaluationCaseService::new(state.pool.clone());
    let evaluation_case = service
        .update_evaluation_case(&id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(evaluation_case.into())
}

#[tauri::command]
pub async fn delete_evaluation_case(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let service = EvaluationCaseService::new(state.pool.clone());
    service
        .delete_evaluation_case(&id)
        .await
        .map_err(|error| error.to_string())
}
