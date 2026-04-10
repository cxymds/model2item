use tauri::{Manager, State};

use crate::{
    app_state::AppState,
    models::comparison_run::{
        ComparisonMessageResponse, ComparisonRunResponse, ComparisonSummaryResponse,
        ComparisonTargetResponse, CreateComparisonRunInput,
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
pub async fn list_target_messages(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<Vec<ComparisonMessageResponse>, String> {
    let service = ComparisonRunService::new(state.pool.clone());
    let messages = service
        .list_target_messages(&target_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(messages.into_iter().map(Into::into).collect())
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
pub async fn export_comparison_run_report(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<String, String> {
    let service = AnalysisService::new(state.pool.clone());
    let export_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("exports");
    let path = service
        .export_comparison_run_report(&run_id, &export_dir)
        .await
        .map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn start_comparison_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let pool = state.pool.clone();
    let run_service = ComparisonRunService::new(pool.clone());
    let run = run_service
        .get_comparison_run(&run_id)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(active_run) = run_service
        .get_active_comparison_run()
        .await
        .map_err(|error| error.to_string())?
    {
        if active_run.id != run_id {
            return Err(format!(
                "invalid input: an active comparison run already exists: {}",
                active_run.title
            ));
        }
    }
    if run.status == "running" {
        return Err(format!(
            "invalid input: comparison run {} is already running",
            run.title
        ));
    }

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
