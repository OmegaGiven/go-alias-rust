use actix_web::{get, post, web, HttpResponse, Responder};
use std::{collections::HashMap, sync::Arc, fs, io};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;
use crate::pages::sql::{
    DbConnection, SqlForm, AddConnForm,
    find_connection, render_table,
    encrypt_and_save, load_and_decrypt,
};
// Added ValueRef to fix .is_null() error
use sqlx::{Row, Column, TypeInfo, postgres::PgPoolOptions, sqlite::SqlitePoolOptions, types::JsonValue, ValueRef}; 

const QUERIES_FILE: &str = "saved_queries.json";

#[derive(Serialize, Deserialize, Clone)]
struct SavedQuery {
    name: String,
    sql: String,
}

#[derive(Deserialize)]
struct SaveQueryForm {
    query_name: String,
    sql: String,
    connection: String, 
}

#[derive(Deserialize)]
struct DeleteQueryForm {
    query_name: String,
    connection: String, 
}

#[derive(Deserialize)]
struct DeleteConnectionForm {
    nickname: String,
}

fn load_queries() -> Vec<SavedQuery> {
    fs::read_to_string(QUERIES_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_queries(queries: &[SavedQuery]) -> io::Result<()> {
    let data = serde_json::to_string_pretty(queries)?;
    fs::write(QUERIES_FILE, data)
}

fn delete_query(name: &str) -> io::Result<()> {
    let mut queries = load_queries();
    if let Some(pos) = queries.iter().position(|q| q.name == name) {
        queries.remove(pos);
        save_queries(&queries)?;
    }
    Ok(())
}

fn render_connection_list(conns: &[DbConnection], current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
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

    let content = format!(r#"
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
                  
                  <button type="submit">Save Connection</button>
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
                    
                    <button type="submit" style="background-color: var(--link-color); color: var(--primary-bg); font-weight: bold;">Create & Save</button>
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
    <link rel="stylesheet" href="/static/sql_connections.css">
    <script src="/static/sql_connections.js" defer></script>
    "#, conn_links = conn_links);
    
    render_base_page("SQL Connections", &content, current_theme, saved_themes)
}


#[get("/sql")]
pub async fn sql_get(state: web::Data<Arc<AppState>>) -> impl Responder {
    let conns = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        conns_opt.clone().unwrap()
    };
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_connection_list(&conns, &current_theme, &saved_themes))
}

#[post("/sql/add")]
pub async fn sql_add(form: web::Form<AddConnForm>, state: web::Data<Arc<AppState>>) -> impl Responder {
    let new_conn = DbConnection {
        db_type: form.db_type.clone().unwrap_or_else(|| "postgres".to_string()),
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
    HttpResponse::Found().append_header(("Location", "/sql")).finish()
}

#[post("/sql/save")]
pub async fn sql_save(form: web::Form<SaveQueryForm>) -> impl Responder {
    let mut queries = load_queries();
    
    if let Some(idx) = queries.iter().position(|q| q.name == form.query_name) {
        queries[idx].sql = form.sql.clone();
    } else {
        queries.push(SavedQuery {
            name: form.query_name.clone(),
            sql: form.sql.clone(),
        });
    }
    
    if let Err(e) = save_queries(&queries) {
        eprintln!("Failed to save queries: {e}");
    }
    
    // Redirect back to the specific connection view
    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found().append_header(("Location", location)).finish()
}

#[post("/sql/delete")]
pub async fn sql_delete(form: web::Form<DeleteQueryForm>) -> impl Responder {
    if let Err(e) = delete_query(&form.query_name) {
        eprintln!("Failed to delete query: {e}");
    }
    
    // Redirect back to the specific connection view
    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found().append_header(("Location", location)).finish()
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

    HttpResponse::Found().append_header(("Location", "/sql")).finish()
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
    if second_of_day < 0 { second_of_day += SECONDS_IN_DAY; }

    let h = second_of_day / SECONDS_IN_HOUR;
    let m = (second_of_day % SECONDS_IN_HOUR) / SECONDS_IN_MINUTE;
    let s = second_of_day % SECONDS_IN_MINUTE;

    // Shift to 0000-03-01 (Algorithm reference)
    let days = days_since_epoch + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / DAYS_IN_400_YEARS;
    let doe = days - era * DAYS_IN_400_YEARS;
    let yoe = (doe - doe/DAYS_IN_100_YEARS + doe/DAYS_IN_400_YEARS - doe/146096) / 365; // Estimate year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe/4 - yoe/100); // Day of year
    let mp = (5 * doy + 2) / 153; // Month
    let d = doy - (153 * mp + 2) / 5 + 1; // Day
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mp < 10 { y } else { y + 1 };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", yr, mo, d, h, m, s)
}


#[post("/sql/run")]
pub async fn sql_run(form: web::Json<SqlForm>, state: web::Data<Arc<AppState>>) -> impl Responder {
    use std::convert::TryInto; 

    let conn_opt = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
        let conns = conns_opt.as_ref().unwrap();
        find_connection(&form.connection, conns).cloned()
    };

    if conn_opt.is_none() {
        return HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(format!("<div style=\"color:var(--link-hover);\">Error: Connection '{}' not found.</div>", htmlescape::encode_minimal(&form.connection)));
    }

    let conn = conn_opt.unwrap();
    
    // --- Variable Substitution ---
    let mut final_sql = form.sql.clone();
    if let Some(vars) = &form.variables {
        for (key, val) in vars {
            // Replace {{key}} with val
            let placeholder = format!("{{{{{}}}}}", key);
            final_sql = final_sql.replace(&placeholder, val);
        }
    }

    let mut headers: Vec<String> = Vec::new();
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    let mut results_vec_for_export: Vec<HashMap<String, String>> = Vec::new();

    // --- EXECUTION BRANCHING ---
    if conn.db_type == "sqlite" {
        // --- SQLITE EXECUTION ---
        let dsn = format!("sqlite:{}?mode=rwc", conn.host);
        
        let pool = {
            let mut pools = state.sqlite_pools.lock().unwrap();
            if let Some(p) = pools.get(&dsn) {
                p.clone()
            } else {
                let p = match SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect(&dsn).await 
                {
                    Ok(p) => p,
                    Err(e) => return HttpResponse::Ok().body(format!("SQLite Connect Error: {}", e)),
                };
                pools.insert(dsn.clone(), p.clone());
                p
            }
        };

        let rows = match sqlx::query(&final_sql).fetch_all(&pool).await {
            Ok(r) => r,
            Err(e) => return HttpResponse::Ok().body(format!("Query Error: {}", e)),
        };

        if !rows.is_empty() {
            headers = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
        }

        for row in rows {
            let mut ordered_row_data: Vec<String> = Vec::new();
            let mut map_for_export: HashMap<String, String> = HashMap::new();

            for (idx, col) in row.columns().iter().enumerate() {
                let name = col.name().to_string();
                
                // Generic SQLite displayer
                let val_str = if let Ok(s) = row.try_get::<String, _>(idx) {
                    s
                } else if let Ok(i) = row.try_get::<i64, _>(idx) {
                    i.to_string()
                } else if let Ok(f) = row.try_get::<f64, _>(idx) {
                    f.to_string()
                } else if let Ok(b) = row.try_get::<Vec<u8>, _>(idx) {
                    format!("<blob len={}>", b.len())
                } else if row.try_get_raw(idx).map(|r| r.is_null()).unwrap_or(true) {
                    "".to_string()
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
        // --- POSTGRES EXECUTION ---
        let dsn = format!("postgres://{}:{}@{}/{}", conn.user, conn.password, conn.host, conn.db_name);
        
        let pool = {
            let mut pools = state.pg_pools.lock().unwrap();
            if let Some(p) = pools.get(&dsn) {
                p.clone()
            } else {
                let p = match PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&dsn).await 
                {
                    Ok(p) => p,
                    Err(e) => {
                        return HttpResponse::Ok()
                            .content_type("text/html; charset=utf-8")
                            .body(format!("<div style=\"color:var(--link-hover);\">DB connect error: {}</div>", htmlescape::encode_minimal(&e.to_string())));
                    }
                };
                pools.insert(dsn.clone(), p.clone());
                p
            }
        };

        let rows = match sqlx::query(&final_sql).fetch_all(&pool).await {
            Ok(r) => r,
            Err(e) => {
                return HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(format!("<div style=\"color:var(--link-hover);\">Query error: {}</div></div>", htmlescape::encode_minimal(&e.to_string())));
            }
        };

        headers = rows.get(0)
            .map(|row| row.columns().iter().map(|col| col.name().to_string()).collect())
            .unwrap_or_default();

        
        for row in rows {
            let mut ordered_row_data: Vec<String> = Vec::new();
            let mut map_for_export: HashMap<String, String> = HashMap::new();

            let get_display_val = |row: &sqlx::postgres::PgRow, idx: usize| -> String {
                let col = row.column(idx);
                let type_name = col.type_info().name();

                // 1. Try standard string/text decoding first
                if let Ok(s) = row.try_get::<String, usize>(idx) { 
                    return s; 
                }

                // 2. Try generic primitive decoding BEFORE binary/raw fallbacks
                // This prevents misinterpreting numeric/binary data as UTF-8
                if let Ok(i) = row.try_get::<i32, usize>(idx) { return i.to_string(); }
                if let Ok(i) = row.try_get::<i16, usize>(idx) { return i.to_string(); }
                if let Ok(i) = row.try_get::<i64, usize>(idx) { return i.to_string(); }
                
                // Floats
                if let Ok(f) = row.try_get::<f64, usize>(idx) { return f.to_string(); }
                if let Ok(f) = row.try_get::<f32, usize>(idx) { return f.to_string(); }
                
                // Booleans
                if let Ok(b) = row.try_get::<bool, usize>(idx) { return b.to_string(); }

                // 3. Handle specific types manually via raw bytes IF string decoding failed
                if let Ok(raw_val) = row.try_get_raw(idx) {
                    if raw_val.is_null() {
                        return "".to_string();
                    }

                    if let Ok(bytes) = raw_val.as_bytes() {
                        match type_name {
                            "TIMESTAMPTZ" | "TIMESTAMP" => {
                                if bytes.len() == 8 {
                                    let micros = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                    let seconds = (micros / 1_000_000) + 946_684_800; 
                                    return format_ts(seconds);
                                }
                            },
                            "DATE" => {
                                if bytes.len() == 4 {
                                    let days = i32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]));
                                    let seconds = (days as i64) * 86400 + 946_684_800;
                                    return format_ts(seconds).split_whitespace().next().unwrap_or("").to_string();
                                }
                            },
                            "UUID" => {
                                if bytes.len() == 16 {
                                    let b = bytes;
                                    return format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                                        b[0],b[1],b[2],b[3], b[4],b[5], b[6],b[7], b[8],b[9], b[10],b[11],b[12],b[13],b[14],b[15]);
                                }
                            },
                            "MONEY" => {
                                if bytes.len() == 8 {
                                    let cents = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                    return format!("${:.2}", cents as f64 / 100.0);
                                }
                            },
                            _ => {
                                // Generic UTF-8 Fallback ONLY if we haven't found a better match
                                if let Ok(s) = std::str::from_utf8(bytes) {
                                    return s.to_string();
                                }
                            }
                        }
                    }
                }

                // 4. Try JSON
                if let Ok(json) = row.try_get::<JsonValue, usize>(idx) {
                    let s = json.to_string();
                    return s.trim_matches('"').to_string();
                }

                // Fallback with Type Name for debugging
                format!("[Complex: {}]", type_name)
            };

            for (idx, col) in row.columns().iter().enumerate() {
                let name = col.name().to_string();
                let display_val = get_display_val(&row, idx);
                ordered_row_data.push(display_val.clone());
                map_for_export.insert(name, display_val);
            }
            data_rows.push(ordered_row_data);
            results_vec_for_export.push(map_for_export);
        }
    } // End Postgres Branch

    {
        let mut last = state.last_results.lock().unwrap();
        *last = results_vec_for_export;
    }

    let table = render_table(&headers, &data_rows);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(table)
}

