use tauri::State;

use crate::{
    app_state::AppState,
    models::comparison_run::{
        ComparisonRunResponse, ComparisonSummaryResponse, ComparisonTargetResponse,
        CreateComparisonRunInput,
    },
    services::{
        analysis_service::AnalysisService, comparison_orchestrator::ComparisonOrchestrator,
        comparison_run_service::ComparisonRunService,
    },
};

#[tauri::command]
pub async fn create_comparison_run(
    state: State<'_, AppState>,
    input: CreateComparisonRunInput,
) -> Result<ComparisonRunResponse, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let run = service
        .create_comparison_run(input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(run.into())
}

#[tauri::command]
pub async fn get_comparison_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<ComparisonRunResponse, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let run = service
        .get_comparison_run(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(run.into())
}

#[tauri::command]
pub async fn list_comparison_runs(
    state: State<'_, AppState>,
) -> Result<Vec<ComparisonRunResponse>, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let runs = service
        .list_comparison_runs(20)
        .await
        .map_err(|error| error.to_string())?;
    Ok(runs.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn list_comparison_targets(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<ComparisonTargetResponse>, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let targets = service
        .list_comparison_targets(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(targets.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn get_comparison_summary(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<ComparisonSummaryResponse, String> {
    let service = AnalysisService::new(state.pool.clone());
    service
        .get_comparison_summary(&run_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_comparison_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let pool = state.pool.clone();
    let run_service = ComparisonRunService::new(pool.clone());
    run_service
        .get_comparison_run(&run_id)
        .await
        .map_err(|error| error.to_string())?;

    tauri::async_runtime::spawn(async move {
        let orchestrator = ComparisonOrchestrator::new(pool);
        if let Err(error) = orchestrator.execute_run(&run_id).await {
            eprintln!("failed to execute comparison run {}: {}", run_id, error);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn send_comparison_run_message(
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let orchestrator = ComparisonOrchestrator::new(state.pool.clone());
    orchestrator
        .broadcast_message(&run_id, &prompt)
        .await
        .map_err(|error| error.to_string())
}
