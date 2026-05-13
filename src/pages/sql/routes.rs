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
    delete, get, post, web,
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
    Column, Row, TypeInfo, ValueRef,
    postgres::{PgConnectOptions, PgPoolOptions},
    sqlite::SqlitePoolOptions,
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

#[derive(Deserialize)]
struct EditConnectionForm {
    original_nickname: String,
    db_type: Option<String>,
    host: String,
    db_name: String,
    user: String,
    password: String,
    nickname: String,
}

#[derive(Serialize)]
struct StartSqlJobResponse {
    job_id: String,
}

#[derive(Deserialize)]
struct SqlRunHistoryQuery {
    tab: Option<String>,
}

#[derive(Deserialize)]
struct SqlRunHistoryPayload {
    id: String,
    connection: String,
    tab_id: Option<String>,
    sql: String,
    query_name: Option<String>,
    query_folder: Option<String>,
    status: Option<String>,
    created_at: String,
    completed_at: Option<String>,
    row_count_text: Option<String>,
    html: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct TableBrowseFilter {
    column: Option<String>,
    op: String,
    value: Option<String>,
}

#[derive(Deserialize)]
struct TableBrowseRequest {
    connection: String,
    table: String,
    page: Option<u32>,
    page_size: Option<u32>,
    filters: Option<Vec<TableBrowseFilter>>,
}

#[derive(Deserialize)]
struct TableUpdateChange {
    original: HashMap<String, String>,
    current: HashMap<String, String>,
}

#[derive(Deserialize)]
struct TableUpdateRequest {
    connection: String,
    tab_id: Option<String>,
    table: String,
    changes: Vec<TableUpdateChange>,
}

#[derive(Serialize)]
struct TableBrowseResponse {
    html: String,
    row_count_text: String,
    page: u32,
    page_size: u32,
    has_next: bool,
}

#[derive(Serialize)]
struct TableUpdateResponse {
    status: String,
    message: String,
    sql: String,
    html: String,
}

#[derive(Serialize, Clone)]
struct DbFunctionInfo {
    name: String,
    schema: String,
    signature: String,
    arguments: String,
    return_type: String,
    definition: String,
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
    conns.retain(|conn| conn.nickname != APP_DB_CONNECTION_NICKNAME);
    conns.insert(0, app_db_connection());
}

fn user_saved_connections(conns: &[DbConnection]) -> Vec<DbConnection> {
    conns
        .iter()
        .filter(|conn| conn.nickname != APP_DB_CONNECTION_NICKNAME)
        .cloned()
        .collect()
}

fn save_user_connections(conns: &[DbConnection]) {
    let user_conns = user_saved_connections(conns);
    if let Err(e) = encrypt_and_save(&user_conns) {
        eprintln!("Failed to save encrypted connections: {e}");
    }
}

fn parse_pg_host_port(host: &str) -> (String, Option<u16>) {
    let trimmed = host.trim();
    if trimmed.starts_with('[') {
        if let Some((host_part, port_part)) = trimmed.rsplit_once("]:") {
            if let Ok(port) = port_part.parse::<u16>() {
                return (host_part.trim_start_matches('[').to_string(), Some(port));
            }
        }
        return (
            trimmed
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string(),
            None,
        );
    }

    if let Some((host_part, port_part)) = trimmed.rsplit_once(':') {
        if !host_part.contains(':') {
            if let Ok(port) = port_part.parse::<u16>() {
                return (host_part.to_string(), Some(port));
            }
        }
    }

    (trimmed.to_string(), None)
}

fn pg_pool_key(conn: &DbConnection) -> String {
    format!(
        "postgres|{}|{}|{}|{}",
        conn.host, conn.db_name, conn.user, conn.password
    )
}

fn pg_connect_options(conn: &DbConnection) -> PgConnectOptions {
    let (host, port) = parse_pg_host_port(&conn.host);
    let mut options = PgConnectOptions::new()
        .host(&host)
        .username(&conn.user)
        .password(&conn.password)
        .database(&conn.db_name);
    if let Some(port) = port {
        options = options.port(port);
    }
    options
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
            let nick_attr = htmlescape::encode_attribute(&c.nickname);
            let db_type_attr = htmlescape::encode_attribute(&c.db_type);
            let host_attr = htmlescape::encode_attribute(&c.host);
            let db_name_attr = htmlescape::encode_attribute(&c.db_name);
            let user_attr = htmlescape::encode_attribute(&c.user);
            let edit_button = if c.nickname == APP_DB_CONNECTION_NICKNAME {
                String::new()
            } else {
                format!(
                    r#"<button type="button" class="edit-connection-button" title="Edit connection" aria-label="Edit connection {nick_attr}" data-nickname="{nick_attr}" data-db-type="{db_type_attr}" data-host="{host_attr}" data-db-name="{db_name_attr}" data-user="{user_attr}">✎</button>"#
                )
            };

            format!(
                r#"
                <li class="saved-connection-item">
                    <a href="/sql/{nick}" class="saved-connection-link">{display_text}</a>
                    <div class="saved-connection-actions">
                    {edit_button}
                    <form method="POST" action="/sql/connection/delete" class="delete-connection-form" onsubmit="return confirm('Delete saved connection {nick_js}?');">
                        <input type="hidden" name="nickname" value="{nick}">
                        <button type="submit" class="delete-connection-button">Delete</button>
                    </form>
                    </div>
                </li>
                "#,
                nick = htmlescape::encode_minimal(&c.nickname),
                nick_js = htmlescape::encode_attribute(&c.nickname),
                display_text = htmlescape::encode_minimal(&display_text),
                edit_button = edit_button
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

    <div id="edit-connection-modal" class="sql-connection-modal" hidden>
        <div class="sql-connection-modal-panel" role="dialog" aria-modal="true" aria-labelledby="edit-connection-title">
            <div class="sql-connection-modal-header">
                <h3 id="edit-connection-title">Edit Connection</h3>
                <button type="button" id="edit-connection-close" class="sql-connection-modal-close" aria-label="Close edit connection dialog">&times;</button>
            </div>
            <form method="POST" action="/sql/connection/update" class="connection-form edit-connection-form">
                <input type="hidden" id="edit_original_nickname" name="original_nickname">

                <label for="edit_db_type">Database Type:</label>
                <select name="db_type" id="edit_db_type">
                    <option value="postgres">Postgres</option>
                    <option value="sqlite">SQLite</option>
                </select>

                <label for="edit_nickname">Nickname:</label>
                <input name="nickname" id="edit_nickname" required>

                <label for="edit_host">Host / File Path:</label>
                <input name="host" id="edit_host" required>

                <div id="edit_pg_fields">
                    <label for="edit_db_name">Database Name:</label>
                    <input name="db_name" id="edit_db_name">

                    <label for="edit_user">User:</label>
                    <input name="user" id="edit_user">

                    <label for="edit_password">Password:</label>
                    <input name="password" id="edit_password" type="password" placeholder="Leave blank to keep existing password">
                </div>

                <div class="sql-connection-modal-actions">
                    <button type="button" id="edit-connection-cancel">Cancel</button>
                    <button type="submit">Save Changes</button>
                </div>
            </form>
        </div>
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
            let pool_key = pg_pool_key(&conn);
            let pool = {
                let mut pools = state
                    .pg_pools
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                pools.remove(&pool_key)
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
        save_user_connections(conns);
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
            save_user_connections(conns);
        }
    }

    HttpResponse::Found()
        .append_header(("Location", "/sql"))
        .finish()
}

#[post("/sql/connection/update")]
pub async fn sql_update_connection(
    form: web::Form<EditConnectionForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }

