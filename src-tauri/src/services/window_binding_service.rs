use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::window_binding::{
        CreateWindowBindingInput, UpdateWindowBindingInput, WindowBindingRecord,
    },
};

#[derive(Clone)]
pub struct WindowBindingService {
    pool: SqlitePool,
}

impl WindowBindingService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    const PROVIDER_BINDING_PROFILE_ID: &'static str = "__provider_binding_profile__";

    async fn profile_exists(&self, profile_id: &str) -> Result<bool, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM model_profiles WHERE id = ?",
        )
        .bind(profile_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    async fn ensure_provider_binding_profile(&self) -> Result<String, AppError> {
        let now = Utc::now().to_rfc3339();
        let profile_id = Self::PROVIDER_BINDING_PROFILE_ID.to_string();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO model_profiles (
              id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&profile_id)
        .bind("Provider Binding Placeholder")
        .bind("openai-compatible")
        .bind("claude_cli")
        .bind("provider-binding-placeholder")
        .bind("")
        .bind("secret://profile/__provider_binding_profile__")
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(profile_id)
    }

    async fn resolve_profile_id_for_binding(
        &self,
        profile_id: &str,
        custom_provider_id: Option<&str>,
    ) -> Result<String, AppError> {
        if custom_provider_id.is_none() {
            return Ok(profile_id.to_string());
        }

        if !profile_id.is_empty() && self.profile_exists(profile_id).await? {
            return Ok(profile_id.to_string());
        }

        self.ensure_provider_binding_profile().await
    }

    pub async fn create_window_binding(
        &self,
        input: CreateWindowBindingInput,
    ) -> Result<WindowBindingRecord, AppError> {
        let id = Uuid::new_v4().to_string();
        let resolved_profile_id = self
            .resolve_profile_id_for_binding(
                &input.profile_id,
                input.custom_provider_id.as_deref(),
            )
            .await?;

        sqlx::query(
            r#"
            INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id, custom_provider_id)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.iterm_session_id)
        .bind(&input.display_name)
        .bind(&resolved_profile_id)
        .bind(&input.custom_provider_id)
        .execute(&self.pool)
        .await?;

        self.get_window_binding(&id).await
    }

    pub async fn list_window_bindings(&self) -> Result<Vec<WindowBindingRecord>, AppError> {
        let rows = sqlx::query_as::<_, WindowBindingRecord>(
            "SELECT * FROM window_bindings WHERE enabled = 1 ORDER BY rowid DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_window_binding(&self, id: &str) -> Result<WindowBindingRecord, AppError> {
        let row = sqlx::query_as::<_, WindowBindingRecord>(
            "SELECT * FROM window_bindings WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_window_binding(
        &self,
        id: &str,
        input: UpdateWindowBindingInput,
    ) -> Result<WindowBindingRecord, AppError> {
        let resolved_profile_id = self
            .resolve_profile_id_for_binding(
                &input.profile_id,
                input.custom_provider_id.as_deref(),
            )
            .await?;

        let result = sqlx::query(
            r#"
            UPDATE window_bindings
            SET iterm_session_id = ?, display_name = ?, profile_id = ?, custom_provider_id = ?
            WHERE id = ?
            "#,
        )
        .bind(&input.iterm_session_id)
        .bind(&input.display_name)
        .bind(&resolved_profile_id)
        .bind(&input.custom_provider_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::MissingDependency(format!(
                "window binding {id} not found"
            )));
        }

        self.get_window_binding(id).await
    }

    pub async fn delete_window_binding(&self, id: &str) -> Result<(), AppError> {
        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM window_bindings WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if exists == 0 {
            return Err(AppError::MissingDependency(format!(
                "window binding {id} not found"
            )));
        }

        let active_reference_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(1)
            FROM comparison_targets ct
            JOIN comparison_runs cr ON cr.id = ct.run_id
            WHERE ct.window_binding_id = ?
              AND cr.status IN ('queued', 'running')
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if active_reference_count > 0 {
            return Err(AppError::InvalidInput(
                "window binding is referenced by comparison runs".to_string(),
            ));
        }

        let historical_reference_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM comparison_targets WHERE window_binding_id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if historical_reference_count > 0 {
            sqlx::query(
                r#"
                UPDATE window_bindings
                SET enabled = 0, metadata_json = '{"deleted":true}'
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("DELETE FROM window_bindings WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn sync_with_online_sessions(
        &self,
        online_session_ids: &[String],
    ) -> Result<Vec<WindowBindingRecord>, AppError> {
        let now = Utc::now().to_rfc3339();
        let bindings = self.list_window_bindings().await?;
        let online_session_ids = online_session_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let mut tx = self.pool.begin().await?;

        for binding in &bindings {
            if online_session_ids.contains(&binding.iterm_session_id) {
                sqlx::query(
                    r#"
                    UPDATE window_bindings
                    SET last_seen_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(&now)
                .bind(&binding.id)
                .execute(&mut *tx)
                .await?;
                continue;
            }

            let active_reference_count = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(1)
                FROM comparison_targets ct
                JOIN comparison_runs cr ON cr.id = ct.run_id
                WHERE ct.window_binding_id = ?
                  AND cr.status IN ('queued', 'running')
                "#,
            )
            .bind(&binding.id)
            .fetch_one(&mut *tx)
            .await?;

            if active_reference_count == 0 {
                let historical_reference_count = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(1) FROM comparison_targets WHERE window_binding_id = ?",
                )
                .bind(&binding.id)
                .fetch_one(&mut *tx)
                .await?;

                if historical_reference_count > 0 {
                    sqlx::query(
                        r#"
                        UPDATE window_bindings
                        SET enabled = 0, metadata_json = '{"deleted":true}'
                        WHERE id = ?
                        "#,
                    )
                    .bind(&binding.id)
                    .execute(&mut *tx)
                    .await?;
                } else {
                    sqlx::query("DELETE FROM window_bindings WHERE id = ?")
                        .bind(&binding.id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;
        self.list_window_bindings().await
    }

    pub async fn refresh_presence(
        &self,
        online_session_ids: &[String],
    ) -> Result<Vec<WindowBindingRecord>, AppError> {
        self.sync_with_online_sessions(online_session_ids).await
    }
}
