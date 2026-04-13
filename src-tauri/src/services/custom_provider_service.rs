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

        sqlx::query("DELETE FROM custom_providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let _ = self.secret_store.delete_secret(&provider.api_key_encrypted);

        Ok(())
    }
}
