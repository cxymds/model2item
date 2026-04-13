pub mod schema;

use std::{path::Path, str::FromStr};

use sqlx::{
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

static MIGRATOR: Migrator = sqlx::migrate!("./src/db/migrations");
const CUSTOM_PROVIDER_MIGRATION_VERSION: i64 = 4;

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    connect_with_max_connections(database_url, 5).await
}

pub async fn connect_file(database_path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    connect_file_with_max_connections(database_path, 5).await
}

pub async fn connect_with_max_connections(
    database_url: &str,
    max_connections: u32,
) -> Result<SqlitePool, sqlx::Error> {
    let connect_options = SqliteConnectOptions::from_str(database_url)?.foreign_keys(true);
    connect_with_options(connect_options, max_connections).await
}

pub async fn connect_file_with_max_connections(
    database_path: impl AsRef<Path>,
    max_connections: u32,
) -> Result<SqlitePool, sqlx::Error> {
    let connect_options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .foreign_keys(true);

    connect_with_options(connect_options, max_connections).await
}

async fn connect_with_options(
    connect_options: SqliteConnectOptions,
    max_connections: u32,
) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON;")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with(connect_options)
        .await?;

    run_migrations_with_compatibility(&pool).await?;
    Ok(pool)
}

async fn run_migrations_with_compatibility(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    match MIGRATOR.run(pool).await {
        Ok(()) => Ok(()),
        Err(MigrateError::VersionMismatch(version))
            if version == CUSTOM_PROVIDER_MIGRATION_VERSION =>
        {
            if repair_custom_provider_migration_checksum(pool).await? {
                MIGRATOR.run(pool).await.map_err(Into::into)
            } else {
                Err(MigrateError::VersionMismatch(version).into())
            }
        }
        Err(err) => Err(err.into()),
    }
}

async fn repair_custom_provider_migration_checksum(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    if !custom_provider_schema_is_present(pool).await? {
        return Ok(false);
    }

    let Some(migration) = MIGRATOR
        .iter()
        .find(|migration| migration.version == CUSTOM_PROVIDER_MIGRATION_VERSION)
    else {
        return Ok(false);
    };

    let rows_affected = sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = ? WHERE version = ? AND success = 1",
    )
    .bind(migration.checksum.as_ref())
    .bind(CUSTOM_PROVIDER_MIGRATION_VERSION)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected == 1)
}

async fn custom_provider_schema_is_present(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let has_custom_providers_table = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = 'custom_providers'",
    )
    .fetch_one(pool)
    .await?
        > 0;

    if !has_custom_providers_table {
        return Ok(false);
    }

    let has_custom_provider_column = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(1) FROM pragma_table_info('window_bindings') WHERE name = 'custom_provider_id'",
    )
    .fetch_one(pool)
    .await?
        > 0;

    Ok(has_custom_provider_column)
}
