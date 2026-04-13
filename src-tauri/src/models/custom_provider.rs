use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CustomProviderRecord {
    pub id: String,
    pub name: String,
    pub provider_key: String,
    pub client_type: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub default_model: String,
    pub enabled: i64,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomProviderResponse {
    pub id: String,
    pub name: String,
    pub provider_key: String,
    pub client_type: String,
    pub base_url: String,
    pub default_model: String,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CustomProviderRecord> for CustomProviderResponse {
    fn from(value: CustomProviderRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider_key: value.provider_key,
            client_type: value.client_type,
            base_url: value.base_url,
            default_model: value.default_model,
            extra_params_json: value.extra_params_json,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCustomProviderInput {
    pub name: String,
    pub provider_key: String,
    pub client_type: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub extra_params_json: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCustomProviderInput {
    pub name: String,
    pub provider_key: String,
    pub client_type: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub extra_params_json: String,
}
