use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;

pub async fn connect(cfg: &crate::config::Config) -> Result<SqlitePool, sqlx::Error> {
    let _ = tokio::fs::create_dir_all(&cfg.data_dir).await;
    let _ = tokio::fs::create_dir_all(&cfg.files_dir).await;

    let opts = SqliteConnectOptions::from_str(&format!(
        "sqlite://{}",
        cfg.db_path().to_string_lossy()
    ))?
    .create_if_missing(true)
    .journal_mode(SqliteJournalMode::Wal)
    .synchronous(SqliteSynchronous::Normal)
    .foreign_keys(true)
    .busy_timeout(Duration::from_secs(8))
    .pragma("journal_size_limit", "67108864")
    .pragma("cache_size", "-16000");

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .connect_with(opts)
        .await?;

    sqlx::query("PRAGMA busy_timeout = 8000;")
        .execute(&pool)
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn setting(pool: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
