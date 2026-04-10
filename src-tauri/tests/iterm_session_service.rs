use async_trait::async_trait;
use iterm_mcp_tools_lib::services::{
    iterm_mcp_adapter::{ItermMcpAdapter, ItermSessionInfo, ItermExecutionRequest, ItermExecutionResult},
    iterm_session_service::ItermSessionService,
};

#[derive(Clone)]
struct FakeAdapter;

#[async_trait]
impl ItermMcpAdapter for FakeAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(vec![ItermSessionInfo {
            session_id: "session-1".to_string(),
            window_id: "window-1".to_string(),
            window_title: "Project A".to_string(),
            tab_id: "tab-1".to_string(),
            tab_title: "Compare".to_string(),
            session_title: "Pane 1".to_string(),
        }])
    }

    async fn execute_prompt(
        &self,
        _request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        unreachable!("execute_prompt is not used in this test");
    }
}

#[tokio::test]
async fn lists_discovered_iterm_sessions() -> Result<(), Box<dyn std::error::Error>> {
    let service = ItermSessionService::with_adapter(FakeAdapter);
    let sessions = service.list_sessions().await?;

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "session-1");
    assert_eq!(sessions[0].window_title, "Project A");

    Ok(())
}
