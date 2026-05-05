use serde::{de::DeserializeOwned, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::{path::Path, str::FromStr, sync::OnceLock};

const DEFAULT_DB_PATH: &str = "go_service.db";
const DEFAULT_USER_ID: &str = "default";

static APP_DB: OnceLock<SqlitePool> = OnceLock::new();

pub fn db_path() -> String {
    std::env::var("GO_ALIAS_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string())
}

pub async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = db_path();
    if let Some(parent) = Path::new(&db_path).parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS app_documents (
            collection TEXT NOT NULL,
            user_id TEXT NOT NULL,
            key TEXT NOT NULL,
            value_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (collection, user_id, key)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let _ = APP_DB.set(pool);
    Ok(())
}

fn pool() -> Option<&'static SqlitePool> {
    APP_DB.get()
}

pub async fn get_json<T>(collection: &str, key: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    let pool = pool()?;
    let row = sqlx::query(
        "SELECT value_json FROM app_documents WHERE collection = ? AND user_id = ? AND key = ?",
    )
    .bind(collection)
    .bind(DEFAULT_USER_ID)
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()??;

    let raw: String = row.try_get("value_json").ok()?;
    serde_json::from_str(&raw).ok()
}

pub async fn put_json<T>(collection: &str, key: &str, value: &T) -> Result<(), sqlx::Error>
where
    T: Serialize,
{
    let Some(pool) = pool() else {
        return Ok(());
    };
    let raw = serde_json::to_string(value).map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO app_documents (collection, user_id, key, value_json, updated_at)
        VALUES (?, ?, ?, ?, unixepoch())
        ON CONFLICT(collection, user_id, key) DO UPDATE SET
            value_json = excluded.value_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(collection)
    .bind(DEFAULT_USER_ID)
    .bind(key)
    .bind(raw)
    .execute(pool)
    .await?;

    Ok(())
}

pub fn put_json_blocking<T>(collection: &str, key: &str, value: &T) -> Result<(), sqlx::Error>
where
    T: Serialize + Sync,
{
    let Some(pool) = pool() else {
        return Ok(());
    };
    let raw = serde_json::to_string(value).map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
    let collection = collection.to_string();
    let key = key.to_string();
    let pool = pool.clone();

    tokio::spawn(async move {
        if let Err(err) = sqlx::query(
            r#"
            INSERT INTO app_documents (collection, user_id, key, value_json, updated_at)
            VALUES (?, ?, ?, ?, unixepoch())
            ON CONFLICT(collection, user_id, key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(collection)
        .bind(DEFAULT_USER_ID)
        .bind(key)
        .bind(raw)
        .execute(&pool)
        .await
        {
            eprintln!("Failed to save document to app database: {err}");
        }
    });
    Ok(())
}

pub async fn migrate_json_file<T>(collection: &str, key: &str, path: &str)
where
    T: DeserializeOwned + Serialize,
{
    if get_json::<serde_json::Value>(collection, key).await.is_some() {
        return;
    }

    let Ok(data) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<T>(&data) else {
        return;
    };
    if let Err(err) = put_json(collection, key, &value).await {
        eprintln!("Failed to migrate {path} into app database: {err}");
    }
}
