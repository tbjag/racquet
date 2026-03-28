use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

pub async fn create_pool(database_url: &str) -> SqlitePool {
    tracing::info!(database_url = %database_url, "connecting to database");

    let options = SqliteConnectOptions::from_str(database_url)
        .expect("invalid database URL")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("failed to create database pool");

    tracing::info!(max_connections = 5, journal_mode = "WAL", "database pool ready");
    pool
}
