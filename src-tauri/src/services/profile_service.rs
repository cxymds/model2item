use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::profile::{CreateProfileInput, ModelProfileRecord, UpdateProfileInput},
    services::secret_store::{profile_secret_locator, SecretStore, SystemSecretStore},
};

#[derive(Clone)]
pub struct ProfileService {
    pool: SqlitePool,
    secret_store: Arc<dyn SecretStore>,
}

impl ProfileService {
    fn persist_secret(&self, locator: &str, secret: &str) -> Result<(), AppError> {
        self.secret_store.set_secret(locator, secret)?;
        let stored_secret = self.secret_store.get_secret(locator)?;
        if stored_secret == secret {
            Ok(())
        } else {
            Err(AppError::SecretStore(
                "saved API key could not be verified in secure storage".to_string(),
            ))
        }
    }

    fn is_missing_secret_error(error: &AppError) -> bool {
        match error {
            AppError::SecretStore(message) => {
                message.contains("No matching entry found in secure storage")
            }
            AppError::MissingDependency(message) => {
                message.contains("secret not found for locator")
            }
            _ => false,
        }
    }

    fn normalize_execution_mode(execution_mode: &str) -> &'static str {
        if execution_mode == "openai_chat" {
            "openai_chat"
        } else {
            "claude_cli"
        }
    }

    fn normalize_provider(provider: &str, execution_mode: &str) -> String {
        let trimmed_provider = provider.trim();
        if execution_mode == "openai_chat" {
            if trimmed_provider.is_empty() {
                "openai".to_string()
            } else {
                trimmed_provider.to_string()
            }
        } else if trimmed_provider.is_empty() {
            "anthropic".to_string()
        } else {
            trimmed_provider.to_string()
        }
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self::with_secret_store(pool, Arc::new(SystemSecretStore))
    }

    pub fn with_secret_store(pool: SqlitePool, secret_store: Arc<dyn SecretStore>) -> Self {
        Self { pool, secret_store }
    }

    pub async fn create_profile(
        &self,
        input: CreateProfileInput,
    ) -> Result<ModelProfileRecord, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let api_key_locator = profile_secret_locator(&id);
        let execution_mode = Self::normalize_execution_mode(&input.execution_mode);
        let provider = Self::normalize_provider(&input.provider, execution_mode);
        self.persist_secret(&api_key_locator, &input.api_key)?;

        sqlx::query(
            r#"
            INSERT INTO model_profiles
              (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&provider)
        .bind(execution_mode)
        .bind(&input.model_name)
        .bind(&input.base_url)
        .bind(&api_key_locator)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_profile(&id).await
    }

    pub async fn list_profiles(&self) -> Result<Vec<ModelProfileRecord>, AppError> {
        let rows = sqlx::query_as::<_, ModelProfileRecord>(
            "SELECT * FROM model_profiles WHERE enabled = 1 ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_profile(&self, id: &str) -> Result<ModelProfileRecord, AppError> {
        let row = sqlx::query_as::<_, ModelProfileRecord>(
            "SELECT * FROM model_profiles WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_profile(
        &self,
        id: &str,
        input: UpdateProfileInput,
    ) -> Result<ModelProfileRecord, AppError> {
        let now = Utc::now().to_rfc3339();
        let api_key_locator = profile_secret_locator(id);
        let execution_mode = Self::normalize_execution_mode(&input.execution_mode);
        let provider = Self::normalize_provider(&input.provider, execution_mode);
        if !input.api_key.trim().is_empty() {
            self.persist_secret(&api_key_locator, &input.api_key)?;
        }

        sqlx::query(
            r#"
            UPDATE model_profiles
            SET
              name = ?,
              provider = ?,
              execution_mode = ?,
              model_name = ?,
              base_url = ?,
              api_key_encrypted = ?,
              updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&input.name)
        .bind(&provider)
        .bind(execution_mode)
        .bind(&input.model_name)
        .bind(&input.base_url)
        .bind(&api_key_locator)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_profile(id).await
    }

    pub async fn get_profile_api_key(&self, id: &str) -> Result<Option<String>, AppError> {
        let profile = self.get_profile(id).await?;
        match self.secret_store.get_secret(&profile.api_key_encrypted) {
            Ok(secret) => Ok(Some(secret)),
            Err(error) if Self::is_missing_secret_error(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn delete_profile(&self, id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let active_binding_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(1)
            FROM window_bindings wb
            JOIN comparison_targets ct ON ct.window_binding_id = wb.id
            JOIN comparison_runs cr ON cr.id = ct.run_id
            WHERE wb.profile_id = ?
              AND cr.status IN ('queued', 'running')
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if active_binding_count > 0 {
            return Err(AppError::InvalidInput(
                "profile is still referenced by active window bindings".to_string(),
            ));
        }

        let binding_ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM window_bindings WHERE profile_id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let mut has_historical_bindings = false;

        let mut tx = self.pool.begin().await?;

        for binding_id in binding_ids {
            let historical_reference_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM comparison_targets WHERE window_binding_id = ?",
            )
            .bind(&binding_id)
            .fetch_one(&mut *tx)
            .await?;

            if historical_reference_count > 0 {
                has_historical_bindings = true;
                sqlx::query(
                    r#"
                    UPDATE window_bindings
                    SET enabled = 0, metadata_json = '{"deleted":true}'
                    WHERE id = ?
                    "#,
                )
                .bind(&binding_id)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query("DELETE FROM window_bindings WHERE id = ?")
                    .bind(&binding_id)
                    .execute(&mut *tx)
                .await?;
            }
        }

        if has_historical_bindings {
            sqlx::query(
                r#"
                UPDATE model_profiles
                SET enabled = 0, updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM model_profiles WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        let _ = self.secret_store.delete_secret(&profile_secret_locator(id));

        Ok(())
    }
}
