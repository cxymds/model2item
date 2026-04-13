mod support;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use iterm_mcp_tools_lib::{
    error::AppError,
    models::{
        comparison_run::CreateComparisonRunInput, evaluation_case::CreateEvaluationCaseInput,
        profile::CreateProfileInput, window_binding::CreateWindowBindingInput,
    },
    services::{
        comparison_orchestrator::{ComparisonOrchestrator, OpenaiChatCompletionExecutor},
        comparison_run_service::ComparisonRunService,
        evaluation_case_service::EvaluationCaseService,
        iterm_mcp_adapter::{
            ItermExecutionRequest, ItermExecutionResult, ItermMcpAdapter, ItermSessionInfo,
        },
        profile_service::ProfileService,
        secret_store::{MemorySecretStore, SecretStore},
        window_binding_service::WindowBindingService,
    },
};

#[derive(Clone, Default)]
struct FakeAdapter {
    sent_texts: Arc<Mutex<Vec<(String, String)>>>,
    screens: Arc<Mutex<HashMap<String, String>>>,
    screen_reads: Arc<Mutex<HashMap<String, usize>>>,
}

#[async_trait]
impl ItermMcpAdapter for FakeAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(vec![
            ItermSessionInfo {
                session_id: "session-ok".to_string(),
                window_id: "window-1".to_string(),
                window_title: "Window 1".to_string(),
                tab_id: "tab-1".to_string(),
                tab_title: "Tab 1".to_string(),
                session_title: "Pane OK".to_string(),
            },
            ItermSessionInfo {
                session_id: "session-fail".to_string(),
                window_id: "window-2".to_string(),
                window_title: "Window 2".to_string(),
                tab_id: "tab-2".to_string(),
                tab_title: "Tab 2".to_string(),
                session_title: "Pane Fail".to_string(),
            },
        ])
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        if session_id == "session-fail" {
            return Err("simulated adapter failure".to_string());
        }

        self.sent_texts
            .lock()
            .expect("sent_texts mutex")
            .push((session_id.to_string(), text.to_string()));
        let mut screens = self.screens.lock().expect("screens mutex");
        let current = screens.entry(session_id.to_string()).or_default();
        current.push_str(text);
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let mut screen_reads = self.screen_reads.lock().expect("screen_reads mutex");
        let current_read = screen_reads.entry(session_id.to_string()).or_insert(0);
        *current_read += 1;

        let screen_text = self
            .screens
            .lock()
            .expect("screens mutex")
            .get(session_id)
            .cloned()
            .unwrap_or_default();

        if session_id != "session-ok" {
            return Ok(screen_text);
        }

        if screen_text.contains("Continue with parser edge cases") {
            return Ok(match *current_read {
                1..=3 => screen_text.clone(),
                4 | 5 => screen_text.clone(),
                _ => format!("{screen_text}\nAssistant follow-up result"),
            });
        }

        Ok(match *current_read {
            1 => screen_text.clone(),
            _ => format!("{screen_text}\nAssistant initial result"),
        })
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        Ok(ItermExecutionResult {
            output_text: format!(
                "session={} provider={} model={}",
                request.session_id, request.provider, request.model_name
            ),
        })
    }
}

#[derive(Clone, Default)]
struct DelayedOutputAdapter {
    sent_texts: Arc<Mutex<Vec<(String, String)>>>,
    screen_reads: Arc<Mutex<HashMap<String, usize>>>,
}

#[derive(Clone, Default)]
struct ClosingSessionAdapter {
    sent_texts: Arc<Mutex<Vec<(String, String)>>>,
    screen_reads: Arc<Mutex<HashMap<String, usize>>>,
}

#[derive(Clone, Default)]
struct OpenaiOnlyAdapter {
    sent_texts: Arc<Mutex<Vec<(String, String)>>>,
    screen_reads: Arc<Mutex<HashMap<String, usize>>>,
}

#[derive(Clone, Default)]
struct EarlyFailureScreenAdapter {
    sent_texts: Arc<Mutex<Vec<(String, String)>>>,
    screen_reads: Arc<Mutex<HashMap<String, usize>>>,
}

