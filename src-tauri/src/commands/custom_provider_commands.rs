use tauri::State;

use crate::{
    app_state::AppState,
    models::custom_provider::{
        CreateCustomProviderInput, CustomProviderResponse, UpdateCustomProviderInput,
    },
    services::custom_provider_service::CustomProviderService,
};

#[tauri::command]
pub async fn create_custom_provider(
    state: State<'_, AppState>,
    input: CreateCustomProviderInput,
) -> Result<CustomProviderResponse, String> {
    let service = CustomProviderService::new(state.pool.clone());
    service
        .create_custom_provider(input)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_custom_providers(
    state: State<'_, AppState>,
) -> Result<Vec<CustomProviderResponse>, String> {
    let service = CustomProviderService::new(state.pool.clone());
    service
        .list_custom_providers()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_custom_provider(
    state: State<'_, AppState>,
    id: String,
    input: UpdateCustomProviderInput,
) -> Result<CustomProviderResponse, String> {
    let service = CustomProviderService::new(state.pool.clone());
    service
        .update_custom_provider(&id, input)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_custom_provider(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let service = CustomProviderService::new(state.pool.clone());
    service
        .delete_custom_provider(&id)
        .await
        .map_err(|error| error.to_string())
}
