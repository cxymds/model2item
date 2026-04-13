use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::custom_provider::{
        CreateCustomProviderInput, CustomProviderRecord, CustomProviderResponse,
        UpdateCustomProviderInput,
    },
    services::secret_store::{SecretStore, SystemSecretStore},
};

#[derive(Clone)]
pub struct CustomProviderService {
    pool: SqlitePool,
    secret_store: Arc<dyn SecretStore>,
}

impl CustomProviderService {
    fn custom_provider_secret_locator(provider_id: &str) -> String {
        format!("secret://custom-provider/{provider_id}")
    }

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

    pub fn new(pool: SqlitePool) -> Self {
        Self::with_secret_store(pool, Arc::new(SystemSecretStore))
    }

    pub fn with_secret_store(pool: SqlitePool, secret_store: Arc<dyn SecretStore>) -> Self {
        Self { pool, secret_store }
    }

    pub async fn create_custom_provider(
        &self,
        input: CreateCustomProviderInput,
    ) -> Result<CustomProviderResponse, AppError> {
        serde_json::from_str::<serde_json::Value>(&input.extra_params_json)
            .map_err(|error| AppError::InvalidJsonInput(error.to_string()))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let api_key_locator = Self::custom_provider_secret_locator(&id);
        self.persist_secret(&api_key_locator, &input.api_key)?;

        sqlx::query(
            r#"
            INSERT INTO custom_providers
              (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, extra_params_json, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.provider_key)
        .bind(&input.client_type)
        .bind(&input.base_url)
        .bind(&api_key_locator)
        .bind(&input.default_model)
        .bind(&input.extra_params_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_custom_provider(&id).await
    }

    async fn get_custom_provider_record(&self, id: &str) -> Result<CustomProviderRecord, AppError> {
        let row = sqlx::query_as::<_, CustomProviderRecord>(
            "SELECT * FROM custom_providers WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_backfilled_profile_id(&self, provider_id: &str) -> Result<Option<String>, AppError> {
        let Some(profile_id) = provider_id.strip_prefix("provider-") else {
            return Ok(None);
        };

        let profile_id = profile_id.to_string();
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM model_profiles WHERE id = ?",
        )
        .bind(&profile_id)
        .fetch_one(&self.pool)
        .await?;

        if count == 0 {
            Ok(None)
        } else {
            Ok(Some(profile_id))
        }
    }

    pub async fn list_custom_providers(&self) -> Result<Vec<CustomProviderResponse>, AppError> {
        let rows = sqlx::query_as::<_, CustomProviderRecord>(
            "SELECT * FROM custom_providers WHERE enabled = 1 ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_custom_provider(&self, id: &str) -> Result<CustomProviderResponse, AppError> {
        let row = self.get_custom_provider_record(id).await?;

        Ok(row.into())
    }

    pub async fn update_custom_provider(
        &self,
        id: &str,
        input: UpdateCustomProviderInput,
    ) -> Result<CustomProviderResponse, AppError> {
        serde_json::from_str::<serde_json::Value>(&input.extra_params_json)
            .map_err(|error| AppError::InvalidJsonInput(error.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let existing_provider = self.get_custom_provider_record(id).await?;
        let api_key_locator = existing_provider.api_key_encrypted;
        if !input.api_key.trim().is_empty() {
            self.persist_secret(&api_key_locator, &input.api_key)?;
        }

        sqlx::query(
            r#"
            UPDATE custom_providers
            SET
              name = ?,
              provider_key = ?,
              client_type = ?,
              base_url = ?,
              api_key_encrypted = ?,
              default_model = ?,
              extra_params_json = ?,
              updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&input.name)
        .bind(&input.provider_key)
        .bind(&input.client_type)
        .bind(&input.base_url)
        .bind(&api_key_locator)
        .bind(&input.default_model)
        .bind(&input.extra_params_json)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_custom_provider(id).await
    }

    pub async fn get_custom_provider_api_key(&self, id: &str) -> Result<Option<String>, AppError> {
        let provider = self.get_custom_provider_record(id).await?;

        match self.secret_store.get_secret(&provider.api_key_encrypted) {
            Ok(secret) => Ok(Some(secret)),
            Err(error) if Self::is_missing_secret_error(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn delete_custom_provider(&self, id: &str) -> Result<(), AppError> {
        let provider = self.get_custom_provider_record(id).await?;
        let legacy_profile_id = self.find_backfilled_profile_id(id).await?;

        let active_binding_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(1)
            FROM window_bindings wb
            JOIN comparison_targets ct ON ct.window_binding_id = wb.id
            JOIN comparison_runs cr ON cr.id = ct.run_id
            WHERE wb.custom_provider_id = ?
              AND cr.status IN ('queued', 'running')
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if active_binding_count > 0 {
            return Err(AppError::InvalidInput(
                "provider is still referenced by active window bindings".to_string(),
            ));
        }

        let binding_ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM window_bindings WHERE custom_provider_id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let placeholder_profile_id = self.ensure_provider_binding_placeholder_profile().await?;
        let mut tx = self.pool.begin().await?;

        for binding_id in binding_ids {
            let historical_reference_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM comparison_targets WHERE window_binding_id = ?",
            )
            .bind(&binding_id)
            .fetch_one(&mut *tx)
            .await?;

            if historical_reference_count > 0 {
                sqlx::query(
                    r#"
                    UPDATE window_bindings
                    SET
                      profile_id = ?,
                      custom_provider_id = NULL,
                      enabled = 0,
                      metadata_json = '{"deleted":true}'
                    WHERE id = ?
                    "#,
                )
                .bind(&placeholder_profile_id)
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

        sqlx::query("DELETE FROM custom_providers WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        if let Some(profile_id) = legacy_profile_id {
            let remaining_binding_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(1) FROM window_bindings WHERE profile_id = ?",
            )
            .bind(&profile_id)
            .fetch_one(&mut *tx)
            .await?;

            if remaining_binding_count == 0 {
                sqlx::query("DELETE FROM model_profiles WHERE id = ?")
                    .bind(&profile_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query(
                    r#"
                    UPDATE model_profiles
                    SET enabled = 0
                    WHERE id = ?
                    "#,
                )
                .bind(&profile_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        let _ = self.secret_store.delete_secret(&provider.api_key_encrypted);

        Ok(())
    }

    async fn ensure_provider_binding_placeholder_profile(&self) -> Result<String, AppError> {
        let now = Utc::now().to_rfc3339();
        let profile_id = "__provider_binding_profile__".to_string();

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
}
