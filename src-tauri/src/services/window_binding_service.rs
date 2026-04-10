use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::window_binding::{CreateWindowBindingInput, WindowBindingRecord},
};

#[derive(Clone)]
pub struct WindowBindingService {
    pool: SqlitePool,
}

impl WindowBindingService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_window_binding(
        &self,
        input: CreateWindowBindingInput,
    ) -> Result<WindowBindingRecord, AppError> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.iterm_session_id)
        .bind(&input.display_name)
        .bind(&input.profile_id)
        .execute(&self.pool)
        .await?;

        self.get_window_binding(&id).await
    }

    pub async fn list_window_bindings(&self) -> Result<Vec<WindowBindingRecord>, AppError> {
        let rows = sqlx::query_as::<_, WindowBindingRecord>(
            "SELECT * FROM window_bindings ORDER BY rowid DESC",
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

    pub async fn refresh_presence(
        &self,
        online_session_ids: &[String],
    ) -> Result<Vec<WindowBindingRecord>, AppError> {
        let now = Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;

        for session_id in online_session_ids {
            sqlx::query(
                r#"
                UPDATE window_bindings
                SET last_seen_at = ?
                WHERE iterm_session_id = ?
                "#,
            )
            .bind(&now)
            .bind(session_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.list_window_bindings().await
    }
}
