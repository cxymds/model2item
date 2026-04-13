use tauri::State;

use crate::{
    app_state::AppState,
    models::iterm_session::ItermSessionResponse,
    models::window_binding::{
        CreateWindowBindingInput, UpdateWindowBindingInput, WindowBindingResponse,
    },
    services::{
        iterm_mcp_adapter::PythonItermMcpAdapter,
        iterm_session_service::ItermSessionService,
        secret_store::SystemSecretStore,
        window_binding_service::WindowBindingService,
        window_binding_sync_service::{
            create_window_binding_and_sync, update_window_binding_and_sync,
        },
    },
};

#[tauri::command]
pub async fn create_window_binding(
    state: State<'_, AppState>,
    input: CreateWindowBindingInput,
) -> Result<WindowBindingResponse, String> {
    let binding = create_window_binding_and_sync(
        state.pool.clone(),
        PythonItermMcpAdapter::default(),
        std::sync::Arc::new(SystemSecretStore),
        input,
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(binding.into())
}

#[tauri::command]
pub async fn list_window_bindings(
    state: State<'_, AppState>,
) -> Result<Vec<WindowBindingResponse>, String> {
    let service = WindowBindingService::new(state.pool.clone());
    let bindings = service
        .list_window_bindings()
        .await
        .map_err(|error| error.to_string())?;
    Ok(bindings.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn update_window_binding(
    state: State<'_, AppState>,
    id: String,
    input: UpdateWindowBindingInput,
) -> Result<WindowBindingResponse, String> {
    let binding = update_window_binding_and_sync(
        state.pool.clone(),
        PythonItermMcpAdapter::default(),
        std::sync::Arc::new(SystemSecretStore),
        &id,
        input,
    )
    .await
        .map_err(|error| error.to_string())?;
    Ok(binding.into())
}

#[tauri::command]
pub async fn delete_window_binding(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let service = WindowBindingService::new(state.pool.clone());
    service
        .delete_window_binding(&id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_iterm_sessions() -> Result<Vec<ItermSessionResponse>, String> {
    let service = ItermSessionService::new();
    let sessions = service
        .list_sessions()
        .await
        .map_err(|error| error.to_string())?;

    Ok(sessions
        .into_iter()
        .map(|session| ItermSessionResponse {
            session_id: session.session_id,
            window_id: session.window_id,
            window_title: session.window_title,
            tab_id: session.tab_id,
            tab_title: session.tab_title,
            session_title: session.session_title,
        })
        .collect())
}

#[tauri::command]
pub async fn refresh_window_binding_presence(
    state: State<'_, AppState>,
) -> Result<Vec<WindowBindingResponse>, String> {
    let session_service = ItermSessionService::new();
    let sessions = session_service
        .list_sessions()
        .await
        .map_err(|error| error.to_string())?;

    let binding_service = WindowBindingService::new(state.pool.clone());
    let bindings = binding_service
        .sync_with_online_sessions(
            &sessions
                .iter()
                .map(|session| session.session_id.clone())
                .collect::<Vec<_>>(),
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(bindings.into_iter().map(Into::into).collect())
}
