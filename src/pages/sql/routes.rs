use crate::app_db;
use crate::app_state::{AppState, SqlJob, Theme};
use crate::base_page::{render_base_page, static_asset};
use crate::pages::sql::{
    AddConnForm, DbConnection, SqlForm, encrypt_and_save, find_connection, load_and_decrypt,
    render_table,
};
use actix_web::{
    HttpResponse, Responder,
    cookie::{Cookie, time::Duration},
    get, post, web,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
// Added ValueRef to fix .is_null() error
use sqlx::{
    Column, Row, TypeInfo, ValueRef, postgres::PgPoolOptions, sqlite::SqlitePoolOptions,
    types::JsonValue,
};

const QUERIES_FILE: &str = "saved_queries.json";
const QUERY_FOLDERS_FILE: &str = "saved_query_folders.json";
const ACTIVE_SQL_CONNECTION_COOKIE: &str = "active_sql_connection";
const APP_DB_CONNECTION_NICKNAME: &str = "app_db";

#[derive(Serialize, Deserialize, Clone)]
struct SavedQuery {
    name: String,
    sql: String,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    connection: Option<String>,
}

#[derive(Deserialize)]
struct SaveQueryForm {
    query_name: String,
    sql: String,
    connection: String,
    folder: Option<String>,
}

#[derive(Deserialize)]
struct DeleteQueryForm {
    query_name: String,
    connection: String,
}

#[derive(Deserialize)]
struct RenameQueryForm {
    query_name: String,
    new_query_name: String,
    connection: String,
}

#[derive(Deserialize)]
struct CreateQueryFolderForm {
    folder_name: String,
    connection: String,
}

#[derive(Deserialize)]
struct DeleteQueryFolderForm {
    folder_name: String,
    connection: String,
}

#[derive(Deserialize)]
struct MoveQueryForm {
    query_name: String,
    connection: String,
    new_folder: Option<String>,
}

#[derive(Deserialize)]
struct MoveQueryFolderForm {
    folder_name: String,
    connection: String,
    new_parent: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct SavedQueryExport {
    version: u32,
    source_connection: String,
    queries: Vec<SavedQuery>,
    folders: Vec<String>,
}

#[derive(Deserialize)]
struct ImportQueriesForm {
    connection: String,
    payload: String,
    duplicate_mode: Option<String>,
}

#[derive(Deserialize)]
struct DeleteConnectionForm {
    nickname: String,
}

#[derive(Serialize)]
struct StartSqlJobResponse {
    job_id: String,
}

fn load_queries() -> Vec<SavedQuery> {
    fs::read_to_string(QUERIES_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_queries(queries: &[SavedQuery]) -> io::Result<()> {
    if let Err(err) = app_db::put_json_blocking("sql", "queries", &queries) {
        eprintln!("Failed to save SQL queries to app database: {err}");
    }
    let data = serde_json::to_string_pretty(queries)?;
    fs::write(QUERIES_FILE, data)
}

fn load_query_folders() -> Vec<String> {
    fs::read_to_string(QUERY_FOLDERS_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_query_folders(folders: &[String]) -> io::Result<()> {
    if let Err(err) = app_db::put_json_blocking("sql", "query_folders", &folders) {
        eprintln!("Failed to save SQL query folders to app database: {err}");
    }
    let data = serde_json::to_string_pretty(folders)?;
    fs::write(QUERY_FOLDERS_FILE, data)
}

fn query_matches_connection(query: &SavedQuery, connection: &str) -> bool {
    query.connection.as_deref() == Some(connection)
}

fn query_identity_matches(query: &SavedQuery, name: &str, connection: &str) -> bool {
    query.name == name && query_matches_connection(query, connection)
}

fn folder_matches_connection(folder: &str, connection: &str) -> bool {
    folder
        .split_once("::")
        .map(|(prefix, _)| prefix == connection)
        .unwrap_or(false)
}

fn stored_folder_name(folder: &str, connection: &str) -> String {
    format!("{connection}::{folder}")
}

fn display_folder_name(folder: &str) -> &str {
    folder
        .split_once("::")
        .map(|(_, name)| name)
        .unwrap_or(folder)
}

fn normalize_folder_path(folder: &str) -> String {
    folder
        .replace(" / ", "/")
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn folder_basename(folder: &str) -> String {
    normalize_folder_path(folder)
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}

fn add_folder_path(folders: &mut Vec<String>, folder: &str) {
    let folder = normalize_folder_path(folder);
    if folder.is_empty() {
        return;
    }
    let parts = folder.split('/').collect::<Vec<_>>();
    for index in 1..=parts.len() {
        let path = parts[..index].join("/");
        if !folders
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&path))
        {
            folders.push(path);
        }
    }
}

fn is_same_or_child_folder(folder: &str, parent: &str) -> bool {
    folder == parent
        || folder
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn move_folder_path(value: &str, old_folder: &str, new_folder: &str) -> String {
    if value == old_folder {
        return new_folder.to_string();
    }

    if let Some(rest) = value.strip_prefix(old_folder) {
        if rest.starts_with('/') {
            return format!("{new_folder}{rest}");
        }
    }

    value.to_string()
}

fn stored_folder_exists(folders: &[String], display_name: &str, connection: &str) -> bool {
    let display_name = normalize_folder_path(display_name);
    let stored = stored_folder_name(&display_name, connection);
    folders.iter().any(|folder| {
        folder.eq_ignore_ascii_case(&stored)
            || (folder_matches_connection(folder, connection)
                && display_folder_name(folder).eq_ignore_ascii_case(&display_name))
    })
}

fn unique_query_name(existing_queries: &[SavedQuery], base_name: &str, connection: &str) -> String {
    if !existing_queries
        .iter()
        .any(|query| query.name == base_name && query.connection.as_deref() == Some(connection))
    {
        return base_name.to_string();
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base_name} ({suffix})");
        if !existing_queries
            .iter()
            .any(|query| query.name == candidate && query.connection.as_deref() == Some(connection))
        {
            return candidate;
        }
        suffix += 1;
    }
}

fn app_db_connection() -> DbConnection {
    DbConnection {
        db_type: "sqlite".to_string(),
        host: app_db::db_path(),
        db_name: String::new(),
        user: String::new(),
        password: String::new(),
        nickname: APP_DB_CONNECTION_NICKNAME.to_string(),
    }
}

fn include_app_db_connection(conns: &mut Vec<DbConnection>) {
    if !conns
        .iter()
        .any(|conn| conn.nickname == APP_DB_CONNECTION_NICKNAME)
    {
        conns.insert(0, app_db_connection());
    }
}

fn render_connection_list(
    conns: &[DbConnection],
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    let conn_links = conns.iter()
        .map(|c| {
            // Differentiate display based on type
            let display_text = if c.db_type == "sqlite" {
                format!("{nick} (SQLite: {path})", nick = c.nickname, path = c.host)
            } else {
                format!("{nick} ({db}@{host})", nick = c.nickname, db = c.db_name, host = c.host)
            };

            format!(
                r#"
                <li class="saved-connection-item">
                    <a href="/sql/{nick}" class="saved-connection-link">{display_text}</a>
                    <form method="POST" action="/sql/connection/delete" class="delete-connection-form" onsubmit="return confirm('Delete saved connection {nick_js}?');">
                        <input type="hidden" name="nickname" value="{nick}">
                        <button type="submit" class="delete-connection-button">Delete</button>
                    </form>
                </li>
                "#,
                nick = htmlescape::encode_minimal(&c.nickname),
                nick_js = htmlescape::encode_attribute(&c.nickname),
                display_text = htmlescape::encode_minimal(&display_text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        r#"
    <div class="sql-connections-page">
        <div class="forms-container">
            <section class="sql-connections-panel">
                <h2>Add Connection</h2>
                <form method="POST" action="/sql/add" class="connection-form">
                  
                  <label for="db_type" style="display:block; margin: calc(var(--element-margin) / 2) var(--element-margin);">Database Type:</label>
                  <select name="db_type" id="db_type" onchange="toggleFields()" style="margin: calc(var(--element-margin) / 2) var(--element-margin); width:100%; padding:10px;">
                      <option value="postgres">Postgres</option>
                      <option value="sqlite">SQLite (Existing File)</option>
                  </select>
    
                  <input name="nickname" placeholder="Nickname (e.g., prod_db)" required>
                  
                  <!-- Shared field: Host for PG, File Path for SQLite -->
                  <input name="host" id="host_input" placeholder="Host (e.g., localhost:5432)" required>
                  
                  <div id="pg_fields">
                      <input name="db_name" placeholder="Database Name">
                      <input name="user" placeholder="User">
                      <input name="password" type="password" placeholder="Password">
                  </div>
                  
                  <button type="submit" class="save-connection-submit">Save Connection</button>
                </form>
            </section>

            <section class="sql-connections-panel">
                <h2>Create New SQLite DB</h2>
                <form method="POST" action="/sql/add" class="connection-form" onsubmit="prepareCreate(event)">
                    <input type="hidden" name="db_type" value="sqlite">
                    <input type="hidden" name="db_name" value="">
                    <input type="hidden" name="user" value="">
                    <input type="hidden" name="password" value="">
                    <!-- These will be populated by JS -->
                    <input type="hidden" name="host" id="create_host">
                    <input type="hidden" name="nickname" id="create_nick">

                    <label style="display:block; margin: calc(var(--element-margin) / 2) var(--element-margin);">New Filename:</label>
                    <input id="new_filename" placeholder="e.g., my_new_project" required>
                    
                    <button type="submit" class="create-sqlite-submit" style="background-color: var(--link-color); color: var(--primary-bg); font-weight: bold;">Create & Save</button>
                    <p style="font-size:0.85em; opacity:0.8; margin: calc(var(--element-margin) / 2) var(--element-margin); line-height: 1.4;">
                        This will register a new SQLite database file. 
                        The file will be created automatically when you first open it.
                    </p>
                </form>
            </section>
        </div>
        
        <section class="sql-connections-panel saved-connections-list">
            <h2>Saved Connections</h2>
            <ul>{conn_links}</ul>
        </section>
    </div>
    <link rel="stylesheet" href="{sql_connections_css}">
    <script src="{sql_connections_js}" defer></script>
    "#,
        conn_links = conn_links,
        sql_connections_css = static_asset("sql_connections.css"),
        sql_connections_js = static_asset("sql_connections.js")
    );

    render_base_page("SQL Connections", &content, current_theme, saved_themes)
}

#[get("/sql")]
pub async fn sql_get(state: web::Data<Arc<AppState>>) -> impl Responder {
    let conns = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        include_app_db_connection(conns_opt.as_mut().unwrap());
        conns_opt.clone().unwrap()
    };

    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_connection_list(
            &conns,
            &current_theme,
            &saved_themes,
        ))
}

#[post("/sql/disconnect")]
pub async fn sql_disconnect() -> impl Responder {
    let expired_cookie = Cookie::build(ACTIVE_SQL_CONNECTION_COOKIE, "")
        .path("/sql")
        .max_age(Duration::seconds(0))
        .finish();

    HttpResponse::NoContent().cookie(expired_cookie).finish()
}

#[post("/sql/disconnect/{nickname}")]
pub async fn sql_disconnect_connection(
    path: web::Path<String>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let nickname = path.into_inner();
    let conn_opt = {
        let mut conns_opt = state
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        include_app_db_connection(conns_opt.as_mut().unwrap());
        conns_opt
            .as_ref()
            .and_then(|conns| conns.iter().find(|conn| conn.nickname == nickname).cloned())
    };

    if let Some(conn) = conn_opt {
        if conn.db_type == "sqlite" {
            let dsn = format!("sqlite:{}?mode=rwc", conn.host);
            let pool = {
                let mut pools = state
                    .sqlite_pools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                pools.remove(&dsn)
            };
            if let Some(pool) = pool {
                pool.close().await;
            }
        } else {
            let dsn = format!(
                "postgres://{}:{}@{}/{}",
                conn.user, conn.password, conn.host, conn.db_name
            );
            let pool = {
                let mut pools = state
                    .pg_pools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                pools.remove(&dsn)
            };
            if let Some(pool) = pool {
                pool.close().await;
            }
        }
    }

    let expired_cookie = Cookie::build(ACTIVE_SQL_CONNECTION_COOKIE, "")
        .path("/sql")
        .max_age(Duration::seconds(0))
        .finish();

    HttpResponse::NoContent().cookie(expired_cookie).finish()
}

#[post("/sql/add")]
pub async fn sql_add(
    form: web::Form<AddConnForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let new_conn = DbConnection {
        db_type: form
            .db_type
            .clone()
            .unwrap_or_else(|| "postgres".to_string()),
        host: form.host.clone(),
        db_name: form.db_name.clone(),
        user: form.user.clone(),
        password: form.password.clone(),
        nickname: form.nickname.clone(),
    };
    {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        let conns = conns_opt.as_mut().unwrap();

        if let Some(idx) = conns.iter().position(|c| c.nickname == new_conn.nickname) {
            conns[idx] = new_conn;
        } else {
            conns.push(new_conn);
        }
        if let Err(e) = encrypt_and_save(conns) {
            eprintln!("Failed to save encrypted connections: {e}");
        }
    }
    HttpResponse::Found()
        .append_header(("Location", "/sql"))
        .finish()
}

#[post("/sql/save")]
pub async fn sql_save(form: web::Form<SaveQueryForm>) -> impl Responder {
    let mut queries = load_queries();
    let folder = form
        .folder
        .as_ref()
        .map(|value| normalize_folder_path(value))
        .filter(|value| !value.is_empty());

    if let Some(idx) = queries.iter().position(|q| {
        q.name == form.query_name && q.connection.as_deref() == Some(&form.connection)
    }) {
        queries[idx].sql = form.sql.clone();
        queries[idx].folder = folder;
        queries[idx].connection = Some(form.connection.clone());
    } else {
        queries.push(SavedQuery {
            name: form.query_name.clone(),
            sql: form.sql.clone(),
            folder,
            connection: Some(form.connection.clone()),
        });
    }

    if let Err(e) = save_queries(&queries) {
        eprintln!("Failed to save queries: {e}");
    }

    // Redirect back to the specific connection view
    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

#[post("/sql/folder")]
pub async fn sql_create_folder(form: web::Form<CreateQueryFolderForm>) -> impl Responder {
    let folder_name = normalize_folder_path(&form.folder_name);
    if !folder_name.is_empty() {
        let mut folders = load_query_folders();
        if !stored_folder_exists(&folders, &folder_name, &form.connection) {
            folders.push(stored_folder_name(&folder_name, &form.connection));
            folders.sort_by_key(|folder| folder.to_lowercase());
            if let Err(e) = save_query_folders(&folders) {
                eprintln!("Failed to save query folders: {e}");
            }
        }
    }

    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

#[post("/sql/folder/delete")]
pub async fn sql_delete_folder(form: web::Form<DeleteQueryFolderForm>) -> impl Responder {
    let folder_name = normalize_folder_path(&form.folder_name);
    if !folder_name.is_empty() {
        let mut folders = load_query_folders();
        folders.retain(|folder| {
            !folder_matches_connection(folder, &form.connection)
                || !is_same_or_child_folder(
                    &normalize_folder_path(display_folder_name(folder)),
                    &folder_name,
                )
        });
        if let Err(err) = save_query_folders(&folders) {
            eprintln!("Failed to delete SQL folder: {err}");
        }

        let mut queries = load_queries();
        queries.retain(|query| {
            if !query_matches_connection(query, &form.connection) {
                return true;
            }
            let folder = query
                .folder
                .as_deref()
                .map(normalize_folder_path)
                .unwrap_or_default();
            !is_same_or_child_folder(&folder, &folder_name)
        });
        if let Err(err) = save_queries(&queries) {
            eprintln!("Failed to delete SQL queries in folder: {err}");
        }
    }

    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

#[post("/sql/query/move")]
pub async fn sql_move_query(form: web::Form<MoveQueryForm>) -> impl Responder {
    let new_folder = form
        .new_folder
        .as_deref()
        .map(normalize_folder_path)
        .filter(|folder| !folder.is_empty());

    let mut queries = load_queries();
    if let Some(query) = queries
        .iter_mut()
        .find(|query| query_identity_matches(query, &form.query_name, &form.connection))
    {
        query.folder = new_folder.clone();
        query.connection = Some(form.connection.clone());
        if let Err(err) = save_queries(&queries) {
            eprintln!("Failed to move SQL query: {err}");
        }
    }

    if let Some(folder) = new_folder {
        let mut folders = load_query_folders();
        if !stored_folder_exists(&folders, &folder, &form.connection) {
            folders.push(stored_folder_name(&folder, &form.connection));
            folders.sort_by_key(|folder| folder.to_lowercase());
            if let Err(err) = save_query_folders(&folders) {
                eprintln!("Failed to save SQL folder after query move: {err}");
            }
        }
    }

    HttpResponse::NoContent().finish()
}

#[post("/sql/folder/move")]
pub async fn sql_move_folder(form: web::Form<MoveQueryFolderForm>) -> impl Responder {
    let old_folder = normalize_folder_path(&form.folder_name);
    let new_parent = form
        .new_parent
        .as_deref()
        .map(normalize_folder_path)
        .unwrap_or_default();

    if old_folder.is_empty() || is_same_or_child_folder(&new_parent, &old_folder) {
        return HttpResponse::BadRequest().body("Invalid folder move");
    }

    let basename = folder_basename(&old_folder);
    if basename.is_empty() {
        return HttpResponse::BadRequest().body("Invalid folder name");
    }

    let new_folder = if new_parent.is_empty() {
        basename
    } else {
        format!("{new_parent}/{basename}")
    };

    if old_folder == new_folder {
        return HttpResponse::NoContent().finish();
    }

    let mut folders = load_query_folders();
    let mut changed = false;
    for folder in folders.iter_mut() {
        if !folder_matches_connection(folder, &form.connection) {
            continue;
        }
        let display = normalize_folder_path(display_folder_name(folder));
        if is_same_or_child_folder(&display, &old_folder) {
            *folder = stored_folder_name(
                &move_folder_path(&display, &old_folder, &new_folder),
                &form.connection,
            );
            changed = true;
        }
    }
    if !folders.iter().any(|folder| {
        folder_matches_connection(folder, &form.connection)
            && display_folder_name(folder).eq_ignore_ascii_case(&new_folder)
    }) {
        folders.push(stored_folder_name(&new_folder, &form.connection));
        changed = true;
    }
    if changed {
        folders.sort_by_key(|folder| folder.to_lowercase());
        if let Err(err) = save_query_folders(&folders) {
            eprintln!("Failed to move SQL folder: {err}");
        }
    }

    let mut queries = load_queries();
    let mut queries_changed = false;
    for query in queries
        .iter_mut()
        .filter(|query| query_matches_connection(query, &form.connection))
    {
        let Some(folder) = query.folder.as_ref() else {
            continue;
        };
        let folder = normalize_folder_path(folder);
        if is_same_or_child_folder(&folder, &old_folder) {
            query.folder = Some(move_folder_path(&folder, &old_folder, &new_folder));
            queries_changed = true;
        }
    }
    if queries_changed {
        if let Err(err) = save_queries(&queries) {
            eprintln!("Failed to move SQL queries with folder: {err}");
        }
    }

    HttpResponse::NoContent().finish()
}

#[post("/sql/delete")]
pub async fn sql_delete(form: web::Form<DeleteQueryForm>) -> impl Responder {
    let mut queries = load_queries();
    if let Some(pos) = queries
        .iter()
        .position(|query| query_identity_matches(query, &form.query_name, &form.connection))
    {
        queries.remove(pos);
        if let Err(e) = save_queries(&queries) {
            eprintln!("Failed to delete query: {e}");
        }
    }

    // Redirect back to the specific connection view
    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

#[post("/sql/rename")]
pub async fn sql_rename(form: web::Form<RenameQueryForm>) -> impl Responder {
    let new_name = form.new_query_name.trim();
    if !new_name.is_empty() {
        let mut queries = load_queries();
        let duplicate_exists = queries.iter().any(|query| {
            query.name == new_name && query.connection.as_deref() == Some(&form.connection)
        });
        if !duplicate_exists {
            if let Some(query) = queries
                .iter_mut()
                .find(|query| query_identity_matches(query, &form.query_name, &form.connection))
            {
                query.name = new_name.to_string();
                query.connection = Some(form.connection.clone());
                if let Err(e) = save_queries(&queries) {
                    eprintln!("Failed to rename query: {e}");
                }
            }
        }
    }

    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found()
        .append_header(("Location", location))
        .finish()
}

#[get("/sql/{connection}/queries/export")]
pub async fn sql_export_queries(path: web::Path<String>) -> impl Responder {
    let connection = path.into_inner();
    let queries = load_queries()
        .into_iter()
        .filter(|query| query_matches_connection(query, &connection))
        .map(|mut query| {
            query.connection = None;
            query
        })
        .collect::<Vec<_>>();
    let folders = load_query_folders()
        .into_iter()
        .filter(|folder| folder_matches_connection(folder, &connection))
        .map(|folder| display_folder_name(&folder).to_string())
        .chain(
            queries
                .iter()
                .filter_map(|query| query.folder.as_ref())
                .map(|folder| folder.trim().to_string())
                .filter(|folder| !folder.is_empty()),
        )
        .fold(Vec::<String>::new(), |mut folders, folder| {
            if !folders
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&folder))
            {
                folders.push(folder);
            }
            folders
        });
    let export = SavedQueryExport {
        version: 1,
        source_connection: connection.clone(),
        queries,
        folders,
    };
    let body = serde_json::to_string_pretty(&export).unwrap_or_else(|_| "{}".to_string());
    HttpResponse::Ok()
        .content_type("application/json")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{connection}-saved-queries.json\""),
        ))
        .body(body)
}

#[post("/sql/queries/import")]
pub async fn sql_import_queries(form: web::Form<ImportQueriesForm>) -> impl Responder {
    let parsed = serde_json::from_str::<SavedQueryExport>(&form.payload).or_else(|_| {
        serde_json::from_str::<Vec<SavedQuery>>(&form.payload).map(|queries| SavedQueryExport {
            version: 1,
            source_connection: String::new(),
            queries,
            folders: Vec::new(),
        })
    });

    match parsed {
        Ok(export) => {
            let duplicate_mode = form.duplicate_mode.as_deref().unwrap_or("rename");
            let mut existing_queries = load_queries();
            for mut query in export.queries {
                let base_name = query.name.trim();
                if base_name.is_empty() {
                    continue;
                }
                query.name = base_name.to_string();
                query.connection = Some(form.connection.clone());
                if let Some(folder) = query
                    .folder
                    .as_ref()
                    .map(|folder| folder.trim())
                    .filter(|folder| !folder.is_empty())
                {
                    query.folder = Some(folder.to_string());
                }

                if let Some(idx) = existing_queries.iter().position(|existing| {
                    existing.name == query.name
                        && existing.connection.as_deref() == Some(&form.connection)
                }) {
                    if duplicate_mode == "overwrite" {
                        existing_queries[idx] = query;
                    } else {
                        query.name =
                            unique_query_name(&existing_queries, &query.name, &form.connection);
                        existing_queries.push(query);
                    }
                } else {
                    existing_queries.push(query);
                }
            }

            let mut folders = load_query_folders();
            for folder in export.folders {
                let folder = folder.trim();
                if folder.is_empty() {
                    continue;
                }
                let stored = stored_folder_name(folder, &form.connection);
                if !folders
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&stored))
                {
                    folders.push(stored);
                }
            }
            for query in &existing_queries {
                if query.connection.as_deref() != Some(&form.connection) {
                    continue;
                }
                if let Some(folder) = query
                    .folder
                    .as_ref()
                    .map(|folder| folder.trim())
                    .filter(|folder| !folder.is_empty())
                {
                    if !stored_folder_exists(&folders, folder, &form.connection) {
                        folders.push(stored_folder_name(folder, &form.connection));
                    }
                }
            }
            folders.sort_by_key(|folder| folder.to_lowercase());
            if let Err(err) = save_queries(&existing_queries) {
                eprintln!("Failed to import SQL queries: {err}");
            }
            if let Err(err) = save_query_folders(&folders) {
                eprintln!("Failed to import SQL query folders: {err}");
            }
        }
        Err(err) => {
            eprintln!("Failed to parse SQL query import: {err}");
        }
    }

    HttpResponse::Found()
        .append_header(("Location", format!("/sql/{}", form.connection)))
        .finish()
}

#[post("/sql/connection/delete")]
pub async fn sql_delete_connection(
    form: web::Form<DeleteConnectionForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }

        let conns = conns_opt.as_mut().unwrap();
        if let Some(idx) = conns.iter().position(|c| c.nickname == form.nickname) {
            conns.remove(idx);
            if let Err(e) = encrypt_and_save(conns) {
                eprintln!("Failed to save encrypted connections after delete: {e}");
            }
        }
    }

    HttpResponse::Found()
        .append_header(("Location", "/sql"))
        .finish()
}

// --- Helper to format unix seconds to readable string (Simplified ISO-like) ---
fn format_ts(seconds: i64) -> String {
    // Constants for date calculation
    const SECONDS_IN_MINUTE: i64 = 60;
    const SECONDS_IN_HOUR: i64 = 3600;
    const SECONDS_IN_DAY: i64 = 86400;
    const DAYS_IN_400_YEARS: i64 = 146097;
    const DAYS_IN_100_YEARS: i64 = 36524;

    let days_since_epoch = seconds / SECONDS_IN_DAY;
    let mut second_of_day = seconds % SECONDS_IN_DAY;
    if second_of_day < 0 {
        second_of_day += SECONDS_IN_DAY;
    }

    let h = second_of_day / SECONDS_IN_HOUR;
    let m = (second_of_day % SECONDS_IN_HOUR) / SECONDS_IN_MINUTE;
    let s = second_of_day % SECONDS_IN_MINUTE;

    // Shift to 0000-03-01 (Algorithm reference)
    let days = days_since_epoch + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / DAYS_IN_400_YEARS;
    let doe = days - era * DAYS_IN_400_YEARS;
    let yoe = (doe - doe / DAYS_IN_100_YEARS + doe / DAYS_IN_400_YEARS - doe / 146096) / 365; // Estimate year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // Day of year
    let mp = (5 * doy + 2) / 153; // Month
    let d = doy - (153 * mp + 2) / 5 + 1; // Day
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mp < 10 { y } else { y + 1 };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", yr, mo, d, h, m, s)
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_isoish() -> String {
    format_ts((now_millis() / 1000) as i64)
}

fn row_count_text_from_html(html: &str) -> String {
    let rows = html.matches("<tr").count().saturating_sub(1);
    if rows == 0 {
        String::new()
    } else {
        format!("{rows} rows")
    }
}

struct SqlExecution {
    html: String,
    results: Vec<HashMap<String, String>>,
}

async fn execute_sql(form: SqlForm, state: Arc<AppState>) -> SqlExecution {
    use std::convert::TryInto;

    let conn_opt = {
        let mut conns_opt = state
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        include_app_db_connection(conns_opt.as_mut().unwrap());
        let conns = conns_opt.as_ref().unwrap();
        find_connection(&form.connection, conns).cloned()
    };

    let Some(conn) = conn_opt else {
        return SqlExecution {
            html: format!(
                "<div style=\"color:var(--link-hover);\">Error: Connection '{}' not found.</div>",
                htmlescape::encode_minimal(&form.connection)
            ),
            results: Vec::new(),
        };
    };

    let mut final_sql = form.sql.clone();
    if let Some(vars) = &form.variables {
        for (key, val) in vars {
            final_sql = final_sql.replace(&format!("{{{{{}}}}}", key), val);
        }
    }

    let mut headers: Vec<String> = Vec::new();
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    let mut results_vec_for_export: Vec<HashMap<String, String>> = Vec::new();

    if conn.db_type == "sqlite" {
        let dsn = format!("sqlite:{}?mode=rwc", conn.host);
        let existing_pool = {
            let pools = state
                .sqlite_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools.get(&dsn).cloned()
        };
        let pool = if let Some(pool) = existing_pool {
            pool
        } else {
            let p = match SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&dsn)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    return SqlExecution {
                        html: format!(
                            "SQLite Connect Error: {}",
                            htmlescape::encode_minimal(&e.to_string())
                        ),
                        results: Vec::new(),
                    };
                }
            };
            let mut pools = state
                .sqlite_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools
                .entry(dsn.clone())
                .or_insert_with(|| p.clone())
                .clone()
        };

        let rows = match sqlx::query(&final_sql).fetch_all(&pool).await {
            Ok(r) => r,
            Err(e) => {
                return SqlExecution {
                    html: format!(
                        "Query Error: {}",
                        htmlescape::encode_minimal(&e.to_string())
                    ),
                    results: Vec::new(),
                };
            }
        };

        if !rows.is_empty() {
            headers = rows[0]
                .columns()
                .iter()
                .map(|c| c.name().to_string())
                .collect();
        }

        for row in rows {
            let mut ordered_row_data = Vec::new();
            let mut map_for_export = HashMap::new();
            for (idx, col) in row.columns().iter().enumerate() {
                let name = col.name().to_string();
                let val_str = if let Ok(s) = row.try_get::<String, _>(idx) {
                    s
                } else if let Ok(i) = row.try_get::<i64, _>(idx) {
                    i.to_string()
                } else if let Ok(f) = row.try_get::<f64, _>(idx) {
                    f.to_string()
                } else if let Ok(b) = row.try_get::<Vec<u8>, _>(idx) {
                    format!("<blob len={}>", b.len())
                } else if row.try_get_raw(idx).map(|r| r.is_null()).unwrap_or(true) {
                    String::new()
                } else {
                    "?".to_string()
                };
                ordered_row_data.push(val_str.clone());
                map_for_export.insert(name, val_str);
            }
            data_rows.push(ordered_row_data);
            results_vec_for_export.push(map_for_export);
        }
    } else {
        let dsn = format!(
            "postgres://{}:{}@{}/{}",
            conn.user, conn.password, conn.host, conn.db_name
        );
        let existing_pool = {
            let pools = state
                .pg_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools.get(&dsn).cloned()
        };
        let pool = if let Some(pool) = existing_pool {
            pool
        } else {
            let p = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
                Ok(p) => p,
                Err(e) => {
                    return SqlExecution {
                        html: format!(
                            "<div style=\"color:var(--link-hover);\">DB connect error: {}</div>",
                            htmlescape::encode_minimal(&e.to_string())
                        ),
                        results: Vec::new(),
                    };
                }
            };
            let mut pools = state
                .pg_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools
                .entry(dsn.clone())
                .or_insert_with(|| p.clone())
                .clone()
        };

        let rows = match sqlx::query(&final_sql).fetch_all(&pool).await {
            Ok(r) => r,
            Err(e) => {
                return SqlExecution {
                    html: format!(
                        "<div style=\"color:var(--link-hover);\">Query error: {}</div>",
                        htmlescape::encode_minimal(&e.to_string())
                    ),
                    results: Vec::new(),
                };
            }
        };

        headers = rows
            .get(0)
            .map(|row| {
                row.columns()
                    .iter()
                    .map(|col| col.name().to_string())
                    .collect()
            })
            .unwrap_or_default();

        for row in rows {
            let mut ordered_row_data = Vec::new();
            let mut map_for_export = HashMap::new();
            for (idx, col) in row.columns().iter().enumerate() {
                let name = col.name().to_string();
                let type_name = col.type_info().name();
                let display_val = if let Ok(s) = row.try_get::<String, usize>(idx) {
                    s
                } else if let Ok(i) = row.try_get::<i32, usize>(idx) {
                    i.to_string()
                } else if let Ok(i) = row.try_get::<i16, usize>(idx) {
                    i.to_string()
                } else if let Ok(i) = row.try_get::<i64, usize>(idx) {
                    i.to_string()
                } else if let Ok(f) = row.try_get::<f64, usize>(idx) {
                    f.to_string()
                } else if let Ok(f) = row.try_get::<f32, usize>(idx) {
                    f.to_string()
                } else if let Ok(b) = row.try_get::<bool, usize>(idx) {
                    b.to_string()
                } else if let Ok(json) = row.try_get::<JsonValue, usize>(idx) {
                    json.to_string().trim_matches('"').to_string()
                } else if let Ok(raw_val) = row.try_get_raw(idx) {
                    if raw_val.is_null() {
                        String::new()
                    } else if let Ok(bytes) = raw_val.as_bytes() {
                        match type_name {
                            "TIMESTAMPTZ" | "TIMESTAMP" if bytes.len() == 8 => {
                                let micros = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                format_ts((micros / 1_000_000) + 946_684_800)
                            }
                            "DATE" if bytes.len() == 4 => {
                                let days = i32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]));
                                format_ts((days as i64) * 86400 + 946_684_800)
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("")
                                    .to_string()
                            }
                            "UUID" if bytes.len() == 16 => {
                                let b = bytes;
                                format!(
                                    "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                                    b[0],
                                    b[1],
                                    b[2],
                                    b[3],
                                    b[4],
                                    b[5],
                                    b[6],
                                    b[7],
                                    b[8],
                                    b[9],
                                    b[10],
                                    b[11],
                                    b[12],
                                    b[13],
                                    b[14],
                                    b[15]
                                )
                            }
                            "MONEY" if bytes.len() == 8 => {
                                let cents = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                format!("${:.2}", cents as f64 / 100.0)
                            }
                            _ => std::str::from_utf8(bytes)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| format!("[Complex: {}]", type_name)),
                        }
                    } else {
                        format!("[Complex: {}]", type_name)
                    }
                } else {
                    format!("[Complex: {}]", type_name)
                };
                ordered_row_data.push(display_val.clone());
                map_for_export.insert(name, display_val);
            }
            data_rows.push(ordered_row_data);
            results_vec_for_export.push(map_for_export);
        }
    }

    SqlExecution {
        html: render_table(&headers, &data_rows),
        results: results_vec_for_export,
    }
}

