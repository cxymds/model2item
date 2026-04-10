use async_trait::async_trait;
use iterm_mcp_tools_lib::{
    error::AppError,
    services::{
        iterm_mcp_adapter::{
            ItermExecutionRequest, ItermExecutionResult, ItermMcpAdapter, ItermSessionInfo,
        },
        iterm_session_service::ItermSessionService,
    },
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

    async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), String> {
        Ok(())
    }

    async fn get_screen_text(&self, _session_id: &str) -> Result<String, String> {
        Ok(String::new())
    }

    async fn execute_prompt(
        &self,
        _request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        unreachable!("execute_prompt is not used in this test");
    }
}

#[derive(Clone)]
struct MissingDependencyAdapter;

#[async_trait]
impl ItermMcpAdapter for MissingDependencyAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Err("Missing Python package 'iterm2'. Install iTerm2 Python API support first.".to_string())
    }

    async fn send_text(&self, _session_id: &str, _text: &str) -> Result<(), String> {
        Err("Missing Python package 'iterm2'. Install iTerm2 Python API support first.".to_string())
    }

    async fn get_screen_text(&self, _session_id: &str) -> Result<String, String> {
        Err("Missing Python package 'iterm2'. Install iTerm2 Python API support first.".to_string())
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

#[tokio::test]
async fn maps_missing_iterm2_package_to_missing_dependency_error() {
    let service = ItermSessionService::with_adapter(MissingDependencyAdapter);

    let result = service.list_sessions().await;

    match result {
        Err(AppError::MissingDependency(message)) => {
            assert!(message.contains("iterm2"));
            assert!(message.contains("python3 -m pip install iterm2"));
        }
        other => panic!("expected missing dependency error, got {other:?}"),
    }
}
