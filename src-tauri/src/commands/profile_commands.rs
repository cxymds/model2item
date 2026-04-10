use tauri::State;

use crate::{
    app_state::AppState,
    models::profile::{CreateProfileInput, ProfileResponse},
    services::profile_service::ProfileService,
};

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    input: CreateProfileInput,
) -> Result<ProfileResponse, String> {
    let service = ProfileService::new(state.pool.clone());
    let profile = service
        .create_profile(input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(profile.into())
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileResponse>, String> {
    let service = ProfileService::new(state.pool.clone());
    let profiles = service
        .list_profiles()
        .await
        .map_err(|error| error.to_string())?;
    Ok(profiles.into_iter().map(Into::into).collect())
}