#[async_trait]
impl ItermMcpAdapter for OpenaiOnlyAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(vec![ItermSessionInfo {
            session_id: "session-openai".to_string(),
            window_id: "window-openai".to_string(),
            window_title: "Window OpenAI".to_string(),
            tab_id: "tab-openai".to_string(),
            tab_title: "Tab OpenAI".to_string(),
            session_title: "Pane OpenAI".to_string(),
        }])
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.sent_texts
            .lock()
            .expect("sent_texts mutex")
            .push((session_id.to_string(), text.to_string()));
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let mut screen_reads = self.screen_reads.lock().expect("screen_reads mutex");
        let current = screen_reads.entry(session_id.to_string()).or_insert(0);
        *current += 1;
        Ok(format!("unexpected screen read {}", current))
    }

    async fn execute_prompt(
        &self,
        _request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        Err("adapter execute_prompt should not be used for openai_chat".to_string())
    }
}

#[async_trait]
impl ItermMcpAdapter for EarlyFailureScreenAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        let reads = self
            .screen_reads
            .lock()
            .expect("screen_reads mutex")
            .get("session-early-fail")
            .copied()
            .unwrap_or_default();

        if reads >= 10 {
            Ok(vec![])
        } else {
            Ok(vec![ItermSessionInfo {
                session_id: "session-early-fail".to_string(),
                window_id: "window-early-fail".to_string(),
                window_title: "Window Early Fail".to_string(),
                tab_id: "tab-early-fail".to_string(),
                tab_title: "Tab Early Fail".to_string(),
                session_title: "Pane Early Fail".to_string(),
            }])
        }
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.sent_texts
            .lock()
            .expect("sent_texts mutex")
            .push((session_id.to_string(), text.to_string()));
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let mut screen_reads = self.screen_reads.lock().expect("screen_reads mutex");
        let current = screen_reads.entry(session_id.to_string()).or_insert(0);
        *current += 1;

        Ok("error: missing auth token for upstream provider".to_string())
    }

    async fn execute_prompt(
        &self,
        _request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        Err("execute_prompt should not be used for interactive CLI targets".to_string())
    }
}

#[async_trait]
impl ItermMcpAdapter for ClosingSessionAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        let reads = self
            .screen_reads
            .lock()
            .expect("screen_reads mutex")
            .get("session-closing")
            .copied()
            .unwrap_or_default();

        if reads >= 10 {
            Ok(vec![])
        } else {
            Ok(vec![ItermSessionInfo {
                session_id: "session-closing".to_string(),
                window_id: "window-closing".to_string(),
                window_title: "Window Closing".to_string(),
                tab_id: "tab-closing".to_string(),
                tab_title: "Tab Closing".to_string(),
                session_title: "Pane Closing".to_string(),
            }])
        }
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.sent_texts
            .lock()
            .expect("sent_texts mutex")
            .push((session_id.to_string(), text.to_string()));
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let mut screen_reads = self.screen_reads.lock().expect("screen_reads mutex");
        let current = screen_reads.entry(session_id.to_string()).or_insert(0);
        *current += 1;
        Ok(String::new())
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        Ok(ItermExecutionResult {
            output_text: format!(
                "session={} provider={} model={}",
                request.session_id, request.provider, request.model_name
            ),
        })
    }
}

#[derive(Clone)]
struct FakeOpenaiExecutor {
    requests: Arc<Mutex<Vec<ItermExecutionRequest>>>,
    response_text: Arc<Mutex<String>>,
}

impl Default for FakeOpenaiExecutor {
    fn default() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            response_text: Arc::new(Mutex::new("OpenAI direct result".to_string())),
        }
    }
}

#[async_trait]
impl OpenaiChatCompletionExecutor for FakeOpenaiExecutor {
    async fn execute_chat_completion(
        &self,
        request: &ItermExecutionRequest,
    ) -> Result<String, String> {
        self.requests
            .lock()
            .expect("requests mutex")
            .push(request.clone());
        Ok(self
            .response_text
            .lock()
            .expect("response_text mutex")
            .clone())
    }
}

