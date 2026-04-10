use serde::Deserialize;
use sqlx::SqlitePool;

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
    provider: Option<String>,
    model_name: Option<String>,
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
                if fastest_duration.is_none() || fastest_duration.is_some_and(|current| duration < current)
                {
                    fastest_duration = Some(duration);
                    fastest_target_id = Some(target.id.clone());
                }
            }
            if target.response_chars > longest_chars {
                longest_chars = target.response_chars;
                longest_target_id = Some(target.id.clone());
            }

            let parsed = serde_json::from_str::<ProfileSnapshot>(&target.profile_snapshot_json).ok();
            let display_name = parsed.as_ref().and_then(|item| item.display_name.clone());
            let provider = parsed.as_ref().and_then(|item| item.provider.clone());
            let model_name = parsed.as_ref().and_then(|item| item.model_name.clone());

            let label = match (provider.as_ref(), model_name.as_ref()) {
                (Some(p), Some(m)) => format!("{p} / {m}"),
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
            fastest_target_id
                .as_deref()
                .unwrap_or("n/a"),
            longest_target_id
                .as_deref()
                .unwrap_or("n/a"),
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
}
