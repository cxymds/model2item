use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::services::iterm_mcp_adapter::ItermExecutionRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenaiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct OpenaiChatExecutor {
    client: reqwest::Client,
}

impl Default for OpenaiChatExecutor {
    fn default() -> Self {
        Self::new(reqwest::Client::new())
    }
}

impl OpenaiChatExecutor {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn execute_chat_completion(
        &self,
        request: &ItermExecutionRequest,
    ) -> Result<String, String> {
        let url = normalize_chat_completions_url(&request.base_url)?;
        let headers = build_openai_headers(&request.api_key)?;
        let payload = build_openai_chat_payload(request)?;
        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|error| format!("failed to send OpenAI chat completion request: {error}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("failed to read OpenAI chat completion response: {error}"))?;

        if !status.is_success() {
            return Err(format!(
                "OpenAI chat completion request failed with status {status}: {body}"
            ));
        }

        extract_first_assistant_text(&body)
    }
}

pub fn normalize_chat_completions_url(base_url: &str) -> Result<String, String> {
    let normalized_base_url = base_url.trim().trim_end_matches('/');
    if normalized_base_url.is_empty() {
        return Err("base_url cannot be empty for OpenAI Chat execution".to_string());
    }

    if normalized_base_url.ends_with("/chat/completions") {
        return Ok(normalized_base_url.to_string());
    }
    if normalized_base_url.ends_with("/v1") {
        return Ok(format!("{normalized_base_url}/chat/completions"));
    }

    Ok(format!("{normalized_base_url}/v1/chat/completions"))
}

pub fn build_openai_headers(api_key: &str) -> Result<HeaderMap, String> {
    let trimmed_api_key = api_key.trim();
    if trimmed_api_key.is_empty() {
        return Err("api_key cannot be empty for OpenAI Chat execution".to_string());
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {trimmed_api_key}"))
            .map_err(|error| format!("failed to build authorization header: {error}"))?,
    );

    Ok(headers)
}

pub fn build_openai_messages(system_prompt: &str, prompt: &str) -> Vec<OpenaiChatMessage> {
    let mut messages = Vec::new();
    let normalized_system_prompt = system_prompt.trim();
    if !normalized_system_prompt.is_empty() {
        messages.push(OpenaiChatMessage {
            role: "system".to_string(),
            content: normalized_system_prompt.to_string(),
        });
    }
    messages.push(OpenaiChatMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    });

    messages
}

pub fn build_openai_chat_payload(request: &ItermExecutionRequest) -> Result<Value, String> {
    if request.model_name.trim().is_empty() {
        return Err("model_name cannot be empty for OpenAI Chat execution".to_string());
    }
    if request.prompt.trim().is_empty() {
        return Err("prompt cannot be empty for OpenAI Chat execution".to_string());
    }

    let mut payload = Map::new();
    payload.insert(
        "model".to_string(),
        Value::String(request.model_name.clone()),
    );
    payload.insert(
        "messages".to_string(),
        serde_json::to_value(build_openai_messages(
            &request.system_prompt,
            &request.prompt,
        ))
        .map_err(|error| format!("failed to serialize OpenAI chat messages: {error}"))?,
    );
    payload.extend(parse_whitelisted_extra_params(&request.extra_params_json)?);

    Ok(Value::Object(payload))
}

pub fn extract_first_assistant_text(response_body: &str) -> Result<String, String> {
    let response: OpenaiChatCompletionResponse = serde_json::from_str(response_body)
        .map_err(|error| format!("invalid OpenAI chat completion response: {error}"))?;
    let content = response
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .ok_or_else(|| {
            "OpenAI chat completion response did not include assistant content".to_string()
        })?;

    match content {
        Value::String(text) => Ok(text),
        Value::Array(parts) => {
            let merged_text = parts
                .iter()
                .filter_map(|part| part.get("text"))
                .filter_map(Value::as_str)
                .collect::<String>();
            if merged_text.is_empty() {
                Err("OpenAI chat completion response did not include assistant content".to_string())
            } else {
                Ok(merged_text)
            }
        }
        _ => Err("OpenAI chat completion response did not include assistant content".to_string()),
    }
}

fn parse_whitelisted_extra_params(raw: &str) -> Result<Map<String, Value>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Map::new());
    }

    let parsed = serde_json::from_str::<Value>(trimmed)
        .map_err(|error| format!("invalid extra_params_json for OpenAI Chat execution: {error}"))?;
    let object = parsed.as_object().ok_or_else(|| {
        "invalid extra_params_json for OpenAI Chat execution: expected JSON object".to_string()
    })?;

    let mut passthrough = Map::new();
    if let Some(temperature) = object.get("temperature").filter(|value| value.is_number()) {
        passthrough.insert("temperature".to_string(), temperature.clone());
    }
    if let Some(max_tokens) = object
        .get("max_tokens")
        .filter(|value| value.as_u64().is_some())
    {
        passthrough.insert("max_tokens".to_string(), max_tokens.clone());
    }

    Ok(passthrough)
}