#[async_trait]
impl ItermMcpAdapter for DelayedOutputAdapter {
    async fn list_sessions(&self) -> Result<Vec<ItermSessionInfo>, String> {
        Ok(vec![ItermSessionInfo {
            session_id: "session-delayed".to_string(),
            window_id: "window-1".to_string(),
            window_title: "Window 1".to_string(),
            tab_id: "tab-1".to_string(),
            tab_title: "Tab 1".to_string(),
            session_title: "Pane Delayed".to_string(),
        }])
    }

    async fn send_text(&self, session_id: &str, text: &str) -> Result<(), String> {
        self.sent_texts
            .lock()
            .expect("sent_texts mutex")
            .push((session_id.to_string(), text.to_string()));
        Ok(())
    }

    async fn get_screen_text(&self, session_id: &str) -> Result<String, String> {
        let mut screen_reads = self.screen_reads.lock().expect("screen_reads mutex");
        let current = screen_reads.entry(session_id.to_string()).or_insert(0);
        *current += 1;
        if *current < 3 {
            Ok(String::new())
        } else {
            Ok("Claude 已进入会话，并开始输出首段结果。".to_string())
        }
    }

    async fn execute_prompt(
        &self,
        request: ItermExecutionRequest,
    ) -> Result<ItermExecutionResult, String> {
        Ok(ItermExecutionResult {
            output_text: format!(
                "session={} provider={} model={}",
                request.session_id, request.provider, request.model_name
            ),
        })
    }
}

#[tokio::test]
async fn executes_run_and_persists_target_outcomes() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = FakeAdapter::default();
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
    );

    let success_profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Success".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "success-secret".to_string(),
        })
        .await?;
    let fail_profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "fail-secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Old code understanding".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let success_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-ok".to_string(),
            display_name: "Window A".to_string(),
            profile_id: success_profile.id.clone(),
        })
        .await?;
    let fail_binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-fail".to_string(),
            display_name: "Window B".to_string(),
            profile_id: fail_profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Legacy compare".to_string(),
            target_ids: vec![success_binding.id.clone(), fail_binding.id.clone()],
            notes: Some("Execute both windows".to_string()),
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "failed");
    assert!(updated_run.started_at.is_some());
    assert!(updated_run.finished_at.is_some());

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets.len(), 2);

    let success_target = targets
        .iter()
        .find(|target| target.window_binding_id == success_binding.id)
        .expect("success target should exist");
    assert_eq!(success_target.status, "running");
    assert_eq!(success_target.success_status.as_deref(), Some("streaming"));
    assert!(success_target.sent_at.is_some());
    assert!(success_target.first_response_at.is_some());
    assert!(success_target.finished_at.is_none());
    assert_eq!(success_target.duration_ms, None);
    assert!(success_target.response_chars > 0);
    assert!(success_target.response_lines > 0);
    assert_eq!(success_target.error_category, None);
    assert_eq!(
        success_target.latest_message_role.as_deref(),
        Some("assistant")
    );
    assert_eq!(
        success_target.latest_message_content.as_deref(),
        Some("Assistant initial result")
    );

    let fail_target = targets
        .iter()
        .find(|target| target.window_binding_id == fail_binding.id)
        .expect("failed target should exist");
    assert_eq!(fail_target.status, "failed");
    assert_eq!(fail_target.error_category.as_deref(), Some("adapter_error"));
    assert_eq!(
        fail_target.error_detail.as_deref(),
        Some("simulated adapter failure")
    );
    assert!(fail_target.sent_at.is_some());
    assert!(fail_target.finished_at.is_some());
    assert_eq!(fail_target.success_status.as_deref(), Some("failed"));
    assert_eq!(fail_target.latest_message_role.as_deref(), Some("system"));

    let sent_texts = adapter.sent_texts.lock().expect("sent_texts mutex");
    assert!(sent_texts.iter().any(|(session_id, text)| {
        session_id == "session-ok" && text.contains("Starting interactive Claude session")
    }));
    assert!(sent_texts.iter().any(|(session_id, text)| {
        session_id == "session-ok" && text.contains("Summarize the core control flow")
    }));

    let stored_messages =
        sqlx::query_scalar::<_, String>("SELECT content FROM messages ORDER BY created_at ASC")
            .fetch_all(&pool)
            .await?;
    assert_eq!(stored_messages.len(), 4);
    let combined_messages = stored_messages.join("\n---\n");
    assert!(combined_messages.contains("Summarize the core control flow"));
    assert!(combined_messages.contains("Assistant initial result"));
    assert!(combined_messages.contains("simulated adapter failure"));

    Ok(())
}

