use std::{collections::BTreeMap, path::PathBuf, process::Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::error::AppError;

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
    pub request_id: String,
    pub session_id: String,
    pub prompt: String,
    pub execution_mode: String,
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

pub fn classify_adapter_error(error: String) -> AppError {
    if error.contains("Missing Python package 'iterm2'") {
        AppError::MissingDependency(
            "缺少 Python 包 iterm2。请先安装 iTerm2 Python API 支持：python3 -m pip install iterm2"
                .to_string(),
        )
    } else {
        AppError::Adapter(error)
    }
}

#[async_trait]
pub trait ItermMcpAdapter: Send + Sync + Clone + 'static {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String>;
    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String>;
    async fn get_screen_text(&self, session_id: &str) -> Result<String, String>;

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
    command_text: &'a str,
    output_path: &'a str,
    error_path: &'a str,
    status_path: &'a str,
}

#[derive(Debug, Deserialize)]
struct ScriptResponse {
    output_text: Option<String>,
    sessions: Option<Vec<ItermSessionInfo>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ClaudeCliExtras {
    #[serde(alias = "claudeExecutable")]
    claude_executable: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<String>,
}

#[derive(Debug)]
struct BuiltCommand {
    command_text: String,
    output_path: String,
    error_path: String,
    status_path: String,
}

#[async_trait]
impl ItermMcpAdapter for PythonItermMcpAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        let response = self
            .run_script(
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

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.run_script(
            serde_json::to_vec(&serde_json::json!({
                "action": "send_text",
                "session_id": session_id,
                "text": text,
            }))
            .map_err(|error| error.to_string())?,
        )
        .await
        .map(|_| ())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let response = self
            .run_script(
                serde_json::to_vec(&serde_json::json!({
                    "action": "get_screen_text",
                    "session_id": session_id,
                }))
                .map_err(|error| error.to_string())?,
            )
            .await?;

        response
            .output_text
            .ok_or_else(|| "adapter response did not include output_text".to_string())
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        let built_command = build_claude_cli_command(&request)?;
        self.send_text(&request.session_id, &built_command.command_text)
            .await?;
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let response = self
            .run_script(
                serde_json::to_vec(&ScriptRequest {
                    action: "execute_prompt",
                    session_id: &request.session_id,
                    command_text: &built_command.command_text,
                    output_path: &built_command.output_path,
                    error_path: &built_command.error_path,
                    status_path: &built_command.status_path,
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

fn build_claude_cli_command(request: &ItermExecutionRequest) -> Result<BuiltCommand, String> {
    let extras = parse_claude_cli_extras(&request.extra_params_json)?;
    let execution_root = "/tmp/iterm-mcp-tools";
    let sanitized_request_id = sanitize_shell_token(&request.request_id);
    let output_path = format!("{execution_root}/{sanitized_request_id}.out");
    let error_path = format!("{execution_root}/{sanitized_request_id}.err");
    let status_path = format!("{execution_root}/{sanitized_request_id}.status");

    let mut environment = provider_env_vars(
        &request.provider,
        &request.api_key,
        &request.base_url,
        &request.model_name,
    );
    environment.extend(extras.env);

    let claude_executable = extras
        .claude_executable
        .unwrap_or_else(|| "claude".to_string());
    let prompt = merge_system_prompt(&request.system_prompt, &request.prompt);

    let mut command_parts = Vec::new();
    command_parts.push(format!("mkdir -p {}", shell_escape(execution_root)));
    command_parts.push(format!(
        "rm -f {} {} {}",
        shell_escape(&output_path),
        shell_escape(&error_path),
        shell_escape(&status_path)
    ));
    if let Some(cwd) = extras.cwd {
        command_parts.push(format!("cd {}", shell_escape(&cwd)));
    }

    let mut invocation_parts = Vec::new();
    if !environment.is_empty() {
        invocation_parts.push("env".to_string());
        for (key, value) in environment {
            invocation_parts.push(format!("{key}={}", shell_escape(&value)));
        }
    }
    invocation_parts.push(shell_escape(&claude_executable));
    invocation_parts.push("--model".to_string());
    invocation_parts.push(shell_escape(&request.model_name));
    for arg in extras.args {
        invocation_parts.push(shell_escape(&arg));
    }
    invocation_parts.push("-p".to_string());
    invocation_parts.push(shell_escape(&prompt));

    command_parts.push(format!(
        "{} > {} 2> {}",
        invocation_parts.join(" "),
        shell_escape(&output_path),
        shell_escape(&error_path)
    ));
    command_parts.push(format!(
        "printf '%s' \"$?\" > {}",
        shell_escape(&status_path)
    ));

    Ok(BuiltCommand {
        command_text: format!("{}\n", command_parts.join(" && ")),
        output_path,
        error_path,
        status_path,
    })
}

pub fn build_binding_sync_command(
    profile_name: &str,
    provider: &str,
    model_name: &str,
    execution_mode: &str,
    base_url: &str,
    api_key: &str,
) -> String {
    let mut commands = Vec::new();
    for (key, value) in provider_env_vars(provider, api_key, base_url, model_name) {
        commands.push(format!("export {key}={}", shell_escape(&value)));
    }
    commands.push(format!(
        "printf '%s\\n' {}",
        shell_escape(&format!(
            "[iterm-mcp-tools] Bound profile '{profile_name}' ({provider}/{model_name}) to this window. Next run will use {model_name} via {}.",
            execution_mode_label(execution_mode)
        ))
    ));

    format!("{}\n", commands.join(" && "))
}

pub fn build_claude_cli_launch_command(request: &ItermExecutionRequest) -> Result<String, String> {
    let extras = parse_claude_cli_extras(&request.extra_params_json)?;
    let mut commands = Vec::new();

    for (key, value) in provider_env_vars(
        &request.provider,
        &request.api_key,
        &request.base_url,
        &request.model_name,
    ) {
        commands.push(format!("export {key}={}", shell_escape(&value)));
    }
    for (key, value) in extras.env {
        commands.push(format!("export {key}={}", shell_escape(&value)));
    }
    if let Some(cwd) = extras.cwd {
        commands.push(format!("cd {}", shell_escape(&cwd)));
    }
    commands.push(format!(
        "printf '%s\\n' {}",
        shell_escape(&format!(
            "[iterm-mcp-tools] Starting interactive Claude session for {} ({}/{})",
            request.request_id, request.provider, request.model_name
        ))
    ));

    let claude_executable = extras
        .claude_executable
        .unwrap_or_else(|| "claude".to_string());
    let mut invocation_parts = vec![
        shell_escape(&claude_executable),
        "--model".to_string(),
        shell_escape(&request.model_name),
    ];
    for arg in extras.args {
        invocation_parts.push(shell_escape(&arg));
    }
    commands.push(invocation_parts.join(" "));

    Ok(format!("{}\n", commands.join(" && ")))
}

fn execution_mode_label(execution_mode: &str) -> &'static str {
    match execution_mode {
        "openai_chat" => "OpenAI Chat API",
        _ => "Claude Code",
    }
}

fn provider_env_vars(
    provider: &str,
    api_key: &str,
    base_url: &str,
    model_name: &str,
) -> BTreeMap<String, String> {
    let normalized_provider = provider.trim().to_ascii_lowercase();
    let uses_anthropic_env = matches!(normalized_provider.as_str(), "" | "anthropic" | "claude");

    let mut environment = BTreeMap::new();
    if uses_anthropic_env {
        if !api_key.trim().is_empty() {
            environment.insert("ANTHROPIC_API_KEY".to_string(), api_key.to_string());
            environment.insert("ANTHROPIC_AUTH_TOKEN".to_string(), api_key.to_string());
        }
        if !base_url.trim().is_empty() {
            environment.insert("ANTHROPIC_BASE_URL".to_string(), base_url.to_string());
        }
        if !model_name.trim().is_empty() {
            environment.insert("ANTHROPIC_MODEL".to_string(), model_name.to_string());
        }
    } else {
        if !api_key.trim().is_empty() {
            environment.insert("OPENAI_API_KEY".to_string(), api_key.to_string());
        }
        if !base_url.trim().is_empty() {
            environment.insert("OPENAI_BASE_URL".to_string(), base_url.to_string());
        }
        if !model_name.trim().is_empty() {
            environment.insert("OPENAI_MODEL".to_string(), model_name.to_string());
        }
    }

    environment
}

fn parse_claude_cli_extras(raw: &str) -> Result<ClaudeCliExtras, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(ClaudeCliExtras::default());
    }

    serde_json::from_str::<ClaudeCliExtras>(trimmed)
        .map_err(|error| format!("invalid extra_params_json for Claude CLI execution: {error}"))
}

fn merge_system_prompt(system_prompt: &str, prompt: &str) -> String {
    let trimmed_system_prompt = system_prompt.trim();
    if trimmed_system_prompt.is_empty() {
        return prompt.to_string();
    }

    format!("## System Instructions\n{trimmed_system_prompt}\n\n{prompt}")
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sanitize_shell_token(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
            _ => '_',
        })
        .collect()
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

        let output = child
            .wait_with_output()
            .await
            .map_err(|error| error.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::{
        build_binding_sync_command, build_claude_cli_command, build_claude_cli_launch_command,
        ItermExecutionRequest,
    };

    #[test]
    fn builds_claude_shell_command_with_env_and_args() {
        let request = ItermExecutionRequest {
            request_id: "target-1".to_string(),
            session_id: "session-1".to_string(),
            prompt: "Summarize this codebase".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet-4-20250514".to_string(),
            base_url: "https://litellm.example.com".to_string(),
            api_key: "test-api-key".to_string(),
            system_prompt: String::new(),
            extra_params_json: "{}".to_string(),
        };

        let command = build_claude_cli_command(&request).expect("command should build");
        assert!(
            command
                .command_text
                .contains("ANTHROPIC_MODEL='claude-sonnet-4-20250514'"),
            "expected model env var in command: {}",
            command.command_text
        );
        assert!(
            command
                .command_text
                .contains("ANTHROPIC_BASE_URL='https://litellm.example.com'"),
            "expected base url env var in command: {}",
            command.command_text
        );
        assert!(
            command
                .command_text
                .contains("ANTHROPIC_API_KEY='test-api-key'"),
            "expected api key env var in command: {}",
            command.command_text
        );
        assert!(
            command.command_text.contains(
                "'claude' --model 'claude-sonnet-4-20250514' -p 'Summarize this codebase'"
            ),
            "expected claude invocation with model arg and prompt: {}",
            command.command_text
        );
    }

    #[test]
    fn merges_extra_env_args_and_cwd_into_claude_shell_command() {
        let request = ItermExecutionRequest {
            request_id: "target:2".to_string(),
            session_id: "session-2".to_string(),
            prompt: "Inspect the service object".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "openai-compatible".to_string(),
            model_name: "gpt-4.1".to_string(),
            base_url: "https://gateway.example.com".to_string(),
            api_key: "gateway-secret".to_string(),
            system_prompt: "Answer in bullet points.".to_string(),
            extra_params_json: r#"{
              "claudeExecutable":"claude-dev",
              "cwd":"/tmp/project",
              "args":["--permission-mode","bypassPermissions"],
              "env":{"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1"}
            }"#
            .to_string(),
        };

        let command = build_claude_cli_command(&request).expect("command should build");

        assert!(command.command_text.contains("cd '/tmp/project'"));
        assert!(command
            .command_text
            .contains("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC='1'"));
        assert!(command
            .command_text
            .contains("'claude-dev' --model 'gpt-4.1' '--permission-mode' 'bypassPermissions' -p"));
        assert!(command.command_text.contains("## System Instructions"));
        assert_eq!(command.output_path, "/tmp/iterm-mcp-tools/target_2.out");
        assert_eq!(command.status_path, "/tmp/iterm-mcp-tools/target_2.status");
    }

    #[test]
    fn rejects_invalid_extra_params_json() {
        let request = ItermExecutionRequest {
            request_id: "target-3".to_string(),
            session_id: "session-3".to_string(),
            prompt: "Hello".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "secret".to_string(),
            system_prompt: String::new(),
            extra_params_json: "{not-json}".to_string(),
        };

        let error = build_claude_cli_command(&request).expect_err("command should fail");
        assert!(error.contains("invalid extra_params_json"));
    }

    #[test]
    fn omits_system_instructions_wrapper_when_empty() {
        let request = ItermExecutionRequest {
            request_id: "target-4".to_string(),
            session_id: "session-4".to_string(),
            prompt: "Plain prompt".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "secret".to_string(),
            system_prompt: "   ".to_string(),
            extra_params_json: "{}".to_string(),
        };

        let command = build_claude_cli_command(&request).expect("command should build");
        assert!(!command.command_text.contains("## System Instructions"));
        assert!(command.command_text.contains("-p 'Plain prompt'"));
    }

    #[test]
    fn binding_sync_command_uses_claude_label_for_claude_cli_mode() {
        let command = build_binding_sync_command(
            "Claude Profile",
            "anthropic",
            "claude-sonnet-4",
            "claude_cli",
            "https://api.anthropic.com",
            "secret",
        );

        assert!(command.contains("via Claude Code"));
    }

    #[test]
    fn binding_sync_command_uses_openai_label_for_openai_mode() {
        let command = build_binding_sync_command(
            "OpenAI Profile",
            "openai",
            "gpt-4.1",
            "openai_chat",
            "https://api.openai.com/v1",
            "secret",
        );

        assert!(command.contains("via OpenAI Chat API"));
        assert!(!command.contains("via Claude Code"));
    }

    #[test]
    fn binding_sync_command_uses_openai_env_for_openai_compatible_claude_cli() {
        let command = build_binding_sync_command(
            "GLM Profile",
            "openai-compatible",
            "glm-4.5",
            "claude_cli",
            "https://gateway.example.com/v1",
            "glm-secret",
        );

        assert!(command.contains("export OPENAI_API_KEY='glm-secret'"));
        assert!(command.contains("export OPENAI_BASE_URL='https://gateway.example.com/v1'"));
        assert!(command.contains("export OPENAI_MODEL='glm-4.5'"));
        assert!(!command.contains("export ANTHROPIC_API_KEY"));
    }

    #[test]
    fn launch_command_uses_openai_env_for_openai_compatible_claude_cli() {
        let request = ItermExecutionRequest {
            request_id: "launch-openai-compatible".to_string(),
            session_id: "session-launch".to_string(),
            prompt: "Ignored".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "openai-compatible".to_string(),
            model_name: "glm-4.5".to_string(),
            base_url: "https://gateway.example.com/v1".to_string(),
            api_key: "glm-secret".to_string(),
            system_prompt: String::new(),
            extra_params_json: "{}".to_string(),
        };

        let command = build_claude_cli_launch_command(&request).expect("command should build");
        assert!(command.contains("export OPENAI_API_KEY='glm-secret'"));
        assert!(command.contains("export OPENAI_BASE_URL='https://gateway.example.com/v1'"));
        assert!(command.contains("export OPENAI_MODEL='glm-4.5'"));
        assert!(!command.contains("export ANTHROPIC_API_KEY"));
    }

    #[test]
    fn launch_command_builder_keeps_claude_cli_invocation_shape() {
        let request = ItermExecutionRequest {
            request_id: "launch-1".to_string(),
            session_id: "session-launch".to_string(),
            prompt: "Ignored for interactive launch".to_string(),
            execution_mode: "claude_cli".to_string(),
            provider: "anthropic".to_string(),
            model_name: "claude-sonnet-4".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            api_key: "launch-secret".to_string(),
            system_prompt: String::new(),
            extra_params_json: r#"{
              "claudeExecutable":"claude-dev",
              "args":["--dangerously-skip-permissions"]
            }"#
            .to_string(),
        };

        let command = build_claude_cli_launch_command(&request).expect("command should build");
        assert!(command.contains("Starting interactive Claude session for launch-1"));
        assert!(command
            .contains("'claude-dev' --model 'claude-sonnet-4' '--dangerously-skip-permissions'"));
    }
}
