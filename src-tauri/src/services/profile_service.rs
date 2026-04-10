use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::profile::{CreateProfileInput, ModelProfileRecord},
    services::secret_store::{profile_secret_locator, SecretStore, SystemSecretStore},
};

#[derive(Clone)]
pub struct ProfileService {
    pool: SqlitePool,
    secret_store: Arc<dyn SecretStore>,
}

impl ProfileService {
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
        self.secret_store
            .set_secret(&api_key_locator, &input.api_key)?;

        sqlx::query(
            r#"
            INSERT INTO model_profiles
              (id, name, provider, model_name, base_url, api_key_encrypted, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.provider)
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
            "SELECT * FROM model_profiles ORDER BY created_at DESC",
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
}
