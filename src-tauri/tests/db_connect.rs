use std::{fs, path::PathBuf};

#[tokio::test]
async fn creates_database_file_for_path_based_connection() -> Result<(), Box<dyn std::error::Error>>
{
    let base_dir = std::env::temp_dir().join(format!("iterm-mcp-tools-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&base_dir)?;

    let database_path = base_dir.join("workbench.db");

    let pool = iterm_mcp_tools_lib::db::connect_file(&database_path).await?;
    pool.close().await;

    assert!(database_path.exists());

    fs::remove_file(&database_path)?;
    fs::remove_dir(PathBuf::from(&base_dir))?;

    Ok(())
}

#[tokio::test]
async fn repairs_custom_provider_migration_checksum_mismatch_for_existing_database(
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = std::env::temp_dir().join(format!("iterm-mcp-tools-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&base_dir)?;

    let database_path = base_dir.join("workbench.db");

    let pool = iterm_mcp_tools_lib::db::connect_file(&database_path).await?;
    pool.close().await;

    let corruption_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&database_path)
                .foreign_keys(true),
        )
        .await?;

    sqlx::query("UPDATE _sqlx_migrations SET checksum = x'00' WHERE version = 4")
        .execute(&corruption_pool)
        .await?;
    corruption_pool.close().await;

    let repaired_pool = iterm_mcp_tools_lib::db::connect_file(&database_path).await?;
    let repaired_checksum =
        sqlx::query_scalar::<_, Vec<u8>>("SELECT checksum FROM _sqlx_migrations WHERE version = 4")
            .fetch_one(&repaired_pool)
            .await?;
    repaired_pool.close().await;

    assert_eq!(repaired_checksum.len(), 48);

    fs::remove_file(&database_path)?;
    fs::remove_dir(PathBuf::from(&base_dir))?;

    Ok(())
}