        let conns = conns_opt.as_mut().unwrap();
        let Some(idx) = conns
            .iter()
            .position(|c| c.nickname == form.original_nickname)
        else {
            return HttpResponse::NotFound().body("Connection not found");
        };

        let existing_password = conns[idx].password.clone();
        let next_password = if form.password.is_empty() {
            existing_password
        } else {
            form.password.clone()
        };

        conns[idx] = DbConnection {
            db_type: form
                .db_type
                .clone()
                .unwrap_or_else(|| "postgres".to_string()),
            host: form.host.clone(),
            db_name: form.db_name.clone(),
            user: form.user.clone(),
            password: next_password,
            nickname: form.nickname.clone(),
        };

        save_user_connections(conns);
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
        "0 rows".to_string()
    } else {
        format!("{rows} rows")
    }
}

fn quote_sql_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn quote_sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn table_reference(conn: &DbConnection, table: &str) -> String {
    if conn.db_type == "postgres" {
        format!("public.{}", quote_sql_identifier(table))
    } else {
        quote_sql_identifier(table)
    }
}

fn filter_sql_for_column(column_expr: &str, op: &str, value: &str) -> Option<String> {
    let value_literal = quote_sql_literal(value);
    match op {
        "is_null" => Some(format!("{column_expr} IS NULL")),
        "not_null" => Some(format!("{column_expr} IS NOT NULL")),
        "eq" => Some(format!("{column_expr} = {value_literal}")),
        "not_eq" => Some(format!("{column_expr} <> {value_literal}")),
        "contains" => Some(format!(
            "CAST({column_expr} AS TEXT) LIKE {}",
            quote_sql_literal(&format!("%{value}%"))
        )),
        "begins_with" => Some(format!(
            "CAST({column_expr} AS TEXT) LIKE {}",
            quote_sql_literal(&format!("{value}%"))
        )),
        "ends_with" => Some(format!(
            "CAST({column_expr} AS TEXT) LIKE {}",
            quote_sql_literal(&format!("%{value}"))
        )),
        "like" => Some(format!("CAST({column_expr} AS TEXT) LIKE {value_literal}")),
        _ => None,
    }
}