#[tokio::test]
async fn broadcasts_follow_up_input_into_running_target_sessions(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = FakeAdapter::default();
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Old code understanding".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-ok".to_string(),
            display_name: "Window A".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Legacy compare".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;
    orchestrator
        .broadcast_message(&run.id, "Continue with parser edge cases")
        .await?;

    let sent_texts = adapter.sent_texts.lock().expect("sent_texts mutex");
    let follow_up_count = sent_texts
        .iter()
        .filter(|(session_id, text)| {
            session_id == "session-ok" && text.contains("Continue with parser edge cases")
        })
        .count();
    assert_eq!(follow_up_count, 1);

    let stored_messages =
        sqlx::query_scalar::<_, String>("SELECT content FROM messages ORDER BY created_at ASC")
            .fetch_all(&pool)
            .await?;
    let combined_messages = stored_messages.join("\n---\n");
    assert!(combined_messages.contains("Continue with parser edge cases"));
    assert!(combined_messages.contains("Assistant initial result"));
    assert!(combined_messages.contains("Assistant follow-up result"));
    assert!(!combined_messages.contains("Starting interactive Claude session"));

    Ok(())
}

#[tokio::test]
async fn rejects_run_when_target_session_is_offline() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        FakeAdapter::default(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Offline".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "success-secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Old code understanding".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-offline".to_string(),
            display_name: "Window Offline".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Offline compare".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let result = orchestrator.execute_run(&run.id).await;
    assert!(matches!(result, Err(AppError::InvalidInput(_))));
    assert!(result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
        .contains("offline"));

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "queued");
    assert!(updated_run.started_at.is_none());

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets[0].status, "queued");

    Ok(())
}

#[tokio::test]
async fn openai_chat_run_does_not_require_online_iterm_session(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = FakeAdapter::default();
    let openai_executor = Arc::new(FakeOpenaiExecutor::default());
    let orchestrator = ComparisonOrchestrator::with_dependencies_and_openai_executor(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
        openai_executor.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4 Offline".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "success-secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "OpenAI offline session case".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{\"files\":[\"legacy/service.rb\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-offline".to_string(),
            display_name: "Window Offline".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "OpenAI offline compare".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.validate_run_startup(&run.id).await?;
    orchestrator.execute_run(&run.id).await?;

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "done");
    assert!(updated_run.started_at.is_some());

    let targets = run_service.list_comparison_targets(&run.id).await?;
    assert_eq!(targets[0].status, "done");
    assert_eq!(targets[0].success_status.as_deref(), Some("completed"));
    assert_eq!(
        targets[0].latest_message_content.as_deref(),
        Some("OpenAI direct result")
    );

    assert!(adapter
        .sent_texts
        .lock()
        .expect("sent_texts mutex")
        .is_empty());
    assert!(adapter
        .screen_reads
        .lock()
        .expect("screen_reads mutex")
        .is_empty());

    let requests = openai_executor.requests.lock().expect("requests mutex");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].session_id, "session-offline");
    assert_eq!(requests[0].execution_mode, "openai_chat");

    Ok(())
}

#[tokio::test]
async fn waits_for_delayed_interactive_output_before_persisting_target_preview(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = DelayedOutputAdapter::default();
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Delayed".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Delayed output case".to_string(),
            prompt: "Wait for the model to respond".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-delayed".to_string(),
            display_name: "Window Delayed".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Delayed interactive output".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let targets = run_service.list_comparison_targets(&run.id).await?;
    let target = targets
        .iter()
        .find(|target| target.window_binding_id == binding.id)
        .expect("target should exist");

    assert_eq!(target.latest_message_role.as_deref(), Some("assistant"));
    assert_eq!(
        target.latest_message_content.as_deref(),
        Some("Claude 已进入会话，并开始输出首段结果。")
    );
    assert_eq!(
        target.response_chars,
        "Claude 已进入会话，并开始输出首段结果。".chars().count() as i64
    );
    assert!(
        adapter
            .screen_reads
            .lock()
            .expect("screen_reads mutex")
            .get("session-delayed")
            .copied()
            .unwrap_or_default()
            >= 3
    );

    Ok(())
}

