use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::evaluation_case::{CreateEvaluationCaseInput, EvaluationCaseRecord},
};

#[derive(Clone)]
pub struct EvaluationCaseService {
    pool: SqlitePool,
}

impl EvaluationCaseService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_evaluation_case(
        &self,
        input: CreateEvaluationCaseInput,
    ) -> Result<EvaluationCaseRecord, AppError> {
        serde_json::from_str::<serde_json::Value>(&input.context_payload)
            .map_err(|error| AppError::InvalidJsonInput(error.to_string()))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let notes = input.notes.unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO evaluation_cases
              (id, title, prompt, context_payload, notes, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.title)
        .bind(&input.prompt)
        .bind(&input.context_payload)
        .bind(notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_evaluation_case(&id).await
    }

    pub async fn list_evaluation_cases(&self) -> Result<Vec<EvaluationCaseRecord>, AppError> {
        let rows = sqlx::query_as::<_, EvaluationCaseRecord>(
            "SELECT * FROM evaluation_cases ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_evaluation_case(&self, id: &str) -> Result<EvaluationCaseRecord, AppError> {
        let row = sqlx::query_as::<_, EvaluationCaseRecord>(
            "SELECT * FROM evaluation_cases WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }
}