fn build_table_filter_sql(
    filters: &[TableBrowseFilter],
    columns: &[String],
) -> Result<String, String> {
    let mut clauses = Vec::new();
    for filter in filters {
        let op = filter.op.as_str();
        let value = filter.value.as_deref().unwrap_or("");
        let is_null_op = op == "is_null" || op == "not_null";
        if !is_null_op && value.is_empty() {
            continue;
        }

        let column = filter.column.as_deref().unwrap_or("");
        if column.is_empty() {
            let any_clauses = columns
                .iter()
                .filter_map(|col| filter_sql_for_column(&quote_sql_identifier(col), op, value))
                .collect::<Vec<_>>();
            if !any_clauses.is_empty() {
                clauses.push(format!("({})", any_clauses.join(" OR ")));
            }
            continue;
        }

        if !columns.iter().any(|col| col == column) {
            return Err(format!("Unknown column: {column}"));
        }
        if let Some(clause) = filter_sql_for_column(&quote_sql_identifier(column), op, value) {
            clauses.push(clause);
        }
    }

    if clauses.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!(" WHERE {}", clauses.join(" AND ")))
    }
}

fn build_table_update_sql(
    conn: &DbConnection,
    table: &str,
    columns: &[String],
    changes: &[TableUpdateChange],
) -> Result<Vec<String>, String> {
    let table_sql = table_reference(conn, table);
    let mut statements = Vec::new();

    for change in changes {
        let mut set_clauses = Vec::new();
        let mut where_clauses = Vec::new();

        for column in columns {
            let original = change.original.get(column).cloned().unwrap_or_default();
            let current = change.current.get(column).cloned().unwrap_or_default();
            let column_sql = quote_sql_identifier(column);

            where_clauses.push(format!("{column_sql} = {}", quote_sql_literal(&original)));

            if original != current {
                set_clauses.push(format!("{column_sql} = {}", quote_sql_literal(&current)));
            }
        }

        if set_clauses.is_empty() {
            continue;
        }
        if where_clauses.is_empty() {
            return Err("Cannot update table without row identity columns".to_string());
        }

        statements.push(format!(
            "UPDATE {table_sql}\nSET {}\nWHERE {};",
            set_clauses.join(",\n    "),
            where_clauses.join("\n  AND ")
        ));
    }

    Ok(statements)
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
        let pool_key = pg_pool_key(&conn);
        let connect_options = pg_connect_options(&conn);
        let existing_pool = {
            let pools = state
                .pg_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools.get(&pool_key).cloned()
        };
        let pool = if let Some(pool) = existing_pool {
            pool
        } else {
            let p = match PgPoolOptions::new()
                .max_connections(5)
                .connect_with(connect_options)
                .await
            {
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
                .entry(pool_key.clone())
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

#[post("/sql/table-data")]
pub async fn sql_table_data(
    payload: web::Json<TableBrowseRequest>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let req = payload.into_inner();
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
        find_connection(&req.connection, conns).cloned()
    };

    let Some(conn) = conn_opt else {
        return HttpResponse::NotFound().body("SQL connection not found");
    };

    let schema = match fetch_schema_map(&conn, &state).await {
        Ok(schema) => schema,
        Err(err) => return HttpResponse::InternalServerError().body(err),
    };
    let Some(columns) = schema.get(&req.table) else {
        return HttpResponse::BadRequest().body("Unknown table");
    };

    let page = req.page.unwrap_or(1).max(1);
    let page_size = req.page_size.unwrap_or(100).clamp(10, 500);
    let offset = (page - 1) * page_size;
    let filters = req.filters.unwrap_or_default();
    let where_sql = match build_table_filter_sql(&filters, columns) {
        Ok(sql) => sql,
        Err(err) => return HttpResponse::BadRequest().body(err),
    };
    let table_sql = table_reference(&conn, &req.table);
    let sql = format!(
        "SELECT * FROM {table_sql}{where_sql} LIMIT {} OFFSET {}",
        page_size + 1,
        offset
    );

    let execution = execute_sql(
        SqlForm {
            connection: req.connection,
            sql,
            variables: None,
        },
        state.get_ref().clone(),
    )
    .await;
    let mut html = execution.html;
    let returned_rows = execution.results.len();
    let has_next = returned_rows > page_size as usize;
    if has_next {
        let limited_sql = format!(
            "SELECT * FROM {table_sql}{where_sql} LIMIT {} OFFSET {}",
            page_size, offset
        );
        let limited_execution = execute_sql(
            SqlForm {
                connection: conn.nickname,
                sql: limited_sql,
                variables: None,
            },
            state.get_ref().clone(),
        )
        .await;
        html = limited_execution.html;
    }

    let visible_rows = returned_rows.min(page_size as usize);
    HttpResponse::Ok().json(TableBrowseResponse {
        html,
        row_count_text: format!("{visible_rows} rows"),
        page,
        page_size,
        has_next,
    })
}

#[post("/sql/table-update")]
pub async fn sql_table_update(
    payload: web::Json<TableUpdateRequest>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let req = payload.into_inner();
    let created_at = now_isoish();
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
        find_connection(&req.connection, conns).cloned()
    };

    let Some(conn) = conn_opt else {
        return HttpResponse::NotFound().body("SQL connection not found");
    };

    let schema = match fetch_schema_map(&conn, &state).await {
        Ok(schema) => schema,
        Err(err) => return HttpResponse::InternalServerError().body(err),
    };
    let Some(columns) = schema.get(&req.table) else {
        return HttpResponse::BadRequest().body("Unknown table");
    };

    let statements = match build_table_update_sql(&conn, &req.table, columns, &req.changes) {
        Ok(statements) => statements,
        Err(err) => return HttpResponse::BadRequest().body(err),
    };

    if statements.is_empty() {
        return HttpResponse::Ok().json(TableUpdateResponse {
            status: "noop".to_string(),
            message: "No changes to save.".to_string(),
            sql: String::new(),
            html: "<pre>No changes to save.</pre>".to_string(),
        });
    }

    let preview_sql = statements.join("\n\n");
    let mut combined_html = Vec::new();
    let mut status = "completed".to_string();
    let mut error_message = None;

    for statement in &statements {
        let execution = execute_sql(
            SqlForm {
                connection: req.connection.clone(),
                sql: statement.clone(),
                variables: None,
            },
            state.get_ref().clone(),
        )
        .await;

        if execution.html.to_lowercase().contains("query error")
            || execution.html.to_lowercase().contains("connect error")
        {
            status = "error".to_string();
            error_message = Some(execution.html.clone());
            combined_html.push(execution.html);
            break;
        }
        combined_html.push(execution.html);
    }

    let completed_at = now_isoish();
    let html = combined_html.join("\n");
    let message = if status == "completed" {
        format!("Saved {} table edit statement(s).", statements.len())
    } else {
        "Table edit failed. See output for database error details.".to_string()
    };

    if let Err(err) = app_db::upsert_sql_run_history(&app_db::SqlRunHistoryRecord {
        id: format!("sql-edit-{}-{}", now_millis(), std::process::id()),
        connection: req.connection,
        tab_id: req.tab_id.unwrap_or_default(),
        sql: preview_sql.clone(),
        query_name: format!("Edit {}", req.table),
        query_folder: String::new(),
        status: status.clone(),
        created_at,
        completed_at: Some(completed_at),
        row_count_text: Some(message.clone()),
        html: Some(html.clone()),
        error: error_message,
    })
    .await
    {
        eprintln!("Failed to persist SQL table edit history: {err}");
    }

    HttpResponse::Ok().json(TableUpdateResponse {
        status,
        message,
        sql: preview_sql,
        html,
    })
}

#[post("/sql/run-background")]
pub async fn sql_run_background(
    form: web::Json<SqlForm>,
    state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let form = form.into_inner();
    let job_id = format!("sql-{}-{}", now_millis(), std::process::id());
    let created_at = now_isoish();
    let job = SqlJob {
        id: job_id.clone(),
        connection: form.connection.clone(),
        sql: form.sql.clone(),
        query_name: String::new(),
        query_folder: String::new(),
        status: "running".to_string(),
        created_at: created_at.clone(),
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
    if let Err(err) = app_db::upsert_sql_run_history(&app_db::SqlRunHistoryRecord {
        id: job_id.clone(),
        connection: form.connection.clone(),
        tab_id: String::new(),
        sql: form.sql.clone(),
        query_name: String::new(),
        query_folder: String::new(),
        status: "running".to_string(),
        created_at,
        completed_at: None,
        row_count_text: None,
        html: None,
        error: None,
    })
    .await
    {
        eprintln!("Failed to persist SQL running job: {err}");
    }

    let state_for_task = state.get_ref().clone();
    let job_id_for_task = job_id.clone();
    let form_for_history = form.clone();
    tokio::spawn(async move {
        let execution = execute_sql(form, state_for_task.clone()).await;
        let row_count_text = row_count_text_from_html(&execution.html);
        let results = execution.results.clone();
        let completed_at = now_isoish();
        let history_record = {
            let mut jobs = state_for_task
                .sql_jobs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut history_record = None;
            if let Some(job) = jobs.get_mut(&job_id_for_task) {
                job.status = "completed".to_string();
                job.completed_at = Some(completed_at.clone());
                job.row_count_text = Some(row_count_text.clone());
                job.html = Some(execution.html.clone());
                job.results = execution.results;
                let mut last = state_for_task
                    .last_results
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                last.insert(job.connection.clone(), results);
                history_record = Some(app_db::SqlRunHistoryRecord {
                    id: job.id.clone(),
                    connection: job.connection.clone(),
                    tab_id: String::new(),
                    sql: job.sql.clone(),
                    query_name: job.query_name.clone(),
                    query_folder: job.query_folder.clone(),
                    status: job.status.clone(),
                    created_at: job.created_at.clone(),
                    completed_at: job.completed_at.clone(),
                    row_count_text: job.row_count_text.clone(),
                    html: job.html.clone(),
                    error: job.error.clone(),
                });
            }
            history_record
        };
        let history_record = history_record.unwrap_or_else(|| app_db::SqlRunHistoryRecord {
            id: job_id_for_task,
            connection: form_for_history.connection,
            tab_id: String::new(),
            sql: form_for_history.sql,
            query_name: String::new(),
            query_folder: String::new(),
            status: "completed".to_string(),
            created_at: completed_at.clone(),
            completed_at: Some(completed_at),
            row_count_text: Some(row_count_text),
            html: Some(execution.html),
            error: None,
        });
        if let Err(err) = app_db::upsert_sql_run_history(&history_record).await {
            eprintln!("Failed to persist completed SQL job: {err}");
        }
    });

    HttpResponse::Ok().json(StartSqlJobResponse { job_id })
}

#[get("/sql/jobs/{connection}")]
pub async fn sql_jobs(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let connection = path.into_inner();
    let mut jobs = app_db::get_sql_run_history(&connection, None, 25)
        .await
        .into_iter()
        .map(sql_job_from_history_record)
        .collect::<Vec<_>>();

    let memory_jobs = {
        let jobs = state
            .sql_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        jobs.values()
            .filter(|job| job.connection == connection)
            .cloned()
            .collect::<Vec<_>>()
    };
    for job in memory_jobs {
        if !jobs.iter().any(|existing| existing.id == job.id) {
            jobs.push(job);
        }
    }
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

    let job = match job {
        Some(job) => Some(job),
        None => app_db::get_sql_run_history_by_id(&job_id)
            .await
            .map(sql_job_from_history_record),
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

#[get("/sql/run-history/{connection}")]
pub async fn sql_run_history_get(
    path: web::Path<String>,
    query: web::Query<SqlRunHistoryQuery>,
) -> impl Responder {
    let connection = path.into_inner();
    let tab = query.tab.as_deref().filter(|tab| !tab.trim().is_empty());
    let history = app_db::get_sql_run_history(&connection, tab, 50).await;
    HttpResponse::Ok().json(history)
}

#[post("/sql/run-history")]
pub async fn sql_run_history_save(payload: web::Json<SqlRunHistoryPayload>) -> impl Responder {
    let payload = payload.into_inner();
    let record = app_db::SqlRunHistoryRecord {
        id: payload.id,
        connection: payload.connection,
        tab_id: payload.tab_id.unwrap_or_default(),
        sql: payload.sql,
        query_name: payload.query_name.unwrap_or_default(),
        query_folder: payload.query_folder.unwrap_or_default(),
        status: payload.status.unwrap_or_else(|| "completed".to_string()),
        created_at: payload.created_at,
        completed_at: payload.completed_at,
        row_count_text: payload.row_count_text,
        html: payload.html,
        error: payload.error,
    };

    match app_db::upsert_sql_run_history(&record).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to save SQL history: {err}"))
        }
    }
}

#[delete("/sql/run-history/{id}")]
pub async fn sql_run_history_delete(path: web::Path<String>) -> impl Responder {
    match app_db::delete_sql_run_history(&path.into_inner()).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to delete SQL history: {err}"))
        }
    }
}