#[tokio::test]
async fn fails_early_when_terminal_session_closes_before_new_output_arrives(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = ClosingSessionAdapter::default();
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Closing".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Closing session case".to_string(),
            prompt: "Wait for terminal output".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-closing".to_string(),
            display_name: "Window Closing".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Closing terminal run".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "failed");

    let targets = run_service.list_comparison_targets(&run.id).await?;
    let target = targets
        .iter()
        .find(|target| target.window_binding_id == binding.id)
        .expect("target should exist");
    assert_eq!(target.status, "failed");
    assert_eq!(target.error_category.as_deref(), Some("adapter_error"));
    assert!(target
        .error_detail
        .as_deref()
        .unwrap_or_default()
        .contains("session session-closing closed before new interactive output arrived"));

    Ok(())
}

#[tokio::test]
async fn surfaces_profile_and_target_when_secret_is_missing_at_startup(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        FakeAdapter::default(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude Missing Secret".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "claude-3.7".to_string(),
            base_url: "https://api.anthropic.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    secret_store.delete_secret(&profile.api_key_encrypted)?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Missing secret case".to_string(),
            prompt: "Summarize the core control flow".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-ok".to_string(),
            display_name: "Window Missing Secret".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "Missing secret run".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let result = orchestrator.validate_run_startup(&run.id).await;
    let error_message = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

    assert!(error_message.contains("profile `Claude Missing Secret`"));
    assert!(error_message.contains("target `Window Missing Secret`"));
    assert!(error_message.contains("re-save the API key"));

    Ok(())
}

#[tokio::test]
async fn openai_chat_targets_complete_without_terminal_incremental_capture(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = OpenaiOnlyAdapter::default();
    let openai_executor = Arc::new(FakeOpenaiExecutor::default());
    let orchestrator = ComparisonOrchestrator::with_dependencies_and_openai_executor(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
        openai_executor.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Direct OpenAI case".to_string(),
            prompt: "Summarize the code path".to_string(),
            context_payload: "{\"files\":[\"parser.rs\"]}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-openai".to_string(),
            display_name: "Window OpenAI".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "OpenAI direct execution".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let updated_run = run_service.get_comparison_run(&run.id).await?;
    assert_eq!(updated_run.status, "done");

    let targets = run_service.list_comparison_targets(&run.id).await?;
    let target = targets
        .iter()
        .find(|target| target.window_binding_id == binding.id)
        .expect("target should exist");
    assert_eq!(target.status, "done");
    assert_eq!(target.success_status.as_deref(), Some("completed"));
    assert_eq!(
        target.latest_message_content.as_deref(),
        Some("OpenAI direct result")
    );

    assert!(adapter
        .sent_texts
        .lock()
        .expect("sent_texts mutex")
        .is_empty());
    assert!(adapter
        .screen_reads
        .lock()
        .expect("screen_reads mutex")
        .is_empty());

    let requests = openai_executor.requests.lock().expect("requests mutex");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].execution_mode, "openai_chat");

    Ok(())
}

#[tokio::test]
async fn cli_target_failure_includes_early_screen_output_in_error_detail(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = EarlyFailureScreenAdapter::default();
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        adapter,
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "Claude CLI GLM".to_string(),
            provider: "anthropic".to_string(),
            execution_mode: "claude_cli".to_string(),
            model_name: "glm5.1".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Early fail case".to_string(),
            prompt: "Summarize the parser".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-early-fail".to_string(),
            display_name: "Window Early Fail".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "CLI early fail".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    orchestrator.execute_run(&run.id).await?;

    let targets = run_service.list_comparison_targets(&run.id).await?;
    let target = targets
        .iter()
        .find(|target| target.window_binding_id == binding.id)
        .expect("failed target should exist");
    assert_eq!(target.status, "failed");
    assert_eq!(target.error_category.as_deref(), Some("adapter_error"));
    assert!(
        target
            .error_detail
            .as_deref()
            .unwrap_or_default()
            .contains("missing auth token for upstream provider")
    );

    Ok(())
}

