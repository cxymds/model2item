use serde::Deserialize;
use sqlx::SqlitePool;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    error::AppError,
    models::comparison_run::{
        ComparisonRunRecord, ComparisonRunResponse, ComparisonSummaryResponse,
        ComparisonSummaryTargetResponse, ComparisonTargetRecord,
    },
    services::comparison_run_service::ComparisonRunService,
};

#[derive(Debug, Clone, Deserialize)]
struct ProfileSnapshot {
    display_name: Option<String>,
    execution_mode: Option<String>,
    provider: Option<String>,
    model_name: Option<String>,
}

fn execution_mode_label(execution_mode: &str) -> &'static str {
    match execution_mode {
        "openai_chat" => "OpenAI Chat",
        _ => "Claude CLI",
    }
}

pub fn build_comparison_summary(
    run: ComparisonRunRecord,
    targets: Vec<ComparisonTargetRecord>,
) -> ComparisonSummaryResponse {
    let mut fastest_target_id: Option<String> = None;
    let mut fastest_duration: Option<i64> = None;
    let mut longest_target_id: Option<String> = None;
    let mut longest_chars: i64 = -1;
    let mut queued_count = 0usize;

    let summary_targets = targets
        .into_iter()
        .map(|target| {
            if target.status == "queued" {
                queued_count += 1;
            }
            if let Some(duration) = target.duration_ms {
                if fastest_duration.is_none()
                    || fastest_duration.is_some_and(|current| duration < current)
                {
                    fastest_duration = Some(duration);
                    fastest_target_id = Some(target.id.clone());
                }
            }
            if target.response_chars > longest_chars {
                longest_chars = target.response_chars;
                longest_target_id = Some(target.id.clone());
            }

            let parsed =
                serde_json::from_str::<ProfileSnapshot>(&target.profile_snapshot_json).ok();
            let display_name = parsed.as_ref().and_then(|item| item.display_name.clone());
            let execution_mode = parsed.as_ref().and_then(|item| item.execution_mode.clone());
            let provider = parsed.as_ref().and_then(|item| item.provider.clone());
            let model_name = parsed.as_ref().and_then(|item| item.model_name.clone());

            let label = match (
                execution_mode.as_deref(),
                provider.as_ref(),
                model_name.as_ref(),
            ) {
                (Some(mode), _, Some(model)) => {
                    format!("{} / {model}", execution_mode_label(mode))
                }
                (Some(mode), Some(provider), None) => {
                    format!("{} / {provider}", execution_mode_label(mode))
                }
                (None, Some(provider), Some(model)) => format!("{provider} / {model}"),
                _ => display_name
                    .clone()
                    .unwrap_or_else(|| format!("Target {}", target.id)),
            };

            ComparisonSummaryTargetResponse {
                target_id: target.id,
                label,
                display_name,
                provider,
                model_name,
                status: target.status,
                success_status: target.success_status,
                duration_ms: target.duration_ms,
                response_chars: target.response_chars,
                response_lines: target.response_lines,
            }
        })
        .collect::<Vec<_>>();

    let summary_text = if summary_targets.is_empty() {
        "No targets found for this run.".to_string()
    } else {
        format!(
            "fastest={}; longest={}; queued={}",
            fastest_target_id.as_deref().unwrap_or("n/a"),
            longest_target_id.as_deref().unwrap_or("n/a"),
            queued_count
        )
    };

    ComparisonSummaryResponse {
        run: ComparisonRunResponse::from(run),
        targets: summary_targets,
        fastest_target_id,
        longest_target_id,
        queued_count,
        summary_text,
    }
}

fn build_markdown_report(
    summary: &ComparisonSummaryResponse,
    raw_targets: &[ComparisonTargetRecord],
) -> String {
    let mut lines = vec![
        format!("# {}", summary.run.title),
        String::new(),
        format!("- 运行 ID：{}", summary.run.id),
        format!("- 状态：{}", summary.run.status),
        format!("- 创建时间：{}", summary.run.created_at),
        format!("- 汇总：{}", summary.summary_text),
        String::new(),
        "## 指标概览".to_string(),
        String::new(),
        "| 目标 | 状态 | 耗时(ms) | 字符数 | 行数 |".to_string(),
        "| --- | --- | ---: | ---: | ---: |".to_string(),
    ];

    for target in &summary.targets {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            target.label,
            target.status,
            target
                .duration_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            target.response_chars,
            target.response_lines
        ));
    }

    lines.push(String::new());
    lines.push("## 最新输出".to_string());
    lines.push(String::new());

    for target in raw_targets {
        let label = summary
            .targets
            .iter()
            .find(|item| item.target_id == target.id)
            .map(|item| item.label.clone())
            .unwrap_or_else(|| format!("Target {}", target.id));
        let role_label = match target.latest_message_role.as_deref() {
            Some("assistant") => "模型输出",
            Some("user") => "用户输入",
            Some("system") => "系统消息",
            _ => "最新消息",
        };
        let content = target
            .latest_message_content
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("暂无输出。");

        lines.push(format!("### {}", label));
        lines.push(String::new());
        lines.push(format!("- 消息类型：{}", role_label));
        lines.push(format!("- 目标状态：{}", target.status));
        lines.push(String::new());
        lines.push("```text".to_string());
        lines.push(content.to_string());
        lines.push("```".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

#[derive(Clone)]
pub struct AnalysisService {
    run_service: ComparisonRunService,
}

impl AnalysisService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            run_service: ComparisonRunService::new(pool),
        }
    }

    pub async fn get_comparison_summary(
        &self,
        run_id: &str,
    ) -> Result<ComparisonSummaryResponse, AppError> {
        let run = self.run_service.get_comparison_run(run_id).await?;
        let targets = self.run_service.list_comparison_targets(run_id).await?;
        Ok(build_comparison_summary(run, targets))
    }

    pub async fn export_comparison_run_report(
        &self,
        run_id: &str,
        output_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        let summary = self.get_comparison_summary(run_id).await?;
        let targets = self.run_service.list_comparison_targets(run_id).await?;
        fs::create_dir_all(output_dir)?;

        let report_path = output_dir.join(format!("run-{run_id}-report.md"));
        fs::write(&report_path, build_markdown_report(&summary, &targets))?;

        Ok(report_path)
    }
}