#[delete("/sql/run-history/connection/{connection}")]
pub async fn sql_run_history_clear(
    path: web::Path<String>,
    query: web::Query<SqlRunHistoryQuery>,
) -> impl Responder {
    let tab = query.tab.as_deref().filter(|tab| !tab.trim().is_empty());
    match app_db::clear_sql_run_history(&path.into_inner(), tab).await {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Failed to clear SQL history: {err}"))
        }
    }
}

fn sql_job_from_history_record(record: app_db::SqlRunHistoryRecord) -> SqlJob {
    SqlJob {
        id: record.id,
        connection: record.connection,
        sql: record.sql,
        query_name: record.query_name,
        query_folder: record.query_folder,
        status: record.status,
        created_at: record.created_at,
        completed_at: record.completed_at,
        html: record.html,
        row_count_text: record.row_count_text,
        error: record.error,
        results: Vec::new(),
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
    db_type: &str,
    table_schema_json: &str,
    function_schema_json: &str,
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
    let function_schema_json_safe = function_schema_json.replace("</", "<\\/");

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
                 <div class="sql-schema-tabs" role="tablist" aria-label="SQL schema objects">
                     <button type="button" id="schema-tab-tables" class="sql-schema-tab active" data-schema-tab="tables" role="tab" aria-selected="true">Tables</button>
                     <button type="button" id="schema-tab-functions" class="sql-schema-tab" data-schema-tab="functions" role="tab" aria-selected="false">Functions</button>
                 </div>
                 <button id="refresh-schema-btn" type="button" class="delete-btn sql-sidebar-refresh" title="Refresh schema">&#x21bb;</button>
            </div>
            <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        </div>
        <ul id="table-list" class="sidebar-scroll-area"></ul>
        <ul id="function-list" class="sidebar-scroll-area" hidden></ul>
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
    <div id="sql-active-connection" data-connection="{nickname}" data-db-type="{db_type}"></div>
    <div class="sql-view-container">
      {sidebar_html}
      
      <div id="main">
        <form id="sql-form">
          <input type="hidden" name="connection" value="{nickname}">
          
          <div class="variables-section" id="variables-section">
             <div class="variables-left">
               <!-- Variables injected here -->
               <button type="button" class="add-var-btn" onclick="addVariable()">+ Var</button>
             </div>
             <div class="variables-actions">
               <button type="submit">Run Query</button>
               <button type="button" id="clear-editor-btn" style="background-color: var(--tertiary-bg); opacity: 0.8;">Clear</button>
               <button type="button" id="save-query-btn">Save Query</button>
               <button type="button" id="save-sql-file-btn">Save SQL to File</button>
             </div>
          </div>

          <div class="editor-container">
            <div id="sql-backdrop" class="editor-layer"><div class="highlights"></div></div>
            <textarea id="sql-editor" class="editor-layer" name="sql" placeholder="SELECT * FROM table_name WHERE..." spellcheck="false"></textarea>
          </div>
        </form>
        
        <div id="output-resizer" class="resizer-h" title="Drag to resize"></div>
        <div class="result-tools">
            <div class="result-tools-left">
                <input type="text" id="output-filter" placeholder="Filter results...">
                <div class="sql-result-menu" id="column-menu">
                    <button type="button" id="column-menu-btn" class="add-var-btn" style="width:auto;" aria-expanded="false">Columns</button>
                    <div id="column-menu-panel" class="sql-result-menu-panel" hidden>
                        <div class="sql-result-menu-empty">Run a query to choose columns.</div>
                    </div>
                </div>
                <span id="row-count" style="font-size: 0.9em; margin: calc(var(--element-margin) / 2) var(--element-margin); color: var(--text-color);"></span>
                <div class="sql-result-menu" id="export-menu">
                    <button type="button" id="export-menu-btn" class="add-var-btn" style="width:auto;" aria-expanded="false">Export CSV</button>
                    <div id="export-menu-panel" class="sql-result-menu-panel" hidden>
                        <button type="button" class="sql-result-menu-item" data-export-mode="all-headers">Export all with headers</button>
                        <button type="button" class="sql-result-menu-item" data-export-mode="all">Export all</button>
                        <button type="button" class="sql-result-menu-item" data-export-mode="selected-headers">Export selected with headers</button>
                        <button type="button" class="sql-result-menu-item" data-export-mode="selected">Export selected</button>
                    </div>
                </div>
                <button type="button" id="clear-output-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg);">Clear Output</button>
                <button type="button" id="clear-selection-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg); display: none;">Clear (0)</button>
            </div>
            <div class="result-tools-right">
                <select id="output-history-select" title="Cached output history">
                    <option value="">Output history</option>
                </select>
                <button type="button" id="delete-output-history-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg);">Delete</button>
                <button type="button" id="clear-output-history-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg);">Clear History</button>
                <select id="sql-jobs-select" title="Running and recent SQL jobs">
                    <option value="">Running queries</option>
                </select>
            </div>
        </div>
        <div class="output" id="output"><pre>Click a table name or enter a query and press 'Run Query'.</pre></div>
      </div>
    </div>
    
    <!-- Autocomplete container attached to body for proper floating behavior -->
    <div id="autocomplete-list"></div>
    <script type="application/json" id="sql-schema-data">{table_schema_json}</script>
    <script type="application/json" id="sql-functions-data">{function_schema_json}</script>
    <script src="{sql_js}" defer></script>
    "###,
        nickname = nickname_attr,
        db_type = htmlescape::encode_attribute(db_type),
        table_schema_json = table_schema_json_safe,
        function_schema_json = function_schema_json_safe,
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
    let function_list = match fetch_function_list(&conn, &state).await {
        Ok(functions) => functions,
        Err(e) => {
            eprintln!("Function fetch error: {}", e);
            Vec::new()
        }
    };
    let function_json = serde_json::to_string(&function_list).unwrap_or_else(|_| "[]".to_string());

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
            &conn.db_type,
            &schema_json,
            &function_json,
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

#[get("/sql/{nickname}/functions-json")]
pub async fn sql_functions_json(
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

    match fetch_function_list(&conn, &state).await {
        Ok(functions) => HttpResponse::Ok().json(functions),
        Err(e) => HttpResponse::InternalServerError().json(format!("Error: {}", e)),
    }
}

async fn fetch_schema_map(
    conn: &DbConnection,
    state: &AppState,
) -> Result<HashMap<String, Vec<String>>, String> {
    use sqlx::{Row, sqlite::SqlitePoolOptions};
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
        let pool_key = pg_pool_key(conn);
        let connect_options = pg_connect_options(conn);
        let existing_pool = {
            let pools = state
                .pg_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools.get(&pool_key).cloned()
        };
        let pool = if let Some(pool) = existing_pool {
            pool
        } else {
            let p = match PgPoolOptions::new()
                .max_connections(5)
                .connect_with(connect_options)
                .await
            {
                Ok(p) => p,
                Err(e) => return Err(format!("Postgres Connect Error: {}", e)),
            };
            let mut pools = state
                .pg_pools
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pools
                .entry(pool_key.clone())
                .or_insert_with(|| p.clone())
                .clone()
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

async fn fetch_function_list(
    conn: &DbConnection,
    state: &AppState,
) -> Result<Vec<DbFunctionInfo>, String> {
    use sqlx::Row;

    if conn.db_type == "sqlite" {
        return Ok(Vec::new());
    }

    let pool_key = pg_pool_key(conn);
    let connect_options = pg_connect_options(conn);
    let existing_pool = {
        let pools = state
            .pg_pools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        pools.get(&pool_key).cloned()
    };
    let pool = if let Some(pool) = existing_pool {
        pool
    } else {
        let p = match PgPoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await
        {
            Ok(p) => p,
            Err(e) => return Err(format!("Postgres Connect Error: {}", e)),
        };
        let mut pools = state
            .pg_pools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        pools
            .entry(pool_key.clone())
            .or_insert_with(|| p.clone())
            .clone()
    };

    let function_query = r#"
        SELECT
            n.nspname AS schema_name,
            p.proname AS function_name,
            pg_get_function_identity_arguments(p.oid) AS arguments,
            pg_get_function_result(p.oid) AS return_type,
            pg_get_functiondef(p.oid) AS definition
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
        ORDER BY n.nspname, p.proname, pg_get_function_identity_arguments(p.oid)
    "#;

    let rows = sqlx::query(function_query)
        .fetch_all(&pool)
        .await
        .map_err(|e| format!("Failed to fetch functions: {}", e))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let schema: String = row.get("schema_name");
            let name: String = row.get("function_name");
            let arguments: String = row.get("arguments");
            let return_type: String = row.get("return_type");
            let definition: String = row.get("definition");
            let signature = format!("{schema}.{name}({arguments})");

            DbFunctionInfo {
                name,
                schema,
                signature,
                arguments,
                return_type,
                definition,
            }
        })
        .collect())
}
