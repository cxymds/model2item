use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::evaluation_case::{
        CreateEvaluationCaseInput, EvaluationCaseRecord, UpdateEvaluationCaseInput,
    },
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

    pub async fn update_evaluation_case(
        &self,
        id: &str,
        input: UpdateEvaluationCaseInput,
    ) -> Result<EvaluationCaseRecord, AppError> {
        serde_json::from_str::<serde_json::Value>(&input.context_payload)
            .map_err(|error| AppError::InvalidJsonInput(error.to_string()))?;

        let notes = input.notes.unwrap_or_default();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE evaluation_cases
            SET title = ?, prompt = ?, context_payload = ?, notes = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&input.title)
        .bind(&input.prompt)
        .bind(&input.context_payload)
        .bind(notes)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get_evaluation_case(id).await
    }

    pub async fn delete_evaluation_case(&self, id: &str) -> Result<(), AppError> {
        let reference_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM comparison_runs WHERE evaluation_case_id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if reference_count > 0 {
            return Err(AppError::InvalidInput(
                "evaluation case is referenced by comparison runs".to_string(),
            ));
        }

        sqlx::query("DELETE FROM evaluation_cases WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
