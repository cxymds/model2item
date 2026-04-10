use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ModelProfileRecord {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub system_prompt: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileResponse {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub system_prompt: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ModelProfileRecord> for ProfileResponse {
    fn from(value: ModelProfileRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            provider: value.provider,
            model_name: value.model_name,
            base_url: value.base_url,
            system_prompt: value.system_prompt,
            temperature: value.temperature,
            max_tokens: value.max_tokens,
            extra_params_json: value.extra_params_json,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key: String,
}