#[post("/sql/run")]
pub async fn sql_run(form: web::Json<SqlForm>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let form = form.into_inner();
    let connection = form.connection.clone();
    let execution = execute_sql(form, state.get_ref().clone()).await;
    {
        let mut last = state
            .last_results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        last.insert(connection, execution.results);
    }
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(execution.html)
}

#[post("/sql/run-background")]
pub async fn sql_run_background(
    form: web::Json<SqlForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let form = form.into_inner();
    let job_id = format!("sql-{}-{}", now_millis(), std::process::id());
    let job = SqlJob {
        id: job_id.clone(),
        connection: form.connection.clone(),
        sql: form.sql.clone(),
        query_name: String::new(),
        query_folder: String::new(),
        status: "running".to_string(),
        created_at: now_isoish(),
        completed_at: None,
        html: None,
        row_count_text: None,
        error: None,
        results: Vec::new(),
    };

    {
        let mut jobs = state
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        jobs.insert(job_id.clone(), job);
    }

    let state_for_task = state.get_ref().clone();
    let job_id_for_task = job_id.clone();
    tokio::spawn(async move {
        let execution = execute_sql(form, state_for_task.clone()).await;
        let row_count_text = row_count_text_from_html(&execution.html);
        let results = execution.results.clone();
        let mut jobs = state_for_task
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(job) = jobs.get_mut(&job_id_for_task) {
            job.status = "completed".to_string();
            job.completed_at = Some(now_isoish());
            job.row_count_text = Some(row_count_text);
            job.html = Some(execution.html);
            job.results = execution.results;
            let mut last = state_for_task
                .last_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            last.insert(job.connection.clone(), results);
        }
    });

    HttpResponse::Ok().json(StartSqlJobResponse { job_id })
}