#[get("/sql/export")]
pub async fn sql_export(state: web::Data<Arc<AppState>>) -> impl Responder {
    let results = state.last_results.lock().unwrap();
    let mut wtr = csv::Writer::from_writer(vec![]);

    if results.is_empty() {
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap_or_default();
        return HttpResponse::Ok()
            .content_type("text/csv")
            .append_header(("Content-Disposition", "attachment; filename=\"results.csv\""))
            .body(data);
    }

    let mut headers: Vec<String> = results[0].keys().cloned().collect();
    headers.sort();
    wtr.write_record(&headers).ok();

    for row in results.iter() {
        let record: Vec<String> = headers.iter()
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
        .append_header(("Content-Disposition", "attachment; filename=\"results.csv\""))
        .body(data)
}

fn render_query_view(nickname: &str, table_schema_json: &str, current_theme: &crate::app_state::Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let saved_queries = load_queries();
    let nickname_safe = htmlescape::encode_minimal(nickname);
    
    let saved_query_list = saved_queries.iter()
        .map(|q| {
            let sql_safe = htmlescape::encode_minimal(&q.sql);
            let name_safe = htmlescape::encode_minimal(&q.name);
            
            format!(
                "<li class=\"saved-query-item\">\
                    <form method=\"POST\" action=\"/sql/delete\" class=\"delete-query-form\">\
                        <input type=\"hidden\" name=\"query_name\" value=\"{}\">\
                        <input type=\"hidden\" name=\"connection\" value=\"{}\">\
                        <button type=\"submit\" class=\"delete-btn\" title=\"Delete\">x</button>\
                    </form>\
                    <a href=\"#\" data-sql=\"{}\" data-name=\"{}\" class=\"query-link\">{}</a>\
                </li>",
                name_safe, nickname_safe, sql_safe, name_safe, name_safe
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let sidebar_content = format!(r###"
        <div class="sidebar-fixed-section">
            <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-color); padding-bottom:2px; margin: calc(var(--element-margin) / 2) var(--element-margin);">
                 <h2 style="margin: calc(var(--element-margin) / 2) var(--element-margin); border:none;">Tables</h2>
                 <button id="refresh-schema-btn" type="button" class="delete-btn" style="width:auto; font-size:1.2em;" title="Refresh Tables">&#x21bb;</button>
            </div>
            <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        </div>
        <ul id="table-list" class="sidebar-scroll-area"></ul>
        
        <div class="sidebar-fixed-section">
            <h2 style="margin: calc(var(--element-margin) / 2) var(--element-margin);">Saved Queries</h2>
            <div class="sidebar-search"><input type="text" id="query-search-input" placeholder="Search queries..."></div>
        </div>
        <ul id="saved-queries-list" class="sidebar-scroll-area">{saved_query_list}</ul>
        
        <div class="sidebar-fixed-section">
            <form id="save-query-form" method="POST" action="/sql/save" class="query-save-form">
                <input type="text" id="query-name" name="query_name" placeholder="Name query to save" required>
                <input type="hidden" id="query-sql" name="sql">
                <input type="hidden" name="connection" value="{nickname}">
                <button type="submit">Save Current Query</button>
            </form>
        </div>
    "###, saved_query_list = saved_query_list, nickname = nickname_safe);
    
    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);

    let body_content = format!(r###"
    <div class="sql-view-container">
      {sidebar_html}
      
      <div id="main">
        <form id="sql-form">
          <input type="hidden" name="connection" value="{nickname}">
          
          <div class="variables-section" id="variables-section">
             <!-- Variables injected here -->
             <button type="button" class="add-var-btn" onclick="addVariable()">+ Var</button>
          </div>

          <div class="editor-container">
            <div id="sql-backdrop" class="editor-layer"><div class="highlights"></div></div>
            <textarea id="sql-editor" class="editor-layer" name="sql" placeholder="SELECT * FROM table_name WHERE..." spellcheck="false"></textarea>
          </div>
          
          <div class="action-bar">
            <button type="submit">Run Query</button>
            <button type="button" id="clear-editor-btn" style="background-color: var(--tertiary-bg); opacity: 0.8;">Clear</button>
            <button type="button" id="save-sql-file-btn">Save SQL File</button>
          </div>
        </form>
        
        <div id="output-resizer" class="resizer-h" title="Drag to resize"></div>
        <div class="result-tools">
            <input type="text" id="output-filter" placeholder="Filter results...">
            <span id="row-count" style="font-size: 0.9em; margin: calc(var(--element-margin) / 2) var(--element-margin); color: var(--text-color);"></span>
            <label><input type="checkbox" id="export-headers" checked> Headers</label>
            <button type="button" id="export-client-btn" class="add-var-btn" style="width:auto;">Export Select CSV</button>
            <button type="button" id="clear-selection-btn" class="add-var-btn" style="width:auto; background-color: var(--tertiary-bg); display: none;">Clear (0)</button>
            <a href="/sql/export" target="_blank" title="Download all latest results from server" style="text-decoration:none;"><button type="button" class="add-var-btn" style="width:auto;">Export All</button></a>
        </div>
        <div class="output" id="output"><pre>Click a table name or enter a query and press 'Run Query'.</pre></div>
      </div>
    </div>
    
    <!-- Autocomplete container attached to body for proper floating behavior -->
    <div id="autocomplete-list"></div>
    <template id="sql-schema-data">{table_schema_json}</template>
    <script src="/static/sql.js" defer></script>
    "###, nickname = nickname_safe, table_schema_json = table_schema_json, sidebar_html = sidebar_html);

    render_base_page(
        &format!("SQL View: {}", htmlescape::encode_minimal(&nickname)),
        &format!(r#"<link rel="stylesheet" href="/static/sql.css">{}"#, body_content),
        current_theme,
        saved_themes
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
        let conns = conns_opt.as_ref().unwrap();
        conns.iter().find(|c| c.nickname == nickname).cloned()
    };
    let conn = match conn_opt {
        Some(c) => c,
        None => {
            let current_theme = state.current_theme.lock().unwrap();
            let saved_themes = state.saved_themes.lock().unwrap();
            let error_content = format!(r#"<h1>Error</h1><p>Connection '{nickname}' not found.</p>"#, nickname = htmlescape::encode_minimal(&nickname));
            return HttpResponse::BadRequest().body(render_base_page("Error", &error_content, &current_theme, &saved_themes));
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
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_query_view(&nickname, &schema_json, &current_theme, &saved_themes))
}

#[get("/sql/{nickname}/schema-json")]
pub async fn sql_schema_json(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    
    let nickname = path.into_inner();
    let conn_opt = {
        let mut conns_opt = state.connections.lock().unwrap();
        if conns_opt.is_none() {
            *conns_opt = Some(load_and_decrypt());
        }
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

async fn fetch_schema_map(conn: &DbConnection, state: &AppState) -> Result<HashMap<String, Vec<String>>, String> {
     use sqlx::{Row, sqlite::SqlitePoolOptions, postgres::PgPoolOptions};
     let mut schema_map: HashMap<String, Vec<String>> = HashMap::new();

     if conn.db_type == "sqlite" {
        let dsn = format!("sqlite:{}?mode=rwc", conn.host);
        let pool = {
            let mut pools = state.sqlite_pools.lock().unwrap();
            if let Some(p) = pools.get(&dsn) {
                p.clone()
            } else {
                let p = match SqlitePoolOptions::new().max_connections(1).connect(&dsn).await {
                    Ok(p) => p,
                    Err(e) => return Err(format!("SQLite Connect Error: {}", e)),
                };
                pools.insert(dsn.clone(), p.clone());
                p
            }
        };

        // 1. Get Tables
        let table_query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let rows = sqlx::query(table_query).fetch_all(&pool).await
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
        let dsn = format!("postgres://{}:{}@{}/{}", conn.user, conn.password, conn.host, conn.db_name);
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

        let rows = sqlx::query(schema_query).fetch_all(&pool).await
            .map_err(|e| format!("Failed to fetch schema: {}", e))?;
            
        for row in rows {
            let table: String = row.get("table_name");
            let col: String = row.get("column_name");
            schema_map.entry(table).or_default().push(col);
        }
    }
    
    Ok(schema_map)
}
