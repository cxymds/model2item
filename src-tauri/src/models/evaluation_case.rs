use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct EvaluationCaseRecord {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub context_payload: String,
    pub expected_checkpoints_json: String,
    pub validation_rules_json: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvaluationCaseResponse {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub context_payload: String,
    pub expected_checkpoints_json: String,
    pub validation_rules_json: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<EvaluationCaseRecord> for EvaluationCaseResponse {
    fn from(value: EvaluationCaseRecord) -> Self {
        Self {
            id: value.id,
            title: value.title,
            prompt: value.prompt,
            context_payload: value.context_payload,
            expected_checkpoints_json: value.expected_checkpoints_json,
            validation_rules_json: value.validation_rules_json,
            notes: value.notes,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEvaluationCaseInput {
    pub title: String,
    pub prompt: String,
    pub context_payload: String,
    pub notes: Option<String>,
}
