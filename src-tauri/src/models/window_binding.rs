use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct WindowBindingRecord {
    pub id: String,
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub custom_provider_id: Option<String>,
    pub enabled: i64,
    pub last_seen_at: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowBindingResponse {
    pub id: String,
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub custom_provider_id: Option<String>,
    pub enabled: i64,
    pub last_seen_at: Option<String>,
    pub metadata_json: String,
}

impl From<WindowBindingRecord> for WindowBindingResponse {
    fn from(value: WindowBindingRecord) -> Self {
        Self {
            id: value.id,
            iterm_session_id: value.iterm_session_id,
            display_name: value.display_name,
            profile_id: value.profile_id,
            custom_provider_id: value.custom_provider_id,
            enabled: value.enabled,
            last_seen_at: value.last_seen_at,
            metadata_json: value.metadata_json,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWindowBindingInput {
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub custom_provider_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWindowBindingInput {
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub custom_provider_id: Option<String>,
}
