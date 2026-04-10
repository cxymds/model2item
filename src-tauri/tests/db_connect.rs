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
