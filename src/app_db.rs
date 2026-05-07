use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::{collections::HashMap, path::Path, str::FromStr, sync::OnceLock};

const DEFAULT_DB_PATH: &str = "go_service.db";
const DEFAULT_USER_ID: &str = "default";

static APP_DB: OnceLock<SqlitePool> = OnceLock::new();

pub fn db_path() -> String {
    std::env::var("GO_ALIAS_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string())
}

pub async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = db_path();
    if let Some(parent) = Path::new(&db_path)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }

    let options =
        SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    create_schema(&pool).await?;
    let _ = APP_DB.set(pool);
    migrate_legacy_documents().await;
    Ok(())
}

async fn create_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Legacy fallback table kept only for unknown document-style data.
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
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS app_data_sets (
            collection TEXT NOT NULL,
            key TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (collection, key)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS shortcuts (
            scope TEXT NOT NULL,
            shortcut_key TEXT NOT NULL,
            url TEXT NOT NULL,
            group_name TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (scope, shortcut_key)
        )
        "#,
    )
    .execute(pool)
    .await?;

    ensure_shortcuts_group_column(pool).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS shortcut_groups (
            scope TEXT NOT NULL,
            name TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (scope, name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sql_saved_queries (
            connection TEXT NOT NULL,
            name TEXT NOT NULL,
            folder TEXT,
            sql TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (connection, name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sql_query_folders (
            connection TEXT NOT NULL,
            name TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (connection, name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sql_run_history (
            id TEXT NOT NULL PRIMARY KEY,
            connection TEXT NOT NULL,
            tab_id TEXT NOT NULL DEFAULT '',
            sql TEXT NOT NULL,
            query_name TEXT NOT NULL DEFAULT '',
            query_folder TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            completed_at TEXT,
            row_count_text TEXT,
            html TEXT,
            error TEXT,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sql_run_history_connection_tab ON sql_run_history(connection, tab_id, updated_at DESC)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS saved_requests (
            folder TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            headers TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            auth_type TEXT,
            oauth_token_url TEXT,
            oauth_client_id TEXT,
            oauth_client_secret TEXT,
            oauth_scope TEXT,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (folder, name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS request_folders (
            name TEXT NOT NULL PRIMARY KEY,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS request_variable_sets (
            name TEXT NOT NULL PRIMARY KEY,
            is_active INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS request_variables (
            set_name TEXT NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (set_name, key),
            FOREIGN KEY (set_name) REFERENCES request_variable_sets(name) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS request_history (
            id TEXT NOT NULL PRIMARY KEY,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            name TEXT,
            method TEXT,
            url TEXT,
            final_url TEXT,
            status INTEGER,
            duration_ms INTEGER,
            size_kb TEXT,
            curl TEXT,
            request_json TEXT NOT NULL,
            response_json TEXT NOT NULL,
            entry_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS scratchpads (
            id TEXT NOT NULL PRIMARY KEY,
            position INTEGER NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            text TEXT NOT NULL DEFAULT '',
            updated_at_text TEXT,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn ensure_shortcuts_group_column(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let columns = sqlx::query("PRAGMA table_info(shortcuts)")
        .fetch_all(pool)
        .await?;
    let has_group_column = columns.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == "group_name")
            .unwrap_or(false)
    });

    if !has_group_column {
        sqlx::query("ALTER TABLE shortcuts ADD COLUMN group_name TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await?;
    }

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
    let value = match get_typed_json(pool, collection, key).await {
        Some(value) => value,
        None => get_legacy_json(pool, collection, key).await?,
    };
    serde_json::from_value(value).ok()
}

pub async fn put_json<T>(collection: &str, key: &str, value: &T) -> Result<(), sqlx::Error>
where
    T: Serialize,
{
    let Some(pool) = pool() else {
        return Ok(());
    };
    let value =
        serde_json::to_value(value).map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
    put_json_value(pool, collection, key, value).await
}

pub fn put_json_blocking<T>(collection: &str, key: &str, value: &T) -> Result<(), sqlx::Error>
where
    T: Serialize + Sync,
{
    let Some(pool) = pool() else {
        return Ok(());
    };
    let value =
        serde_json::to_value(value).map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
    let collection = collection.to_string();
    let key = key.to_string();
    let pool = pool.clone();

    tokio::spawn(async move {
        if let Err(err) = put_json_value(&pool, &collection, &key, value).await {
            eprintln!("Failed to save app data to database: {err}");
        }
    });
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SqlRunHistoryRecord {
    pub id: String,
    pub connection: String,
    pub tab_id: String,
    pub sql: String,
    pub query_name: String,
    pub query_folder: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub row_count_text: Option<String>,
    pub html: Option<String>,
    pub error: Option<String>,
}

pub async fn upsert_sql_run_history(record: &SqlRunHistoryRecord) -> Result<(), sqlx::Error> {
    let Some(pool) = pool() else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO sql_run_history (
            id, connection, tab_id, sql, query_name, query_folder, status, created_at,
            completed_at, row_count_text, html, error, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
        ON CONFLICT(id) DO UPDATE SET
            connection = excluded.connection,
            tab_id = excluded.tab_id,
            sql = excluded.sql,
            query_name = excluded.query_name,
            query_folder = excluded.query_folder,
            status = excluded.status,
            created_at = excluded.created_at,
            completed_at = excluded.completed_at,
            row_count_text = excluded.row_count_text,
            html = excluded.html,
            error = excluded.error,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&record.id)
    .bind(&record.connection)
    .bind(&record.tab_id)
    .bind(&record.sql)
    .bind(&record.query_name)
    .bind(&record.query_folder)
    .bind(&record.status)
    .bind(&record.created_at)
    .bind(&record.completed_at)
    .bind(&record.row_count_text)
    .bind(&record.html)
    .bind(&record.error)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_sql_run_history(
    connection: &str,
    tab_id: Option<&str>,
    limit: i64,
) -> Vec<SqlRunHistoryRecord> {
    let Some(pool) = pool() else {
        return Vec::new();
    };

    let rows = if let Some(tab_id) = tab_id {
        sqlx::query(
            r#"
            SELECT id, connection, tab_id, sql, query_name, query_folder, status, created_at,
                   completed_at, row_count_text, html, error
            FROM sql_run_history
            WHERE connection = ? AND tab_id = ?
            ORDER BY updated_at DESC, created_at DESC
            LIMIT ?
            "#,
        )
        .bind(connection)
        .bind(tab_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            r#"
            SELECT id, connection, tab_id, sql, query_name, query_folder, status, created_at,
                   completed_at, row_count_text, html, error
            FROM sql_run_history
            WHERE connection = ?
            ORDER BY updated_at DESC, created_at DESC
            LIMIT ?
            "#,
        )
        .bind(connection)
        .bind(limit)
        .fetch_all(pool)
        .await
    };

    rows.unwrap_or_default()
        .into_iter()
        .filter_map(sql_run_history_record_from_row)
        .collect()
}

pub async fn get_sql_run_history_by_id(id: &str) -> Option<SqlRunHistoryRecord> {
    let pool = pool()?;
    let row = sqlx::query(
        r#"
        SELECT id, connection, tab_id, sql, query_name, query_folder, status, created_at,
               completed_at, row_count_text, html, error
        FROM sql_run_history
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()??;

    sql_run_history_record_from_row(row)
}

pub async fn delete_sql_run_history(id: &str) -> Result<(), sqlx::Error> {
    let Some(pool) = pool() else {
        return Ok(());
    };

    sqlx::query("DELETE FROM sql_run_history WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_sql_run_history(
    connection: &str,
    tab_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let Some(pool) = pool() else {
        return Ok(());
    };

    if let Some(tab_id) = tab_id {
        sqlx::query("DELETE FROM sql_run_history WHERE connection = ? AND tab_id = ?")
            .bind(connection)
            .bind(tab_id)
            .execute(pool)
            .await?;
    } else {
        sqlx::query("DELETE FROM sql_run_history WHERE connection = ?")
            .bind(connection)
            .execute(pool)
            .await?;
    }
    Ok(())
}

fn sql_run_history_record_from_row(row: sqlx::sqlite::SqliteRow) -> Option<SqlRunHistoryRecord> {
    Some(SqlRunHistoryRecord {
        id: row.try_get("id").ok()?,
        connection: row.try_get("connection").ok()?,
        tab_id: row.try_get("tab_id").ok()?,
        sql: row.try_get("sql").ok()?,
        query_name: row.try_get("query_name").ok()?,
        query_folder: row.try_get("query_folder").ok()?,
        status: row.try_get("status").ok()?,
        created_at: row.try_get("created_at").ok()?,
        completed_at: row.try_get("completed_at").ok(),
        row_count_text: row.try_get("row_count_text").ok(),
        html: row.try_get("html").ok(),
        error: row.try_get("error").ok(),
    })
}

pub async fn migrate_json_file<T>(collection: &str, key: &str, path: &str)
where
    T: DeserializeOwned + Serialize,
{
    if get_json::<serde_json::Value>(collection, key)
        .await
        .is_some()
    {
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

async fn put_json_value(
    pool: &SqlitePool,
    collection: &str,
    key: &str,
    value: Value,
) -> Result<(), sqlx::Error> {
    if put_typed_json(pool, collection, key, &value).await? {
        delete_legacy_json(pool, collection, key).await?;
        return Ok(());
    }

    let raw =
        serde_json::to_string(&value).map_err(|err| sqlx::Error::Protocol(err.to_string()))?;
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

async fn get_legacy_json(pool: &SqlitePool, collection: &str, key: &str) -> Option<Value> {
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

async fn delete_legacy_json(
    pool: &SqlitePool,
    collection: &str,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM app_documents WHERE collection = ? AND user_id = ? AND key = ?")
        .bind(collection)
        .bind(DEFAULT_USER_ID)
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

async fn get_typed_json(pool: &SqlitePool, collection: &str, key: &str) -> Option<Value> {
    let value = match (collection, key) {
        ("shortcuts", "visible" | "hidden" | "work") => get_shortcuts(pool, key).await,
        ("sql", "queries") => get_sql_queries(pool).await,
        ("sql", "query_folders") => get_sql_query_folders(pool).await,
        ("requests", "saved") => get_saved_requests(pool).await,
        ("requests", "folders") => get_request_folders(pool).await,
        ("requests", "variables") => get_request_variables(pool).await,
        ("requests", "history") => get_request_history(pool).await,
        ("scratchpads", "pads") => get_scratchpads(pool).await,
        _ => None,
    };

    if value.is_some() {
        return value;
    }

    if !is_known_typed_store(collection, key) || !has_data_set(pool, collection, key).await {
        return None;
    }

    Some(empty_typed_value(collection, key))
}

async fn put_typed_json(
    pool: &SqlitePool,
    collection: &str,
    key: &str,
    value: &Value,
) -> Result<bool, sqlx::Error> {
    match (collection, key) {
        ("shortcuts", "visible" | "hidden" | "work") => {
            put_shortcuts(pool, key, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("sql", "queries") => {
            put_sql_queries(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("sql", "query_folders") => {
            put_sql_query_folders(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("requests", "saved") => {
            put_saved_requests(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("requests", "folders") => {
            put_request_folders(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("requests", "variables") => {
            put_request_variables(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("requests", "history") => {
            put_request_history(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        ("scratchpads", "pads") => {
            put_scratchpads(pool, value).await?;
            mark_data_set(pool, collection, key).await?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn is_known_typed_store(collection: &str, key: &str) -> bool {
    matches!(
        (collection, key),
        ("shortcuts", "visible" | "hidden" | "work")
            | ("sql", "queries")
            | ("sql", "query_folders")
            | ("requests", "saved")
            | ("requests", "folders")
            | ("requests", "variables")
            | ("requests", "history")
            | ("scratchpads", "pads")
    )
}

fn empty_typed_value(collection: &str, key: &str) -> Value {
    match (collection, key) {
        ("shortcuts", "visible" | "hidden" | "work") => Value::Object(Map::new()),
        ("requests", "variables") => json!({
            "active_set": "",
            "sets": [],
            "global": {},
        }),
        _ => Value::Array(Vec::new()),
    }
}

async fn has_data_set(pool: &SqlitePool, collection: &str, key: &str) -> bool {
    sqlx::query("SELECT 1 FROM app_data_sets WHERE collection = ? AND key = ?")
        .bind(collection)
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .is_some()
}

async fn mark_data_set(pool: &SqlitePool, collection: &str, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO app_data_sets (collection, key, updated_at)
        VALUES (?, ?, unixepoch())
        ON CONFLICT(collection, key) DO UPDATE SET updated_at = excluded.updated_at
        "#,
    )
    .bind(collection)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(())
}

async fn migrate_legacy_documents() {
    let Some(pool) = pool() else {
        return;
    };
    let Ok(rows) = sqlx::query("SELECT collection, key, value_json FROM app_documents")
        .fetch_all(pool)
        .await
    else {
        return;
    };

    for row in rows {
        let Ok(collection) = row.try_get::<String, _>("collection") else {
            continue;
        };
        let Ok(key) = row.try_get::<String, _>("key") else {
            continue;
        };
        let Ok(raw) = row.try_get::<String, _>("value_json") else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        match put_typed_json(pool, &collection, &key, &value).await {
            Ok(true) => {
                if let Err(err) = delete_legacy_json(pool, &collection, &key).await {
                    eprintln!(
                        "Failed to remove migrated legacy document {collection}/{key}: {err}"
                    );
                }
            }
            Ok(false) => {}
            Err(err) => eprintln!("Failed to migrate legacy document {collection}/{key}: {err}"),
        }
    }
}

async fn get_shortcuts(pool: &SqlitePool, scope: &str) -> Option<Value> {
    let rows = sqlx::query(
        "SELECT shortcut_key, url FROM shortcuts WHERE scope = ? ORDER BY shortcut_key",
    )
    .bind(scope)
    .fetch_all(pool)
    .await
    .ok()?;
    if rows.is_empty() {
        return None;
    }
    let mut map = Map::new();
    for row in rows {
        map.insert(
            row.try_get("shortcut_key").ok()?,
            Value::String(row.try_get("url").ok()?),
        );
    }
    Some(Value::Object(map))
}

pub async fn get_shortcut_groups(scope: &str) -> Vec<String> {
    let Some(pool) = pool() else {
        return Vec::new();
    };

    sqlx::query("SELECT name FROM shortcut_groups WHERE scope = ? ORDER BY lower(name)")
        .bind(scope)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| row.try_get::<String, _>("name").ok())
                .collect()
        })
        .unwrap_or_default()
}

pub async fn get_shortcut_group_map(scope: &str) -> HashMap<String, String> {
    let Some(pool) = pool() else {
        return HashMap::new();
    };

    sqlx::query(
        "SELECT shortcut_key, group_name FROM shortcuts WHERE scope = ? AND group_name <> ''",
    )
    .bind(scope)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .filter_map(|row| {
                Some((
                    row.try_get::<String, _>("shortcut_key").ok()?,
                    row.try_get::<String, _>("group_name").ok()?,
                ))
            })
            .collect()
    })
    .unwrap_or_default()
}

pub async fn create_shortcut_group(scope: &str, name: &str) -> Result<(), sqlx::Error> {
    let Some(pool) = pool() else {
        return Ok(());
    };
    let scope = normalize_shortcut_scope(scope);
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO shortcut_groups (scope, name, updated_at)
        VALUES (?, ?, unixepoch())
        ON CONFLICT(scope, name) DO UPDATE SET updated_at = excluded.updated_at
        "#,
    )
    .bind(scope)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_shortcut_group(
    scope: &str,
    shortcut_key: &str,
    group_name: &str,
) -> Result<(), sqlx::Error> {
    let Some(pool) = pool() else {
        return Ok(());
    };
    let scope = normalize_shortcut_scope(scope);
    let shortcut_key = shortcut_key.trim();
    let group_name = group_name.trim();
    if shortcut_key.is_empty() {
        return Ok(());
    }

    if !group_name.is_empty() {
        create_shortcut_group(scope, group_name).await?;
    }

    sqlx::query(
        r#"
        UPDATE shortcuts
        SET group_name = ?, updated_at = unixepoch()
        WHERE scope = ? AND shortcut_key = ?
        "#,
    )
    .bind(group_name)
    .bind(scope)
    .bind(shortcut_key)
    .execute(pool)
    .await?;

    Ok(())
}

fn normalize_shortcut_scope(scope: &str) -> &str {
    match scope {
        "hidden" | "hidden_global" => "hidden",
        "work" => "work",
        _ => "visible",
    }
}

async fn put_shortcuts(pool: &SqlitePool, scope: &str, value: &Value) -> Result<(), sqlx::Error> {
    let existing_groups: HashMap<String, String> = sqlx::query(
        "SELECT shortcut_key, group_name FROM shortcuts WHERE scope = ? AND group_name <> ''",
    )
    .bind(scope)
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter_map(|row| {
        Some((
            row.try_get::<String, _>("shortcut_key").ok()?,
            row.try_get::<String, _>("group_name").ok()?,
        ))
    })
    .collect();

    sqlx::query("DELETE FROM shortcuts WHERE scope = ?")
        .bind(scope)
        .execute(pool)
        .await?;

    if let Some(map) = value.as_object() {
        for (shortcut_key, url) in map {
            let group_name = existing_groups
                .get(shortcut_key)
                .map(String::as_str)
                .unwrap_or("");
            sqlx::query(
                r#"
                INSERT INTO shortcuts (scope, shortcut_key, url, group_name, updated_at)
                VALUES (?, ?, ?, ?, unixepoch())
                "#,
            )
            .bind(scope)
            .bind(shortcut_key)
            .bind(value_as_string(url))
            .bind(group_name)
            .execute(pool)
            .await?;

            if !group_name.is_empty() {
                sqlx::query(
                    r#"
                    INSERT INTO shortcut_groups (scope, name, updated_at)
                    VALUES (?, ?, unixepoch())
                    ON CONFLICT(scope, name) DO NOTHING
                    "#,
                )
                .bind(scope)
                .bind(group_name)
                .execute(pool)
                .await?;
            }
        }
    }
    Ok(())
}

async fn get_sql_queries(pool: &SqlitePool) -> Option<Value> {
    let rows = sqlx::query(
        "SELECT connection, name, folder, sql FROM sql_saved_queries ORDER BY connection, COALESCE(folder, ''), name",
    )
    .fetch_all(pool)
    .await
    .ok()?;
    if rows.is_empty() {
        return None;
    }
    let queries = rows
        .into_iter()
        .map(|row| {
            json!({
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "sql": row.try_get::<String, _>("sql").unwrap_or_default(),
                "folder": row.try_get::<Option<String>, _>("folder").ok().flatten(),
                "connection": row.try_get::<String, _>("connection").unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    Some(Value::Array(queries))
}

async fn put_sql_queries(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sql_saved_queries")
        .execute(pool)
        .await?;
    if let Some(items) = value.as_array() {
        for item in items {
            let connection = string_field(item, "connection");
            let name = string_field(item, "name");
            if connection.is_empty() || name.is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO sql_saved_queries (connection, name, folder, sql, updated_at)
                VALUES (?, ?, ?, ?, unixepoch())
                ON CONFLICT(connection, name) DO UPDATE SET
                    folder = excluded.folder,
                    sql = excluded.sql,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(connection)
            .bind(name)
            .bind(optional_string_field(item, "folder"))
            .bind(string_field(item, "sql"))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn get_sql_query_folders(pool: &SqlitePool) -> Option<Value> {
    let rows =
        sqlx::query("SELECT connection, name FROM sql_query_folders ORDER BY connection, name")
            .fetch_all(pool)
            .await
            .ok()?;
    if rows.is_empty() {
        return None;
    }
    let folders = rows
        .into_iter()
        .map(|row| {
            let connection = row.try_get::<String, _>("connection").unwrap_or_default();
            let name = row.try_get::<String, _>("name").unwrap_or_default();
            if connection.is_empty() {
                Value::String(name)
            } else {
                Value::String(format!("{connection}::{name}"))
            }
        })
        .collect::<Vec<_>>();
    Some(Value::Array(folders))
}

async fn put_sql_query_folders(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sql_query_folders")
        .execute(pool)
        .await?;
    if let Some(items) = value.as_array() {
        for item in items {
            let raw = value_as_string(item);
            let (connection, name) = raw
                .split_once("::")
                .map(|(connection, name)| (connection.to_string(), name.to_string()))
                .unwrap_or_else(|| (String::new(), raw));
            if name.trim().is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO sql_query_folders (connection, name, updated_at)
                VALUES (?, ?, unixepoch())
                ON CONFLICT(connection, name) DO UPDATE SET updated_at = excluded.updated_at
                "#,
            )
            .bind(connection)
            .bind(name)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn get_saved_requests(pool: &SqlitePool) -> Option<Value> {
    let rows = sqlx::query(
        r#"
        SELECT folder, name, method, url, headers, body, auth_type, oauth_token_url,
               oauth_client_id, oauth_client_secret, oauth_scope
        FROM saved_requests
        ORDER BY folder, name
        "#,
    )
    .fetch_all(pool)
    .await
    .ok()?;
    if rows.is_empty() {
        return None;
    }
    let requests = rows
        .into_iter()
        .map(|row| {
            let folder = row.try_get::<String, _>("folder").unwrap_or_default();
            json!({
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "method": row.try_get::<String, _>("method").unwrap_or_else(|_| "GET".to_string()),
                "url": row.try_get::<String, _>("url").unwrap_or_default(),
                "headers": row.try_get::<String, _>("headers").unwrap_or_default(),
                "body": row.try_get::<String, _>("body").unwrap_or_default(),
                "auth_type": row.try_get::<Option<String>, _>("auth_type").ok().flatten(),
                "oauth_token_url": row.try_get::<Option<String>, _>("oauth_token_url").ok().flatten(),
                "oauth_client_id": row.try_get::<Option<String>, _>("oauth_client_id").ok().flatten(),
                "oauth_client_secret": row.try_get::<Option<String>, _>("oauth_client_secret").ok().flatten(),
                "oauth_scope": row.try_get::<Option<String>, _>("oauth_scope").ok().flatten(),
                "folder": if folder.is_empty() { Value::Null } else { Value::String(folder) },
            })
        })
        .collect::<Vec<_>>();
    Some(Value::Array(requests))
}

async fn put_saved_requests(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM saved_requests")
        .execute(pool)
        .await?;
    if let Some(items) = value.as_array() {
        for item in items {
            let name = string_field(item, "name");
            if name.is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO saved_requests (
                    folder, name, method, url, headers, body, auth_type, oauth_token_url,
                    oauth_client_id, oauth_client_secret, oauth_scope, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
                ON CONFLICT(folder, name) DO UPDATE SET
                    method = excluded.method,
                    url = excluded.url,
                    headers = excluded.headers,
                    body = excluded.body,
                    auth_type = excluded.auth_type,
                    oauth_token_url = excluded.oauth_token_url,
                    oauth_client_id = excluded.oauth_client_id,
                    oauth_client_secret = excluded.oauth_client_secret,
                    oauth_scope = excluded.oauth_scope,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(optional_string_field(item, "folder").unwrap_or_default())
            .bind(name)
            .bind(nonempty_string_field(item, "method", "GET"))
            .bind(string_field(item, "url"))
            .bind(string_field(item, "headers"))
            .bind(string_field(item, "body"))
            .bind(optional_string_field(item, "auth_type"))
            .bind(optional_string_field(item, "oauth_token_url"))
            .bind(optional_string_field(item, "oauth_client_id"))
            .bind(optional_string_field(item, "oauth_client_secret"))
            .bind(optional_string_field(item, "oauth_scope"))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn get_request_folders(pool: &SqlitePool) -> Option<Value> {
    let rows = sqlx::query("SELECT name FROM request_folders ORDER BY name")
        .fetch_all(pool)
        .await
        .ok()?;
    if rows.is_empty() {
        return None;
    }
    Some(Value::Array(
        rows.into_iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .map(Value::String)
            .collect(),
    ))
}

async fn put_request_folders(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM request_folders")
        .execute(pool)
        .await?;
    if let Some(items) = value.as_array() {
        for item in items {
            let name = value_as_string(item);
            if name.trim().is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO request_folders (name, updated_at)
                VALUES (?, unixepoch())
                ON CONFLICT(name) DO UPDATE SET updated_at = excluded.updated_at
                "#,
            )
            .bind(name)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn get_request_variables(pool: &SqlitePool) -> Option<Value> {
    let set_rows = sqlx::query("SELECT name, is_active FROM request_variable_sets ORDER BY name")
        .fetch_all(pool)
        .await
        .ok()?;
    if set_rows.is_empty() {
        return None;
    }

    let mut active_set = String::new();
    let mut sets = Vec::new();
    for row in set_rows {
        let name = row.try_get::<String, _>("name").unwrap_or_default();
        if row.try_get::<i64, _>("is_active").unwrap_or(0) == 1 {
            active_set = name.clone();
        }
        let variable_rows =
            sqlx::query("SELECT key, value FROM request_variables WHERE set_name = ? ORDER BY key")
                .bind(&name)
                .fetch_all(pool)
                .await
                .unwrap_or_default();
        let mut values = Map::new();
        for variable_row in variable_rows {
            values.insert(
                variable_row.try_get::<String, _>("key").unwrap_or_default(),
                Value::String(
                    variable_row
                        .try_get::<String, _>("value")
                        .unwrap_or_default(),
                ),
            );
        }
        sets.push(json!({ "name": name, "values": values }));
    }

    if active_set.is_empty() {
        active_set = sets
            .first()
            .and_then(|set| set.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }

    Some(json!({
        "active_set": active_set,
        "sets": sets,
        "global": {},
    }))
}

async fn put_request_variables(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM request_variables")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM request_variable_sets")
        .execute(pool)
        .await?;

    let active_set = string_field(value, "active_set");
    if let Some(sets) = value.get("sets").and_then(Value::as_array) {
        for set in sets {
            let name = string_field(set, "name");
            if name.is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO request_variable_sets (name, is_active, updated_at)
                VALUES (?, ?, unixepoch())
                "#,
            )
            .bind(&name)
            .bind(if name == active_set { 1 } else { 0 })
            .execute(pool)
            .await?;

            if let Some(values) = set.get("values").and_then(Value::as_object) {
                for (key, variable_value) in values {
                    if key.trim().is_empty() {
                        continue;
                    }
                    sqlx::query(
                        r#"
                        INSERT INTO request_variables (set_name, key, value, updated_at)
                        VALUES (?, ?, ?, unixepoch())
                        "#,
                    )
                    .bind(&name)
                    .bind(key)
                    .bind(value_as_string(variable_value))
                    .execute(pool)
                    .await?;
                }
            }
        }
    }
    Ok(())
}

async fn get_request_history(pool: &SqlitePool) -> Option<Value> {
    let rows = sqlx::query("SELECT entry_json FROM request_history ORDER BY position")
        .fetch_all(pool)
        .await
        .ok()?;
    if rows.is_empty() {
        return None;
    }
    Some(Value::Array(
        rows.into_iter()
            .filter_map(|row| row.try_get::<String, _>("entry_json").ok())
            .filter_map(|raw| serde_json::from_str::<Value>(&raw).ok())
            .collect(),
    ))
}

async fn put_request_history(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM request_history")
        .execute(pool)
        .await?;
    if let Some(entries) = value.as_array() {
        for (position, entry) in entries.iter().enumerate() {
            let id = string_field(entry, "id");
            if id.is_empty() {
                continue;
            }
            let request = entry.get("request").cloned().unwrap_or_else(|| json!({}));
            let response = entry.get("response").cloned().unwrap_or_else(|| json!({}));
            sqlx::query(
                r#"
                INSERT INTO request_history (
                    id, position, created_at, name, method, url, final_url, status,
                    duration_ms, size_kb, curl, request_json, response_json, entry_json, updated_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, unixepoch())
                "#,
            )
            .bind(id)
            .bind(position as i64)
            .bind(nonempty_string_field(entry, "createdAt", ""))
            .bind(optional_string_field(&request, "name"))
            .bind(optional_string_field(&request, "method"))
            .bind(optional_string_field(&request, "url"))
            .bind(optional_string_field(&request, "finalUrl"))
            .bind(response.get("status").and_then(Value::as_i64))
            .bind(response.get("duration_ms").and_then(Value::as_i64))
            .bind(optional_string_field(&response, "size_kb"))
            .bind(optional_string_field(&response, "curl"))
            .bind(serde_json::to_string(&request).unwrap_or_else(|_| "{}".to_string()))
            .bind(serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()))
            .bind(serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string()))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

async fn get_scratchpads(pool: &SqlitePool) -> Option<Value> {
    let rows =
        sqlx::query("SELECT id, title, text, updated_at_text FROM scratchpads ORDER BY position")
            .fetch_all(pool)
            .await
            .ok()?;
    if rows.is_empty() {
        return None;
    }
    Some(Value::Array(
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.try_get::<String, _>("id").unwrap_or_default(),
                    "title": row.try_get::<String, _>("title").unwrap_or_else(|_| "Untitled".to_string()),
                    "text": row.try_get::<String, _>("text").unwrap_or_default(),
                    "updatedAt": row.try_get::<Option<String>, _>("updated_at_text").ok().flatten(),
                })
            })
            .collect(),
    ))
}

async fn put_scratchpads(pool: &SqlitePool, value: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM scratchpads").execute(pool).await?;
    if let Some(pads) = value.as_array() {
        for (position, pad) in pads.iter().enumerate() {
            let id = string_field(pad, "id");
            if id.is_empty() {
                continue;
            }
            sqlx::query(
                r#"
                INSERT INTO scratchpads (id, position, title, text, updated_at_text, updated_at)
                VALUES (?, ?, ?, ?, ?, unixepoch())
                ON CONFLICT(id) DO UPDATE SET
                    position = excluded.position,
                    title = excluded.title,
                    text = excluded.text,
                    updated_at_text = excluded.updated_at_text,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(id)
            .bind(position as i64)
            .bind(nonempty_string_field(pad, "title", "Untitled"))
            .bind(string_field(pad, "text"))
            .bind(optional_string_field(pad, "updatedAt"))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(value_as_string)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn nonempty_string_field(value: &Value, key: &str, fallback: &str) -> String {
    let value = string_field(value, key);
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn optional_string_field(value: &Value, key: &str) -> Option<String> {
    let value = string_field(value, key);
    if value.is_empty() { None } else { Some(value) }
}

fn value_as_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