#[get("/sql/jobs/{connection}")]
pub async fn sql_jobs(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let connection = path.into_inner();
    let mut jobs = {
        let jobs = state
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        jobs.values()
            .filter(|job| job.connection == connection)
            .cloned()
            .collect::<Vec<_>>()
    };
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    jobs.truncate(25);
    HttpResponse::Ok().json(jobs)
}

#[get("/sql/job/{job_id}")]
pub async fn sql_job(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let job_id = path.into_inner();
    let job = {
        let jobs = state
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        jobs.get(&job_id).cloned()
    };

    match job {
        Some(job) => HttpResponse::Ok().json(job),
        None => HttpResponse::NotFound().body("SQL job not found"),
    }
}

#[post("/sql/job/{job_id}/activate")]
pub async fn sql_job_activate(
    path: web::Path<String>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let job_id = path.into_inner();
    let results = {
        let jobs = state
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        jobs.get(&job_id)
            .map(|job| (job.connection.clone(), job.results.clone()))
    };

    match results {
        Some((connection, results)) => {
            let mut last = state
                .last_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            last.insert(connection, results);
            HttpResponse::NoContent().finish()
        }
        None => HttpResponse::NotFound().body("SQL job not found"),
    }
}

#[get("/sql/{connection}/export")]
pub async fn sql_export(
    path: web::Path<String>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let connection = path.into_inner();
    let results = {
        let last_results = state
            .last_results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        last_results.get(&connection).cloned().unwrap_or_default()
    };
    let mut wtr = csv::Writer::from_writer(vec![]);

    if results.is_empty() {
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap_or_default();
        return HttpResponse::Ok()
            .content_type("text/csv")
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{connection}-results.csv\""),
            ))
            .body(data);
    }

    let mut headers: Vec<String> = results[0].keys().cloned().collect();
    headers.sort();
    wtr.write_record(&headers).ok();

    for row in results.iter() {
        let record: Vec<String> = headers
            .iter()
            .map(|h| row.get(h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&record).ok();
    }

    let data = match wtr.into_inner() {
        Ok(buf) => String::from_utf8(buf).unwrap_or_default(),
        Err(_) => "".to_string(),
    };

    HttpResponse::Ok()
        .content_type("text/csv")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{connection}-results.csv\""),
        ))
        .body(data)
}