#[derive(Debug, Deserialize)]
struct OpenaiChatCompletionResponse {
    choices: Vec<OpenaiChatChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenaiChatChoice {
    message: OpenaiChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct OpenaiChatChoiceMessage {
    content: Option<Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use reqwest::header::AUTHORIZATION;

    use super::{
        build_openai_chat_payload, build_openai_headers, build_openai_messages,
        extract_first_assistant_text, normalize_chat_completions_url,
    };
    use crate::services::iterm_mcp_adapter::ItermExecutionRequest;

    fn sample_request() -> ItermExecutionRequest {
        ItermExecutionRequest {
            request_id: "task4".to_string(),
            session_id: "session-1".to_string(),
            prompt: "Summarize the diff".to_string(),
            execution_mode: "openai_chat".to_string(),
            provider: "openai".to_string(),
            model_name: "gpt-4.1-mini".to_string(),
            base_url: "https://api.openai.com/v1/".to_string(),
            api_key: "secret".to_string(),
            system_prompt: "You are concise".to_string(),
            extra_params_json: r#"{"temperature":0.2,"max_tokens":256,"top_p":0.9}"#.to_string(),
        }
    }

    #[test]
    fn normalize_url_appends_chat_completions_for_v1_base() {
        let url = normalize_chat_completions_url("https://api.openai.com/v1/")
            .expect("url should be normalized");
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn normalize_url_adds_v1_prefix_when_missing() {
        let url = normalize_chat_completions_url("https://gateway.example.com/")
            .expect("url should be normalized");
        assert_eq!(url, "https://gateway.example.com/v1/chat/completions");
    }

    #[test]
    fn normalize_url_keeps_existing_chat_completion_path() {
        let url = normalize_chat_completions_url("https://proxy.example.com/v1/chat/completions/")
            .expect("url should be normalized");
        assert_eq!(url, "https://proxy.example.com/v1/chat/completions");
    }

    #[test]
    fn build_messages_includes_system_and_user_when_system_present() {
        let messages = build_openai_messages("System prompt", "User prompt");
        assert_eq!(
            messages,
            vec![
                super::OpenaiChatMessage {
                    role: "system".to_string(),
                    content: "System prompt".to_string()
                },
                super::OpenaiChatMessage {
                    role: "user".to_string(),
                    content: "User prompt".to_string()
                }
            ]
        );
    }

    #[test]
    fn build_headers_includes_bearer_token() {
        let headers = build_openai_headers("sk-test").expect("headers should build");
        let auth = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .expect("authorization header should exist");
        assert_eq!(auth, "Bearer sk-test");
    }

    #[test]
    fn payload_contains_model_messages_and_whitelisted_extras() {
        let payload = build_openai_chat_payload(&sample_request()).expect("payload should build");
        assert_eq!(payload["model"], json!("gpt-4.1-mini"));
        assert_eq!(payload["messages"][0]["role"], json!("system"));
        assert_eq!(payload["messages"][1]["role"], json!("user"));
        assert_eq!(payload["temperature"], json!(0.2));
        assert_eq!(payload["max_tokens"], json!(256));
        assert!(payload.get("top_p").is_none());
    }

    #[test]
    fn payload_rejects_invalid_extra_params_json() {
        let mut request = sample_request();
        request.extra_params_json = "{not-json}".to_string();
        let error = build_openai_chat_payload(&request).expect_err("payload should fail");
        assert!(error.contains("invalid extra_params_json"));
    }

    #[test]
    fn extract_first_assistant_text_returns_first_choice_content() {
        let body = json!({
            "id":"chatcmpl-test",
            "choices":[
                {"message":{"role":"assistant","content":"First answer"}},
                {"message":{"role":"assistant","content":"Second answer"}}
            ]
        })
        .to_string();

        let text = extract_first_assistant_text(&body).expect("response should parse");
        assert_eq!(text, "First answer");
    }

    #[test]
    fn extract_first_assistant_text_errors_when_missing() {
        let body = json!({
            "choices":[{"message":{"role":"assistant"}}]
        })
        .to_string();
        let error =
            extract_first_assistant_text(&body).expect_err("missing content should return error");
        assert!(error.contains("did not include assistant content"));
    }
}
