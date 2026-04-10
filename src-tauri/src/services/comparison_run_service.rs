use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, Row, SqlitePool};
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        comparison_run::{ComparisonRunRecord, ComparisonTargetRecord, CreateComparisonRunInput},
        evaluation_case::EvaluationCaseRecord,
    },
};

#[derive(Debug, Clone, Serialize)]
struct ProfileSnapshot {
    position: i64,
    display_name: String,
    profile_id: String,
    provider: String,
    model_name: String,
    base_url: String,
}

#[derive(Debug, FromRow)]
struct BindingProfileRow {
    binding_id: String,
    display_name: String,
    profile_id: String,
    provider: String,
    model_name: String,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct ComparisonTargetExecutionRecord {
    pub position: i64,
    pub target_id: String,
    pub run_id: String,
    pub window_binding_id: String,
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key_locator: String,
    pub system_prompt: String,
    pub extra_params_json: String,
}

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for ComparisonTargetExecutionRecord {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            position: row.try_get("position")?,
            target_id: row.try_get("target_id")?,
            run_id: row.try_get("run_id")?,
            window_binding_id: row.try_get("window_binding_id")?,
            iterm_session_id: row.try_get("iterm_session_id")?,
            display_name: row.try_get("display_name")?,
            profile_id: row.try_get("profile_id")?,
            provider: row.try_get("provider")?,
            model_name: row.try_get("model_name")?,
            base_url: row.try_get("base_url")?,
            api_key_locator: row.try_get("api_key_locator")?,
            system_prompt: row.try_get("system_prompt")?,
            extra_params_json: row.try_get("extra_params_json")?,
        })
    }
}

#[derive(Clone)]
pub struct ComparisonRunService {
    pool: SqlitePool,
}