fn render_query_view(
    nickname: &str,
    table_schema_json: &str,
    current_theme: &crate::app_state::Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    let saved_queries = load_queries()
        .into_iter()
        .filter(|query| query_matches_connection(query, nickname))
        .collect::<Vec<_>>();
    let mut query_folders = load_query_folders()
        .into_iter()
        .filter(|folder| folder_matches_connection(folder, nickname))
        .map(|folder| display_folder_name(&folder).to_string())
        .collect::<Vec<_>>();
    for query in &saved_queries {
        if let Some(folder) = query
            .folder
            .as_ref()
            .filter(|folder| !folder.trim().is_empty())
        {
            add_folder_path(&mut query_folders, folder);
        }
    }
    let existing_folders = query_folders.clone();
    for folder in existing_folders {
        add_folder_path(&mut query_folders, &folder);
    }
    query_folders.sort_by_key(|folder| folder.to_lowercase());

    let nickname_safe = htmlescape::encode_minimal(nickname);
    let nickname_attr = htmlescape::encode_attribute(nickname);
    let table_schema_json_safe = table_schema_json.replace("</", "<\\/");

    let render_query_item = |q: &SavedQuery| {
        let sql_attr = htmlescape::encode_attribute(&q.sql);
        let name_attr = htmlescape::encode_attribute(&q.name);
        let name_safe = htmlescape::encode_minimal(&q.name);
        let folder_attr = htmlescape::encode_attribute(q.folder.as_deref().unwrap_or(""));

        format!(
            "<li class=\"saved-query-item\" draggable=\"true\" data-query-name=\"{}\" data-folder=\"{}\">\
                <a href=\"#\" data-sql=\"{}\" data-name=\"{}\" data-folder=\"{}\" class=\"query-link\">{}</a>\
                <form method=\"POST\" action=\"/sql/rename\" class=\"rename-query-form\">\
                    <input type=\"hidden\" name=\"query_name\" value=\"{}\">\
                    <input type=\"hidden\" name=\"new_query_name\" value=\"\">\
                    <input type=\"hidden\" name=\"connection\" value=\"{}\">\
                    <button type=\"button\" class=\"delete-btn rename-saved-query-btn\" title=\"Rename\">✎</button>\
                </form>\
                <form method=\"POST\" action=\"/sql/delete\" class=\"delete-query-form\" onsubmit=\"return confirm('Delete saved query {}?');\">\
                    <input type=\"hidden\" name=\"query_name\" value=\"{}\">\
                    <input type=\"hidden\" name=\"connection\" value=\"{}\">\
                    <button type=\"submit\" class=\"delete-btn\" title=\"Delete\">x</button>\
                </form>\
            </li>",
            name_attr,
            folder_attr,
            sql_attr,
            name_attr,
            folder_attr,
            name_safe,
            name_attr,
            nickname_attr,
            name_attr,
            name_attr,
            nickname_attr
        )
    };

    let mut saved_query_list_parts = Vec::new();
    let unfiled_queries = saved_queries
        .iter()
        .filter(|query| query.folder.as_deref().unwrap_or("").trim().is_empty())
        .map(render_query_item)
        .collect::<Vec<_>>();
    if !unfiled_queries.is_empty() {
        saved_query_list_parts.push(r#"<li class="saved-query-folder">Unfiled</li>"#.to_string());
        saved_query_list_parts.extend(unfiled_queries);
    }
    for folder in &query_folders {
        let folder_path = normalize_folder_path(folder);
        let folder_depth = folder_path.matches('/').count();
        let folder_label = folder_basename(&folder_path);
        let folder_attr = htmlescape::encode_attribute(&folder_path);
        let folder_confirm = htmlescape::encode_attribute(&folder_path);
        saved_query_list_parts.push(format!(
            r#"<li class="saved-query-folder" draggable="true" data-folder="{folder_attr}" data-depth="{folder_depth}" style="--folder-depth:{folder_depth};"><span class="saved-query-folder-name">{}</span><form method="POST" action="/sql/folder/delete" class="delete-query-folder-form" onsubmit="return confirm('Delete folder {folder_confirm} and all saved queries inside it?');"><input type="hidden" name="folder_name" value="{folder_attr}"><input type="hidden" name="connection" value="{nickname_attr}"><button type="submit" class="delete-btn saved-folder-delete-btn" title="Delete folder">x</button></form><span class="saved-query-folder-toggle">▾</span></li>"#,
            htmlescape::encode_minimal(&folder_label)
        ));
        saved_query_list_parts.extend(
            saved_queries
                .iter()
                .filter(|query| {
                    query
                        .folder
                        .as_deref()
                        .map(normalize_folder_path)
                        .as_deref()
                        == Some(folder_path.as_str())
                })
                .map(render_query_item),
        );
    }
    let saved_query_list = saved_query_list_parts.join("\n");

    let query_folder_options = query_folders
        .iter()
        .map(|folder| {
            let folder = normalize_folder_path(folder);
            let folder_attr = htmlescape::encode_attribute(&folder);
            let folder_safe = htmlescape::encode_minimal(&folder);
            format!(r#"<option value="{folder_attr}">{folder_safe}</option>"#)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sidebar_content = format!(
        r###"
        <div class="sidebar-fixed-section">
            <div class="sql-sidebar-heading">
                 <h2 class="sql-sidebar-title">Tables</h2>
                 <button id="refresh-schema-btn" type="button" class="delete-btn sql-sidebar-refresh" title="Refresh Tables">&#x21bb;</button>
            </div>
            <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        </div>
        <ul id="table-list" class="sidebar-scroll-area"></ul>
        <div id="table-query-resizer" class="sql-sidebar-section-resizer" title="Drag to resize tables and saved queries"></div>
        
        <div class="sidebar-fixed-section">
            <div class="sql-sidebar-heading">
                <h2 class="sql-sidebar-title">Saved Queries</h2>
                <div class="sql-sidebar-actions">
                    <button type="button" id="new-sql-file-btn" class="delete-btn sql-sidebar-refresh" title="New SQL file" aria-label="New SQL file">
                        <svg class="sql-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M6 3.5h7.5L18 8v12.5H6V3.5Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                            <path d="M13.5 3.5V8H18M12 11v6M9 14h6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                    <form id="create-query-folder-form" method="POST" action="/sql/folder" class="create-query-folder-form">
                        <input type="hidden" id="new-query-folder-name" name="folder_name">
                        <input type="hidden" name="connection" value="{nickname}">
                        <button type="button" id="create-query-folder-btn" class="delete-btn sql-sidebar-refresh" title="New query folder" aria-label="New query folder">
                        <svg class="sql-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M3 6.5A2.5 2.5 0 0 1 5.5 4h4.1l2 2H18.5A2.5 2.5 0 0 1 21 8.5v9A2.5 2.5 0 0 1 18.5 20h-13A2.5 2.5 0 0 1 3 17.5v-11Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                            <path d="M12 10v6M9 13h6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                        </svg>
                        </button>
                    </form>
                    <a href="/sql/{nickname_path}/queries/export" class="delete-btn sql-sidebar-refresh sql-sidebar-link-button" title="Export saved queries" aria-label="Export saved queries">
                        <svg class="sql-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M12 3v11M8 10l4 4 4-4M5 17v3h14v-3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </a>
                    <form id="import-query-form" method="POST" action="/sql/queries/import" class="import-query-form">
                        <input type="hidden" name="connection" value="{nickname}">
                        <input type="hidden" id="import-query-payload" name="payload">
                        <input type="hidden" name="duplicate_mode" value="rename">
                        <button type="button" id="import-query-btn" class="delete-btn sql-sidebar-refresh" title="Import saved queries" aria-label="Import saved queries">
                            <svg class="sql-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M12 21V10M8 14l4-4 4 4M5 7V4h14v3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                        </button>
                        <input type="file" id="import-query-file" accept="application/json,.json" hidden>
                    </form>
                </div>
            </div>
            <div class="sidebar-search"><input type="text" id="query-search-input" placeholder="Search queries..."></div>
        </div>
        <ul id="saved-queries-list" class="sidebar-scroll-area">{saved_query_list}</ul>
        
        <form id="save-query-form" method="POST" action="/sql/save" hidden>
            <input type="hidden" id="query-name" name="query_name">
            <select id="query-folder" name="folder" hidden>
                <option value="">Unfiled</option>
                {query_folder_options}
            </select>
            <input type="hidden" id="query-sql" name="sql">
            <input type="hidden" name="connection" value="{nickname}">
        </form>
    "###,
        saved_query_list = saved_query_list,
        nickname = nickname_attr,
        nickname_path = nickname_safe,
        query_folder_options = query_folder_options
    );

    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);

    let body_content = format!(
        r###"
    <div id="sql-active-connection" data-connection="{nickname}"></div>
    <div class="sql-view-container">
      {sidebar_html}
      
      <div id="main">
        <form id="sql-form">
          <input type="hidden" name="connection" value="{nickname}">
          
          <div class="variables-section" id="variables-section">
             <!-- Variables injected here -->
             <button type="button" class="add-var-btn" onclick="addVariable()">+ Var</button>
             <button type="button" id="variable-help-btn" class="sql-var-help-btn" title="SQL variables help" aria-label="SQL variables help">?</button>
             <button type="button" id="sql-disconnect-btn" class="sql-disconnect-btn" title="Disconnect" aria-label="Disconnect from SQL manager">Disconnect</button>
          </div>

          <div class="editor-container">
            <div id="sql-backdrop" class="editor-layer"><div class="highlights"></div></div>
            <textarea id="sql-editor" class="editor-layer" name="sql" placeholder="SELECT * FROM table_name WHERE..." spellcheck="false"></textarea>
          </div>
          
          <div class="action-bar">
            <button type="submit">Run Query</button>
            <button type="button" id="clear-editor-btn" style="background-color: var(--tertiary-bg); opacity: 0.8;">Clear</button>
            <button type="button" id="save-query-btn">Save Query</button>
            <button type="button" id="save-sql-file-btn">Save SQL to File</button>
          </div>
        </form>
        
        <div id="output-resizer" class="resizer-h" title="Drag to resize"></div>
        <div class="result-tools">
            <div class="sql-result-menu" id="column-menu">
                <button type="button" id="column-menu-btn" class="add-var-btn" style="width:auto;" aria-expanded="false">Columns</button>
                <div id="column-menu-panel" class="sql-result-menu-panel" hidden>
                    <div class="sql-result-menu-empty">Run a query to choose columns.</div>
                </div>
            </div>
            <input type="text" id="output-filter" placeholder="Filter results...">
            <span id="row-count" style="font-size: 0.9em; margin: calc(var(--element-margin) / 2) var(--element-margin); color: var(--text-color);"></span>
            <select id="output-history-select" title="Cached output history">
                <option value="">Output history</option>
            </select>
            <button type="button" id="delete-output-history-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg);">Delete</button>
            <button type="button" id="clear-output-history-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg);">Clear History</button>
            <select id="sql-jobs-select" title="Running and recent SQL jobs">
                <option value="">Running queries</option>
            </select>
            <div class="sql-result-menu" id="export-menu">
                <button type="button" id="export-menu-btn" class="add-var-btn" style="width:auto;" aria-expanded="false">Export CSV</button>
                <div id="export-menu-panel" class="sql-result-menu-panel" hidden>
                    <button type="button" class="sql-result-menu-item" data-export-mode="all-headers">Export all with headers</button>
                    <button type="button" class="sql-result-menu-item" data-export-mode="all">Export all</button>
                    <button type="button" class="sql-result-menu-item" data-export-mode="selected-headers">Export selected with headers</button>
                    <button type="button" class="sql-result-menu-item" data-export-mode="selected">Export selected</button>
                </div>
            </div>
            <button type="button" id="clear-selection-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg); display: none;">Clear (0)</button>
        </div>
        <div class="output" id="output"><pre>Click a table name or enter a query and press 'Run Query'.</pre></div>
      </div>
    </div>
    
    <!-- Autocomplete container attached to body for proper floating behavior -->
    <div id="autocomplete-list"></div>
    <script type="application/json" id="sql-schema-data">{table_schema_json}</script>
    <script src="{sql_js}" defer></script>
    "###,
        nickname = nickname_attr,
        table_schema_json = table_schema_json_safe,
        sidebar_html = sidebar_html,
        sql_js = static_asset("sql.js")
    );

    render_base_page(
        &format!("SQL View: {}", htmlescape::encode_minimal(&nickname)),
        &format!(
            r#"<link rel="stylesheet" href="{}">{}"#,
            static_asset("sql.css"),
            body_content
        ),
        current_theme,
        saved_themes,
    )
}

// Ensure sql_view is pub so it can be exported
#[get("/sql/{nickname}")]
pub async fn sql_view(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let nickname = path.into_inner();
    let conn_opt = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        include_app_db_connection(conns_opt.as_mut().unwrap());
        let conns = conns_opt.as_ref().unwrap();
        conns.iter().find(|c| c.nickname == nickname).cloned()
    };
    let conn = match conn_opt {
        Some(c) => c,
        None => {
            let current_theme = state.current_theme.lock().unwrap();
            let saved_themes = state.saved_themes.lock().unwrap();
            let error_content = format!(
                r#"<h1>Error</h1><p>Connection '{nickname}' not found.</p>"#,
                nickname = htmlescape::encode_minimal(&nickname)
            );
            return HttpResponse::BadRequest().body(render_base_page(
                "Error",
                &error_content,
                &current_theme,
                &saved_themes,
            ));
        }
    };

    let schema_map = match fetch_schema_map(&conn, &state).await {
        Ok(map) => map,
        Err(e) => {
            // Just log error and return empty map for view, or handle differently
            eprintln!("Schema fetch error: {}", e);
            HashMap::new()
        }
    };

    let schema_json = serde_json::to_string(&schema_map).unwrap_or_else(|_| "{}".to_string());

    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();
    let active_connection_cookie = Cookie::build(ACTIVE_SQL_CONNECTION_COOKIE, nickname.clone())
        .path("/sql")
        .http_only(true)
        .finish();

    HttpResponse::Ok()
        .cookie(active_connection_cookie)
        .content_type("text/html; charset=utf-8")
        .body(render_query_view(
            &nickname,
            &schema_json,
            &current_theme,
            &saved_themes,
        ))
}

#[get("/sql/{nickname}/schema-json")]
pub async fn sql_schema_json(
    path: web::Path<String>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let nickname = path.into_inner();
    let conn_opt = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        include_app_db_connection(conns_opt.as_mut().unwrap());
        let conns = conns_opt.as_ref().unwrap();
        conns.iter().find(|c| c.nickname == nickname).cloned()
    };

    let conn = match conn_opt {
        Some(c) => c,
        None => return HttpResponse::NotFound().json("Connection not found"),
    };

    match fetch_schema_map(&conn, &state).await {
        Ok(map) => HttpResponse::Ok().json(map),
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

async fn fetch_schema_map(
    conn: &DbConnection,
    state: &AppState,
) -> Result<HashMap<String, Vec<String>>, String> {
    use sqlx::{Row, postgres::PgPoolOptions, sqlite::SqlitePoolOptions};
    let mut schema_map: HashMap<String, Vec<String>> = HashMap::new();

    if conn.db_type == "sqlite" {
        let dsn = format!("sqlite:{}?mode=rwc", conn.host);
        let pool = {
            let mut pools = state.sqlite_pools.lock().unwrap();
            if let Some(p) = pools.get(&dsn) {
                p.clone()
            } else {
                let p = match SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect(&dsn)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => return Err(format!("SQLite Connect Error: {}", e)),
                };
                pools.insert(dsn.clone(), p.clone());
                p
            }
        };

        // 1. Get Tables
        let table_query =
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let rows = sqlx::query(table_query)
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("Failed to fetch tables: {}", e))?;

        for row in rows {
            let table_name: String = row.get("name");
            schema_map.insert(table_name.clone(), Vec::new());

            // 2. Get Columns for each table
            let col_query = format!("PRAGMA table_info(\"{}\")", table_name);
            if let Ok(cols) = sqlx::query(&col_query).fetch_all(&pool).await {
                for col_row in cols {
                    let col_name: String = col_row.get("name");
                    if let Some(vec) = schema_map.get_mut(&table_name) {
                        vec.push(col_name);
                    }
                }
            }
        }
    } else {
        // Postgres Schema Fetching
        let dsn = format!(
            "postgres://{}:{}@{}/{}",
            conn.user, conn.password, conn.host, conn.db_name
        );
        let pool = {
            let mut pools = state.pg_pools.lock().unwrap();
            if let Some(p) = pools.get(&dsn) {
                p.clone()
            } else {
                let p = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
                    Ok(p) => p,
                    Err(e) => return Err(format!("Postgres Connect Error: {}", e)),
                };
                pools.insert(dsn.clone(), p.clone());
                p
            }
        };

        let schema_query = r#"
            SELECT table_name, column_name 
            FROM information_schema.columns 
            WHERE table_schema = 'public' 
            ORDER BY table_name, ordinal_position
        "#;

        let rows = sqlx::query(schema_query)
            .fetch_all(&pool)
            .await
            .map_err(|e| format!("Failed to fetch schema: {}", e))?;

        for row in rows {
            let table: String = row.get("table_name");
            let col: String = row.get("column_name");
            schema_map.entry(table).or_default().push(col);
        }
    }

    Ok(schema_map)
}
