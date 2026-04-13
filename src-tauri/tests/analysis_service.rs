use iterm_mcp_tools_lib::{
    models::comparison_run::{ComparisonRunRecord, ComparisonTargetRecord},
    services::analysis_service::build_comparison_summary,
};

fn sample_run() -> ComparisonRunRecord {
    ComparisonRunRecord {
        id: "run-1".to_string(),
        evaluation_case_id: "case-1".to_string(),
        title: "Legacy parser benchmark".to_string(),
        status: "queued".to_string(),
        prompt_snapshot: "prompt".to_string(),
        context_snapshot: "{}".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        started_at: None,
        finished_at: None,
        notes: String::new(),
    }
}

fn target(
    id: &str,
    status: &str,
    duration_ms: Option<i64>,
    response_chars: i64,
    response_lines: i64,
    success_status: Option<&str>,
    profile_snapshot_json: &str,
) -> ComparisonTargetRecord {
    ComparisonTargetRecord {
        position: 0,
        id: id.to_string(),
        run_id: "run-1".to_string(),
        window_binding_id: "binding-1".to_string(),
        profile_snapshot_json: profile_snapshot_json.to_string(),
        status: status.to_string(),
        sent_at: None,
        first_response_at: None,
        finished_at: None,
        duration_ms,
        response_chars,
        response_lines,
        success_status: success_status.map(ToString::to_string),
        error_category: None,
        error_detail: None,
        latest_message_role: None,
        latest_message_content: None,
    }
}

#[test]
fn computes_fastest_and_longest_targets() {
    let run = sample_run();
    let targets = vec![
        target(
            "target-a",
            "queued",
            Some(2800),
            120,
            8,
            None,
            r#"{"display_name":"Window A","execution_mode":"openai_chat","provider":"openai","model_name":"gpt-5.4"}"#,
        ),
        target(
            "target-b",
            "done",
            Some(1200),
            240,
            16,
            Some("success"),
            r#"{"display_name":"Window B","execution_mode":"claude_cli","provider":"anthropic","model_name":"claude-sonnet"}"#,
        ),
    ];

    let summary = build_comparison_summary(run, targets);
    assert_eq!(summary.fastest_target_id.as_deref(), Some("target-b"));
    assert_eq!(summary.longest_target_id.as_deref(), Some("target-b"));
    assert!(summary.summary_text.contains("fastest"));
    assert_eq!(summary.targets[0].label, "OpenAI Chat / gpt-5.4");
    assert_eq!(summary.targets[1].label, "Claude CLI / claude-sonnet");
}

#[test]
fn handles_empty_targets() {
    let summary = build_comparison_summary(sample_run(), vec![]);
    assert!(summary.targets.is_empty());
    assert_eq!(summary.fastest_target_id, None);
    assert_eq!(summary.longest_target_id, None);
    assert!(summary.summary_text.contains("No targets"));
}
