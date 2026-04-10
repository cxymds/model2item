use std::sync::Arc;

use sqlx::{FromRow, SqlitePool};

use crate::{
    error::AppError,
    models::window_binding::{CreateWindowBindingInput, WindowBindingRecord},
    services::{
        iterm_mcp_adapter::{
            build_binding_sync_command, classify_adapter_error, ItermMcpAdapter,
            PythonItermMcpAdapter,
        },
        secret_store::{SecretStore, SystemSecretStore},
        window_binding_service::WindowBindingService,
    },
};

#[derive(Debug, FromRow)]
struct BindingSyncRecord {
    iterm_session_id: String,
    profile_name: String,
    provider: String,
    model_name: String,
    base_url: String,
    api_key_locator: String,
}

#[derive(Clone)]
pub struct WindowBindingSyncService<A: ItermMcpAdapter> {
    pool: SqlitePool,
    adapter: A,
    secret_store: Arc<dyn SecretStore>,
}

impl WindowBindingSyncService<PythonItermMcpAdapter> {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            adapter: PythonItermMcpAdapter::default(),
            secret_store: Arc::new(SystemSecretStore),
        }
    }
}

impl<A: ItermMcpAdapter> WindowBindingSyncService<A> {
    pub fn with_dependencies(
        pool: SqlitePool,
        adapter: A,
        secret_store: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            pool,
            adapter,
            secret_store,
        }
    }

    pub async fn apply_binding(&self, binding_id: &str) -> Result<(), AppError> {
        let record = sqlx::query_as::<_, BindingSyncRecord>(
            r#"
            SELECT
              wb.iterm_session_id AS iterm_session_id,
              mp.name AS profile_name,
              mp.provider AS provider,
              mp.model_name AS model_name,
              mp.base_url AS base_url,
              mp.api_key_encrypted AS api_key_locator
            FROM window_bindings wb
            JOIN model_profiles mp ON mp.id = wb.profile_id
            WHERE wb.id = ?
            LIMIT 1
            "#,
        )
        .bind(binding_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::MissingDependency(format!("window binding {binding_id} not found"))
        })?;

        let api_key = self.secret_store.get_secret(&record.api_key_locator)?;
        let command = build_binding_sync_command(
            &record.profile_name,
            &record.provider,
            &record.model_name,
            &record.base_url,
            &api_key,
        );

        self.adapter
            .send_text(&record.iterm_session_id, &command)
            .await
            .map_err(classify_adapter_error)
    }
}

pub async fn create_window_binding_and_sync<A: ItermMcpAdapter>(
    pool: SqlitePool,
    adapter: A,
    secret_store: Arc<dyn SecretStore>,
    input: CreateWindowBindingInput,
) -> Result<WindowBindingRecord, AppError> {
    let binding_service = WindowBindingService::new(pool.clone());
    let binding = binding_service.create_window_binding(input).await?;
    let sync_service = WindowBindingSyncService::with_dependencies(pool, adapter, secret_store);

    if let Err(error) = sync_service.apply_binding(&binding.id).await {
        binding_service.delete_window_binding(&binding.id).await?;
        return Err(error);
    }

    Ok(binding)
}
