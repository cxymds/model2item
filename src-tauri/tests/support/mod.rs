use sqlx::SqlitePool;

pub async fn create_test_pool() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    // A single pooled connection avoids per-connection split state for in-memory SQLite.
    let pool = iterm_mcp_tools_lib::db::connect_with_max_connections("sqlite::memory:", 1).await?;
    Ok(pool)
}

pub async fn table_names(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn foreign_keys_enabled(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let value = sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys;")
        .fetch_one(pool)
        .await?;
    Ok(value == 1)
}
