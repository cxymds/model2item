use tauri::State;

use crate::{
    app_state::AppState,
    models::iterm_session::ItermSessionResponse,
    models::window_binding::{CreateWindowBindingInput, WindowBindingResponse},
    services::{
        iterm_session_service::ItermSessionService, window_binding_service::WindowBindingService,
    },
};

#[tauri::command]
pub async fn create_window_binding(
    state: State<'_, AppState>,
    input: CreateWindowBindingInput,
) -> Result<WindowBindingResponse, String> {
    let service = WindowBindingService::new(state.pool.clone());
    let binding = service
        .create_window_binding(input)
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