#[tokio::test]
async fn startup_validation_skips_claude_launch_command_for_openai_chat(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = OpenaiOnlyAdapter::default();
    let openai_executor = Arc::new(FakeOpenaiExecutor::default());
    let orchestrator = ComparisonOrchestrator::with_dependencies_and_openai_executor(
        pool.clone(),
        secret_store.clone(),
        adapter,
        openai_executor,
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    sqlx::query("UPDATE model_profiles SET extra_params_json = '{\"cwd\":123}' WHERE id = ?")
        .bind(&profile.id)
        .execute(&pool)
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "Startup validation".to_string(),
            prompt: "Validate startup".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-openai".to_string(),
            display_name: "Window OpenAI".to_string(),
            profile_id: profile.id,
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "OpenAI startup validation".to_string(),
            target_ids: vec![binding.id],
            notes: None,
        })
        .await?;

    orchestrator.validate_run_startup(&run.id).await?;

    Ok(())
}

#[tokio::test]
async fn broadcasts_follow_up_for_openai_chat_without_terminal_io(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let profile_service = ProfileService::with_secret_store(pool.clone(), secret_store.clone());
    let case_service = EvaluationCaseService::new(pool.clone());
    let binding_service = WindowBindingService::new(pool.clone());
    let run_service = ComparisonRunService::new(pool.clone());
    let adapter = OpenaiOnlyAdapter::default();
    let openai_executor = Arc::new(FakeOpenaiExecutor::default());
    *openai_executor
        .response_text
        .lock()
        .expect("response_text mutex") = "OpenAI follow-up result".to_string();
    let orchestrator = ComparisonOrchestrator::with_dependencies_and_openai_executor(
        pool.clone(),
        secret_store.clone(),
        adapter.clone(),
        openai_executor.clone(),
    );

    let profile = profile_service
        .create_profile(CreateProfileInput {
            name: "GPT 5.4".to_string(),
            provider: "openai".to_string(),
            execution_mode: "openai_chat".to_string(),
            model_name: "gpt-5.4".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "secret".to_string(),
        })
        .await?;

    let case = case_service
        .create_evaluation_case(CreateEvaluationCaseInput {
            title: "OpenAI follow-up case".to_string(),
            prompt: "Initial prompt".to_string(),
            context_payload: "{}".to_string(),
            notes: None,
        })
        .await?;

    let binding = binding_service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-openai".to_string(),
            display_name: "Window OpenAI".to_string(),
            profile_id: profile.id.clone(),
        })
        .await?;

    let run = run_service
        .create_comparison_run(CreateComparisonRunInput {
            evaluation_case_id: case.id,
            title: "OpenAI follow-up run".to_string(),
            target_ids: vec![binding.id.clone()],
            notes: None,
        })
        .await?;

    let target = run_service
        .list_comparison_targets(&run.id)
        .await?
        .into_iter()
        .find(|target| target.window_binding_id == binding.id)
        .expect("target should exist");

    run_service.mark_run_started(&run.id).await?;
    run_service.mark_target_running(&target.id).await?;

    orchestrator
        .broadcast_message(&run.id, "Continue with parser edge cases")
        .await?;

    assert!(adapter
        .sent_texts
        .lock()
        .expect("sent_texts mutex")
        .is_empty());
    assert!(adapter
        .screen_reads
        .lock()
        .expect("screen_reads mutex")
        .is_empty());

    let requests = openai_executor.requests.lock().expect("requests mutex");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].execution_mode, "openai_chat");
    assert_eq!(requests[0].prompt, "Continue with parser edge cases");

    let stored_messages =
        sqlx::query_scalar::<_, String>("SELECT content FROM messages ORDER BY created_at ASC")
            .fetch_all(&pool)
            .await?;
    let combined_messages = stored_messages.join("\n---\n");
    assert!(combined_messages.contains("Continue with parser edge cases"));
    assert!(combined_messages.contains("OpenAI follow-up result"));

    Ok(())
}
