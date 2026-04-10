use crate::{
    error::AppError,
    services::iterm_mcp_adapter::{
        classify_adapter_error, ItermMcpAdapter, ItermSessionInfo, PythonItermMcpAdapter,
    },
};

#[derive(Clone)]
pub struct ItermSessionService<A: ItermMcpAdapter> {
    adapter: A,
}

impl ItermSessionService<PythonItermMcpAdapter> {
    pub fn new() -> Self {
        Self {
            adapter: PythonItermMcpAdapter::default(),
        }
    }
}

impl<A: ItermMcpAdapter> ItermSessionService<A> {
    pub fn with_adapter(adapter: A) -> Self {
        Self { adapter }
    }

    pub async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, AppError> {
        self.adapter
            .list_sessions()
            .await
            .map_err(classify_adapter_error)
    }
}
