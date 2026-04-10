use std::{path::PathBuf, process::Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItermSessionInfo {
    pub session_id: String,
    pub window_id: String,
    pub window_title: String,
    pub tab_id: String,
    pub tab_title: String,
    pub session_title: String,
}

#[derive(Debug, Clone)]
pub struct ItermExecutionRequest {
    pub session_id: String,
    pub prompt: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key: String,
    pub system_prompt: String,
    pub extra_params_json: String,
}

#[derive(Debug, Clone)]
pub struct ItermExecutionResult {
    pub output_text: String,
}

#[async_trait]
pub trait ItermMcpAdapter: Send + Sync + Clone + 'static {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String>;

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String>;
}

#[derive(Debug, Clone)]
pub struct PythonItermMcpAdapter {
    script_path: PathBuf,
    python_bin: String,
}

impl Default for PythonItermMcpAdapter {
    fn default() -> Self {
        Self {
            script_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("iterm_mcp_adapter.py"),
            python_bin: "python3".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScriptRequest<'a> {
    action: &'a str,
    session_id: &'a str,
    prompt: &'a str,
    provider: &'a str,
    model_name: &'a str,
    base_url: &'a str,
    api_key: &'a str,
    system_prompt: &'a str,
    extra_params_json: &'a str,
}

#[derive(Debug, Deserialize)]
struct ScriptResponse {
    output_text: Option<String>,
    sessions: Option<Vec<ItermSessionInfo>>,
}

#[async_trait]
impl ItermMcpAdapter for PythonItermMcpAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        let response = self.run_script(
            serde_json::to_vec(&serde_json::json!({
                "action": "list_sessions"
            }))
            .map_err(|error| error.to_string())?,
        )
        .await?;

        response
            .sessions
            .ok_or_else(|| "adapter response did not include sessions".to_string())
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        let response = self
            .run_script(
                serde_json::to_vec(&ScriptRequest {
            action: "execute_prompt",
            session_id: &request.session_id,
            prompt: &request.prompt,
            provider: &request.provider,
            model_name: &request.model_name,
            base_url: &request.base_url,
            api_key: &request.api_key,
            system_prompt: &request.system_prompt,
            extra_params_json: &request.extra_params_json,
        })
        .map_err(|error| error.to_string())?,
            )
            .await?;

        Ok(ItermExecutionResult {
            output_text: response
                .output_text
                .ok_or_else(|| "adapter response did not include output_text".to_string())?,
        })
    }
}

impl PythonItermMcpAdapter {
    async fn run_script(&self, payload: Vec<u8>) -> Result<ScriptResponse, String> {
        if !self.script_path.exists() {
            return Err(format!(
                "adapter script not found at {}",
                self.script_path.display()
            ));
        }

        let mut child = Command::new(&self.python_bin)
            .arg(&self.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(&payload)
                .await
                .map_err(|error| error.to_string())?;
        }

        let output = child.wait_with_output().await.map_err(|error| error.to_string())?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("adapter exited with status {}", output.status)
            } else {
                stderr
            });
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
        serde_json::from_str(&stdout).map_err(|error| error.to_string())
    }
}