impl ComparisonRunService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_comparison_run(
        &self,
        input: CreateComparisonRunInput,
    ) -> Result<ComparisonRunRecord, AppError> {
        let unique_target_ids = dedupe_target_ids(input.target_ids);
        if unique_target_ids.is_empty() {
            return Err(AppError::InvalidInput(
                "comparison run requires at least one target id".to_string(),
            ));
        }

        let mut tx = self.pool.begin().await?;

        let case = sqlx::query_as::<_, EvaluationCaseRecord>(
            "SELECT * FROM evaluation_cases WHERE id = ? LIMIT 1",
        )
        .bind(&input.evaluation_case_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AppError::MissingDependency(format!(
                "evaluation case {} not found",
                input.evaluation_case_id
            ))
        })?;

        let now = Utc::now().to_rfc3339();
        let run_id = Uuid::new_v4().to_string();
        let notes = input.notes.unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO comparison_runs
              (id, evaluation_case_id, title, status, prompt_snapshot, context_snapshot, created_at, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&run_id)
        .bind(&case.id)
        .bind(&input.title)
        .bind("queued")
        .bind(&case.prompt)
        .bind(&case.context_payload)
        .bind(&now)
        .bind(&notes)
        .execute(&mut *tx)
        .await?;

        for (position, binding_id) in unique_target_ids.iter().enumerate() {
            let binding = sqlx::query_as::<_, BindingProfileRow>(
                r#"
                SELECT
                  wb.id AS binding_id,
                  wb.display_name AS display_name,
                  mp.id AS profile_id,
                  mp.provider AS provider,
                  mp.model_name AS model_name,
                  mp.base_url AS base_url
                FROM window_bindings wb
                JOIN model_profiles mp ON mp.id = wb.profile_id
                WHERE wb.id = ?
                LIMIT 1
                "#,
            )
            .bind(binding_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AppError::MissingDependency(format!("window binding {} not found", binding_id))
            })?;

            let target_id = Uuid::new_v4().to_string();
            let snapshot = serde_json::to_string(&ProfileSnapshot {
                position: position as i64,
                display_name: binding.display_name,
                profile_id: binding.profile_id,
                provider: binding.provider,
                model_name: binding.model_name,
                base_url: binding.base_url,
            })?;

            sqlx::query(
                r#"
                INSERT INTO comparison_targets
                  (id, run_id, window_binding_id, profile_snapshot_json, status)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&target_id)
            .bind(&run_id)
            .bind(&binding.binding_id)
            .bind(&snapshot)
            .bind("queued")
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_comparison_run(&run_id).await
    }

    pub async fn get_comparison_run(&self, run_id: &str) -> Result<ComparisonRunRecord, AppError> {
        let row = sqlx::query_as::<_, ComparisonRunRecord>(
            "SELECT * FROM comparison_runs WHERE id = ? LIMIT 1",
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_comparison_runs(
        &self,
        limit: i64,
    ) -> Result<Vec<ComparisonRunRecord>, AppError> {
        let rows = sqlx::query_as::<_, ComparisonRunRecord>(
            r#"
            SELECT *
            FROM comparison_runs
            ORDER BY datetime(created_at) DESC, rowid DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_comparison_targets(
        &self,
        run_id: &str,
    ) -> Result<Vec<ComparisonTargetRecord>, AppError> {
        let rows = sqlx::query_as::<_, ComparisonTargetRecord>(
            r#"
            SELECT
              CAST(json_extract(profile_snapshot_json, '$.position') AS INTEGER) AS position,
              id,
              run_id,
              window_binding_id,
              profile_snapshot_json,
              status,
              sent_at,
              first_response_at,
              finished_at,
              duration_ms,
              response_chars,
              response_lines,
              success_status,
              error_category,
              error_detail,
              (
                SELECT role
                FROM messages m
                WHERE m.comparison_target_id = comparison_targets.id
                ORDER BY datetime(m.created_at) DESC, rowid DESC
                LIMIT 1
              ) AS latest_message_role,
              (
                SELECT content
                FROM messages m
                WHERE m.comparison_target_id = comparison_targets.id
                ORDER BY datetime(m.created_at) DESC, rowid DESC
                LIMIT 1
              ) AS latest_message_content
            FROM comparison_targets
            WHERE run_id = ?
            ORDER BY position ASC, id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_target_execution_records(
        &self,
        run_id: &str,
    ) -> Result<Vec<ComparisonTargetExecutionRecord>, AppError> {
        let rows = sqlx::query_as::<_, ComparisonTargetExecutionRecord>(
            r#"
            SELECT
              CAST(json_extract(ct.profile_snapshot_json, '$.position') AS INTEGER) AS position,
              ct.id AS target_id,
              ct.run_id AS run_id,
              ct.window_binding_id AS window_binding_id,
              wb.iterm_session_id AS iterm_session_id,
              wb.display_name AS display_name,
              mp.id AS profile_id,
              mp.provider AS provider,
              mp.model_name AS model_name,
              mp.base_url AS base_url,
              mp.api_key_encrypted AS api_key_locator,
              mp.system_prompt AS system_prompt,
              mp.extra_params_json AS extra_params_json
            FROM comparison_targets ct
            JOIN window_bindings wb ON wb.id = ct.window_binding_id
            JOIN model_profiles mp ON mp.id = wb.profile_id
            WHERE ct.run_id = ?
            ORDER BY position ASC, ct.id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_running_target_execution_records(
        &self,
        run_id: &str,
    ) -> Result<Vec<ComparisonTargetExecutionRecord>, AppError> {
        let rows = sqlx::query_as::<_, ComparisonTargetExecutionRecord>(
            r#"
            SELECT
              CAST(json_extract(ct.profile_snapshot_json, '$.position') AS INTEGER) AS position,
              ct.id AS target_id,
              ct.run_id AS run_id,
              ct.window_binding_id AS window_binding_id,
              wb.iterm_session_id AS iterm_session_id,
              wb.display_name AS display_name,
              mp.id AS profile_id,
              mp.provider AS provider,
              mp.model_name AS model_name,
              mp.base_url AS base_url,
              mp.api_key_encrypted AS api_key_locator,
              mp.system_prompt AS system_prompt,
              mp.extra_params_json AS extra_params_json
            FROM comparison_targets ct
            JOIN window_bindings wb ON wb.id = ct.window_binding_id
            JOIN model_profiles mp ON mp.id = wb.profile_id
            WHERE ct.run_id = ? AND ct.status = 'running'
            ORDER BY position ASC, ct.id ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn mark_run_started(&self, run_id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE comparison_runs
            SET status = 'running', started_at = COALESCE(started_at, ?)
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_target_running(&self, target_id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE comparison_targets
            SET status = 'running', sent_at = COALESCE(sent_at, ?)
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(target_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_target_message(
        &self,
        target_id: &str,
        role: &str,
        content: &str,
        message_type: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO messages
              (id, comparison_target_id, role, content, message_type, created_at, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(target_id)
        .bind(role)
        .bind(content)
        .bind(message_type)
        .bind(now)
        .bind("{}")
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_target_completed(
        &self,
        target_id: &str,
        output_text: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let response_chars = output_text.chars().count() as i64;
        let response_lines = output_text.lines().count() as i64;

        sqlx::query(
            r#"
            UPDATE comparison_targets
            SET
              status = 'done',
              first_response_at = COALESCE(first_response_at, ?),
              finished_at = ?,
              duration_ms = CAST((julianday(?) - julianday(COALESCE(sent_at, ?))) * 86400000 AS INTEGER),
              response_chars = ?,
              response_lines = ?,
              success_status = 'completed',
              error_category = NULL,
              error_detail = NULL
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .bind(response_chars)
        .bind(response_lines)
        .bind(target_id)
        .execute(&self.pool)
        .await?;

        self.store_target_message(target_id, "assistant", output_text, "response")
            .await
    }

    pub async fn record_target_interactive_output(
        &self,
        target_id: &str,
        output_text: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let response_chars = output_text.chars().count() as i64;
        let response_lines = output_text.lines().count() as i64;

        sqlx::query(
            r#"
            UPDATE comparison_targets
            SET
              first_response_at = COALESCE(first_response_at, ?),
              response_chars = ?,
              response_lines = ?,
              success_status = 'streaming',
              error_category = NULL,
              error_detail = NULL
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(response_chars)
        .bind(response_lines)
        .bind(target_id)
        .execute(&self.pool)
        .await?;

        self.store_target_message(target_id, "assistant", output_text, "response")
            .await
    }

    pub async fn mark_target_failed(
        &self,
        target_id: &str,
        error_category: &str,
        error_detail: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE comparison_targets
            SET
              status = 'failed',
              finished_at = ?,
              duration_ms = CAST((julianday(?) - julianday(COALESCE(sent_at, ?))) * 86400000 AS INTEGER),
              success_status = 'failed',
              error_category = ?,
              error_detail = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .bind(error_category)
        .bind(error_detail)
        .bind(target_id)
        .execute(&self.pool)
        .await?;

        self.store_target_message(target_id, "system", error_detail, "error")
            .await
    }

    pub async fn reconcile_closed_sessions(
        &self,
        online_session_ids: &[String],
    ) -> Result<(), AppError> {
        let online_session_ids = online_session_ids.iter().cloned().collect::<HashSet<_>>();
        let active_targets = sqlx::query_as::<_, ComparisonTargetExecutionRecord>(
            r#"
            SELECT
              CAST(json_extract(ct.profile_snapshot_json, '$.position') AS INTEGER) AS position,
              ct.id AS target_id,
              ct.run_id AS run_id,
              ct.window_binding_id AS window_binding_id,
              wb.iterm_session_id AS iterm_session_id,
              wb.display_name AS display_name,
              mp.id AS profile_id,
              mp.provider AS provider,
              mp.model_name AS model_name,
              mp.base_url AS base_url,
              mp.api_key_encrypted AS api_key_locator,
              mp.system_prompt AS system_prompt,
              mp.extra_params_json AS extra_params_json
            FROM comparison_targets ct
            JOIN comparison_runs cr ON cr.id = ct.run_id
            JOIN window_bindings wb ON wb.id = ct.window_binding_id
            JOIN model_profiles mp ON mp.id = wb.profile_id
            WHERE ct.status IN ('queued', 'running')
              AND cr.status IN ('queued', 'running')
            ORDER BY position ASC, ct.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut affected_run_ids = HashSet::new();
        for target in active_targets {
            if online_session_ids.contains(&target.iterm_session_id) {
                continue;
            }

            let detail = format!(
                "iTerm session {} is no longer available",
                target.iterm_session_id
            );
            self.mark_target_failed(&target.target_id, "session_closed", &detail)
                .await?;
            affected_run_ids.insert(target.run_id);
        }

        for run_id in affected_run_ids {
            self.finalize_run_if_terminal(&run_id).await?;
        }

        Ok(())
    }

    pub async fn finalize_run(&self, run_id: &str, status: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE comparison_runs
            SET status = ?, finished_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(now)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn finalize_run_if_terminal(&self, run_id: &str) -> Result<(), AppError> {
        let row = sqlx::query(
            r#"
            SELECT
              SUM(CASE WHEN status IN ('queued', 'running') THEN 1 ELSE 0 END) AS active_count,
              SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed_count
            FROM comparison_targets
            WHERE run_id = ?
            "#,
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        let active_count = row.try_get::<Option<i64>, _>("active_count")?.unwrap_or(0);
        if active_count > 0 {
            return Ok(());
        }

        let failed_count = row.try_get::<Option<i64>, _>("failed_count")?.unwrap_or(0);
        let run_status = if failed_count > 0 { "failed" } else { "done" };
        self.finalize_run(run_id, run_status).await
    }
}

fn dedupe_target_ids(target_ids: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for target_id in target_ids {
        if !unique.contains(&target_id) {
            unique.push(target_id);
        }
    }
    unique
}
