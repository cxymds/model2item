use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ItermSessionResponse {
    pub session_id: String,
    pub window_id: String,
    pub window_title: String,
    pub tab_id: String,
    pub tab_title: String,
    pub session_title: String,
}
