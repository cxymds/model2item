use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ComparisonRunRecord {
    pub id: String,
    pub evaluation_case_id: String,
    pub title: String,
    pub status: String,
    pub prompt_snapshot: String,
    pub context_snapshot: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ComparisonTargetRecord {
    pub position: i64,
    pub id: String,
    pub run_id: String,
    pub window_binding_id: String,
    pub profile_snapshot_json: String,
    pub status: String,
    pub sent_at: Option<String>,
    pub first_response_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub response_chars: i64,
    pub response_lines: i64,
    pub success_status: Option<String>,
    pub error_category: Option<String>,
    pub error_detail: Option<String>,
    pub latest_message_role: Option<String>,
    pub latest_message_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRunResponse {
    pub id: String,
    pub evaluation_case_id: String,
    pub title: String,
    pub status: String,
    pub prompt_snapshot: String,
    pub context_snapshot: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub notes: String,
}

impl From<ComparisonRunRecord> for ComparisonRunResponse {
    fn from(value: ComparisonRunRecord) -> Self {
        Self {
            id: value.id,
            evaluation_case_id: value.evaluation_case_id,
            title: value.title,
            status: value.status,
            prompt_snapshot: value.prompt_snapshot,
            context_snapshot: value.context_snapshot,
            created_at: value.created_at,
            started_at: value.started_at,
            finished_at: value.finished_at,
            notes: value.notes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonTargetResponse {
    pub position: i64,
    pub id: String,
    pub run_id: String,
    pub window_binding_id: String,
    pub profile_snapshot_json: String,
    pub status: String,
    pub sent_at: Option<String>,
    pub first_response_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub response_chars: i64,
    pub response_lines: i64,
    pub success_status: Option<String>,
    pub error_category: Option<String>,
    pub error_detail: Option<String>,
    pub latest_message_role: Option<String>,
    pub latest_message_content: Option<String>,
}

impl From<ComparisonTargetRecord> for ComparisonTargetResponse {
    fn from(value: ComparisonTargetRecord) -> Self {
        Self {
            position: value.position,
            id: value.id,
            run_id: value.run_id,
            window_binding_id: value.window_binding_id,
            profile_snapshot_json: value.profile_snapshot_json,
            status: value.status,
            sent_at: value.sent_at,
            first_response_at: value.first_response_at,
            finished_at: value.finished_at,
            duration_ms: value.duration_ms,
            response_chars: value.response_chars,
            response_lines: value.response_lines,
            success_status: value.success_status,
            error_category: value.error_category,
            error_detail: value.error_detail,
            latest_message_role: value.latest_message_role,
            latest_message_content: value.latest_message_content,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateComparisonRunInput {
    pub evaluation_case_id: String,
    pub title: String,
    pub target_ids: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonSummaryTargetResponse {
    pub target_id: String,
    pub label: String,
    pub display_name: Option<String>,
    pub provider: Option<String>,
    pub model_name: Option<String>,
    pub status: String,
    pub success_status: Option<String>,
    pub duration_ms: Option<i64>,
    pub response_chars: i64,
    pub response_lines: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonSummaryResponse {
    pub run: ComparisonRunResponse,
    pub targets: Vec<ComparisonSummaryTargetResponse>,
    pub fastest_target_id: Option<String>,
    pub longest_target_id: Option<String>,
    pub queued_count: usize,
    pub summary_text: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ComparisonMessageRecord {
    pub id: String,
    pub comparison_target_id: String,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonMessageResponse {
    pub id: String,
    pub comparison_target_id: String,
    pub role: String,
    pub content: String,
    pub message_type: String,
    pub created_at: String,
}

impl From<ComparisonMessageRecord> for ComparisonMessageResponse {
    fn from(value: ComparisonMessageRecord) -> Self {
        Self {
            id: value.id,
            comparison_target_id: value.comparison_target_id,
            role: value.role,
            content: value.content,
            message_type: value.message_type,
            created_at: value.created_at,
        }
    }
}
