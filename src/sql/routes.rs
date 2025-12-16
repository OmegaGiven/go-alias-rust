use actix_web::{get, post, web, HttpResponse, Responder};
use actix_web::web::{Data, Form, Json};
use std::{collections::HashMap, sync::Arc, fs, io};
use serde::{Deserialize, Serialize};
use crate::app_state::AppState;
use crate::base_page::render_base_page;
use crate::sql::{
    DbConnection, SqlForm, AddConnForm,
    find_connection, render_table,
    encrypt_and_save, load_and_decrypt,
};

// --- NEW: Saved Query Structures and Persistence ---
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
    connection: String, // Added to handle redirect back to view
}

#[derive(Deserialize)]
struct DeleteQueryForm {
    query_name: String,
    connection: String, // Added to handle redirect back to view
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

// Helper to remove a query by name
fn delete_query(name: &str) -> io::Result<()> {
    let mut queries = load_queries();
    if let Some(pos) = queries.iter().position(|q| q.name == name) {
        queries.remove(pos);
        save_queries(&queries)?;
    }
    Ok(())
}
// --- END: Saved Query Structures and Persistence ---


// Helper function to render the connection list page content
fn render_connection_list(conns: &[DbConnection], current_theme: &crate::app_state::Theme) -> String {
    let conn_links = conns.iter()
        .map(|c| format!(
            r#"<li><a href="/sql/{nick}">{nick} ({db}@{host})</a></li>"#,
            nick = htmlescape::encode_minimal(&c.nickname),
            db = htmlescape::encode_minimal(&c.db_name),
            host = htmlescape::encode_minimal(&c.host)
        ))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(r#"
    <div class="sql-connections-page">
        <h1>SQL Connection Manager</h1>
        
        <div class="connection-form-container">
            <h2>Add New / Update Connection</h2>
            <form method="POST" action="/sql/add" class="connection-form">
              <input name="nickname" placeholder="Nickname (e.g., prod_db)" required>
              <input name="host" placeholder="Host (e.g., localhost:5432)" required>
              <input name="db_name" placeholder="Database Name" required>
              <input name="user" placeholder="User" required>
              <input name="password" type="password" placeholder="Password" required>
              <button type="submit">Save Connection</button>
            </form>
        </div>
        
        <div class="saved-connections-list">
            <h2>Saved Connections</h2>
            <ul>{conn_links}</ul>
        </div>
    </div>
    <style>
        .sql-connections-page {{
            max-width: 800px;
            margin: 0 auto;
        }}
        .connection-form-container {{
            background-color: var(--secondary-bg);
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
            border: 1px solid var(--border-color);
        }}
        .connection-form input {{
            width: 100%;
            padding: 10px;
            margin-bottom: 10px;
            box-sizing: border-box;
            background-color: var(--primary-bg);
            color: var(--text-color);
            border: 1px solid var(--border-color);
            border-radius: 4px;
        }}
        .connection-form button {{
            width: 100%;
            padding: 10px;
        }}
        .saved-connections-list ul {{
            list-style-type: none;
            padding: 0;
        }}
        .saved-connections-list li {{
            background-color: var(--tertiary-bg);
            margin: 5px 0;
            padding: 10px;
            border-radius: 4px;
        }}
    </style>
    "#, conn_links = conn_links);
    
    render_base_page("SQL Connections", &content, current_theme)
}


#[get("/sql")]
pub async fn sql_get(state: Data<Arc<AppState>>) -> impl Responder {
    {
        let mut conns = state.connections.lock().unwrap();
        if conns.is_empty() {
            *conns = load_and_decrypt();
        }
    }
    let conns = state.connections.lock().unwrap().clone();
    let current_theme = state.current_theme.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_connection_list(&conns, &current_theme))
}

#[post("/sql/add")]
pub async fn sql_add(form: Form<AddConnForm>, state: Data<Arc<AppState>>) -> impl Responder {
    let new_conn = DbConnection {
        host: form.host.clone(),
        db_name: form.db_name.clone(),
        user: form.user.clone(),
        password: form.password.clone(),
        nickname: form.nickname.clone(),
    };
    {
        let mut conns = state.connections.lock().unwrap();
        if let Some(idx) = conns.iter().position(|c| c.nickname == new_conn.nickname) {
            conns[idx] = new_conn;
        } else {
            conns.push(new_conn);
        }
        if let Err(e) = encrypt_and_save(&conns) {
            eprintln!("Failed to save encrypted connections: {e}");
        }
    }
    HttpResponse::Found().append_header(("Location", "/sql")).finish()
}

#[post("/sql/save")]
pub async fn sql_save(form: Form<SaveQueryForm>) -> impl Responder {
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

// --- NEW HANDLER: Delete SQL Query ---
#[post("/sql/delete")]
pub async fn sql_delete(form: Form<DeleteQueryForm>) -> impl Responder {
    if let Err(e) = delete_query(&form.query_name) {
        eprintln!("Failed to delete query: {e}");
    }
    
    // Redirect back to the specific connection view
    let location = format!("/sql/{}", form.connection);
    HttpResponse::Found().append_header(("Location", location)).finish()
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
pub async fn sql_run(form: Json<SqlForm>, state: Data<Arc<AppState>>) -> impl Responder {
    // Import TypeInfo to check column types manually
    use sqlx::{Row, Column, TypeInfo, postgres::PgPoolOptions, ValueRef, types::JsonValue}; 
    use std::convert::TryInto; 

    let conn_opt = {
        let conns = state.connections.lock().unwrap();
        find_connection(&form.connection, &conns).cloned()
    };

    if conn_opt.is_none() {
        return HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(format!("<div style=\"color:var(--link-hover);\">Error: Connection '{}' not found.</div>", htmlescape::encode_minimal(&form.connection)));
    }

    let conn = conn_opt.unwrap();
    let dsn = format!("postgres://{}:{}@{}/{}", conn.user, conn.password, conn.host, conn.db_name);
    let pool = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(format!("<div style=\"color:var(--link-hover);\">DB connect error: {}</div>", htmlescape::encode_minimal(&e.to_string())));
        }
    };

    // --- Variable Substitution ---
    let mut final_sql = form.sql.clone();
    if let Some(vars) = &form.variables {
        for (key, val) in vars {
            // Replace {{key}} with val
            let placeholder = format!("{{{{{}}}}}", key);
            final_sql = final_sql.replace(&placeholder, val);
        }
    }

    let rows = match sqlx::query(&final_sql).fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(format!("<div style=\"color:var(--link-hover);\">Query error: {}</div></div>", htmlescape::encode_minimal(&e.to_string())));
        }
    };

    let headers: Vec<String> = rows.get(0)
        .map(|row| row.columns().iter().map(|col| col.name().to_string()).collect())
        .unwrap_or_default();

    let mut data_rows: Vec<Vec<String>> = Vec::new();
    let mut results_vec_for_export: Vec<HashMap<String, String>> = Vec::new();
    
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

            // 2. Handle specific types manually via raw bytes
            if let Ok(raw_val) = row.try_get_raw(idx) {
                if raw_val.is_null() {
                    return "".to_string();
                }

                if let Ok(bytes) = raw_val.as_bytes() {
                    match type_name {
                        "TIMESTAMPTZ" | "TIMESTAMP" => {
                            // 8 bytes: int64 microseconds since 2000-01-01
                            if bytes.len() == 8 {
                                let micros = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                // Convert Postgres epoch (2000-01-01) to Unix epoch
                                let seconds = (micros / 1_000_000) + 946_684_800; 
                                // Use the helper to format it to "YYYY-MM-DD HH:MM:SS"
                                return format_ts(seconds);
                            }
                        },
                        "DATE" => {
                            // 4 bytes: int32 days since 2000-01-01
                            if bytes.len() == 4 {
                                let days = i32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]));
                                let seconds = (days as i64) * 86400 + 946_684_800;
                                // Format showing only date part
                                return format_ts(seconds).split_whitespace().next().unwrap_or("").to_string();
                            }
                        },
                        "UUID" => {
                            // 16 bytes
                            if bytes.len() == 16 {
                                let b = bytes;
                                return format!("{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                                    b[0],b[1],b[2],b[3], b[4],b[5], b[6],b[7], b[8],b[9], b[10],b[11],b[12],b[13],b[14],b[15]);
                            }
                        },
                        "BOOL" | "BOOL[]" => {
                             // 1 byte
                             if !bytes.is_empty() {
                                 return if bytes[0] != 0 { "true".to_string() } else { "false".to_string() };
                             }
                        },
                        "MONEY" => {
                            // 8 bytes: int64 cents
                            if bytes.len() == 8 {
                                let cents = i64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));
                                return format!("${:.2}", cents as f64 / 100.0);
                            }
                        },
                        _ => {
                            // Generic UTF-8 Fallback: If bytes are valid UTF-8, show them.
                            // This handles CITEXT, NAME, BPCHAR, XML, etc.
                            if let Ok(s) = std::str::from_utf8(bytes) {
                                return s.to_string();
                            }
                        }
                    }
                }
            }

            // 3. Try generic primitive decoding
            if let Ok(i) = row.try_get::<i32, usize>(idx) { return i.to_string(); }
            if let Ok(i) = row.try_get::<i16, usize>(idx) { return i.to_string(); }
            if let Ok(i) = row.try_get::<i64, usize>(idx) { return i.to_string(); }
            
            // Floats
            if let Ok(f) = row.try_get::<f64, usize>(idx) { return f.to_string(); }
            
            // Booleans
            if let Ok(b) = row.try_get::<bool, usize>(idx) { return b.to_string(); }
            
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
pub async fn sql_export(state: Data<Arc<AppState>>) -> impl Responder {
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

// Helper function to render the SQL query view page content
fn render_query_view(nickname: &str, table_schema_json: &str, current_theme: &crate::app_state::Theme) -> String {
    let saved_queries = load_queries();
    let nickname_safe = htmlescape::encode_minimal(nickname);
    
    let saved_query_list = saved_queries.iter()
        .map(|q| {
            let sql_safe = htmlescape::encode_minimal(&q.sql);
            let name_safe = htmlescape::encode_minimal(&q.name);
            
            // Layout change: 'x' button on the left, then the name
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

    // Using r###" to avoid termination on "#" in html
    let page_styles = r###"
<style>
    .sql-view-container { display: flex; height: calc(100vh - 60px); position: relative; overflow: hidden; }
    
    #sidebar { 
        width: 250px; 
        min-width: 0; 
        background: var(--secondary-bg); 
        color: var(--text-color); 
        padding: 5px; 
        overflow-y: auto; 
        flex-shrink: 0; 
        font-size: 0.9em; 
    }
    
    #sidebar.collapsed {
        width: 0 !important;
        padding: 0 !important;
        overflow: hidden;
    }

    #sidebar-resizer {
        width: 5px;
        background-color: var(--tertiary-bg);
        border-left: 1px solid var(--border-color);
        border-right: 1px solid var(--border-color);
        cursor: col-resize;
        flex-shrink: 0;
        z-index: 100;
        transition: background-color 0.2s;
    }
    
    #sidebar-resizer:hover, #sidebar-resizer.resizing {
        background-color: var(--link-hover);
    }

    #output-resizer {
        height: 5px;
        background-color: var(--tertiary-bg);
        border-top: 1px solid var(--border-color);
        border-bottom: 1px solid var(--border-color);
        cursor: row-resize;
        flex-shrink: 0;
        z-index: 10;
        transition: background-color 0.2s;
    }
    #output-resizer:hover, #output-resizer.resizing {
        background-color: var(--link-hover);
    }

    #sidebar h2 { margin: 5px 0 2px 0; padding-bottom: 2px; border-bottom: 1px solid var(--border-color); font-size: 1.1em; white-space: nowrap; overflow: hidden; }
    #sidebar ul { list-style: none; padding: 0; margin: 0; }
    #sidebar li { padding: 1px 0; cursor: pointer; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    
    .saved-query-item { display: flex; align-items: center; padding-left: 2px; }
    .delete-query-form { margin: 0; display: inline-flex; }
    .query-link { flex-grow: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-left: 5px; text-decoration: none; color: var(--text-color); font-size: 0.9em; opacity: 0.9; }
    .query-link:hover { opacity: 1; text-decoration: underline; color: var(--link-hover); }
    
    .delete-btn { background: none; border: none; color: #666; font-weight: bold; padding: 0 5px; margin: 0; cursor: pointer; width: 20px; text-align: center; font-size: 1em;}
    .delete-btn:hover { color: #ff3b3b; background: rgba(255,0,0,0.1); border-radius: 3px; }
    
    .sidebar-search input { width: 100%; padding: 4px; margin-bottom: 5px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; font-size: 0.9em; }
    
    .table-list-item { padding-left: 5px; display: block; color: var(--text-color); text-decoration: none; }
    .table-list-item:hover { color: var(--link-hover); background-color: var(--tertiary-bg); border-radius: 2px;}
    
    .query-save-form { margin-top: 10px; padding-top: 5px; border-top: 1px solid var(--border-color); }
    .query-save-form input[type="text"] { width: 100%; padding: 4px; margin-bottom: 5px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; font-size: 0.9em; }
    .query-save-form button { width: 100%; padding: 4px; cursor: pointer; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; font-size: 0.9em;}
    .query-save-form button:hover { background: var(--link-hover); color: #fff; border-color: var(--link-hover); }
    
    #toggle-arrow { position: absolute; top: 10px; left: 250px; cursor: pointer; font-size: 14px; user-select: none; background: var(--tertiary-bg); color: var(--text-color); padding: 6px 2px; border-radius: 0 4px 4px 0; transition: left 0.3s, background-color 0.2s; line-height: 1; z-index: 10; border: 1px solid var(--border-color); border-left: none; }
    #toggle-arrow:hover { background: var(--border-color); }
    
    #main { flex: 1; display: flex; flex-direction: column; padding: 0; overflow: hidden; }
    #sql-form { display: flex; flex-direction: column; flex-grow: 1; height: 100%; }
    
    .variables-section { padding: 5px; background: var(--secondary-bg); border-bottom: 1px solid var(--border-color); display: flex; flex-wrap: wrap; gap: 5px; align-items: center; flex-shrink: 0; }
    .var-input-group { display: flex; align-items: center; gap: 5px; background: var(--tertiary-bg);height: 26px; border-radius: 4px; border: 1px solid var(--border-color); box-sizing: border-box; }
    .var-input-group label { font-size: 0.8em; color: var(--text-color); font-weight: bold; white-space: nowrap; }
    .var-input-group input { padding: 2px 2px; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 2px; font-size: 0.9em; width: 100px; height: 20px; box-sizing: border-box; }
    .add-var-btn { padding: 0 8px; height: 26px; font-size: 0.8em; cursor: pointer; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; line-height: 24px; box-sizing: border-box; }
    .add-var-btn:hover { background: var(--link-hover); color: white; border-color: var(--link-hover); }

    .editor-container { 
        flex: 1; 
        min-height: 100px; 
        margin-bottom: 0; 
        position: relative; 
        display: flex; 
        flex-direction: column; 
        border-bottom: none; 
        background-color: var(--primary-bg);
        padding: 0px;
    }
    
    .editor-layer {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        margin: 0;
        padding: 5px; /* Minimal padding */
        border: none;
        font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
        font-size: 14px;
        line-height: 1.5;
        box-sizing: border-box;
        overflow: auto;
        white-space: pre-wrap;
        word-wrap: break-word;
    }

    #sql-backdrop {
        z-index: 1;
        pointer-events: none;
        background-color: transparent;
        color: var(--text-color);
    }

    #sql-editor {
        z-index: 2;
        background: transparent;
        color: transparent; /* Hide text, show caret */
        caret-color: var(--text-color);
        resize: none;
        outline: none;
    }
    
    /* Syntax Highlighting Colors */
    .hl-keyword { color: #ff79c6; font-weight: bold; } /* Pink/Purple */
    .hl-string { color: #f1fa8c; } /* Yellow */
    .hl-number { color: #bd93f9; } /* Purple */
    .hl-comment { color: #6272a4; } /* Grey/Blue */
    
    .action-bar { padding: 5px; background: var(--secondary-bg); border-top: 1px solid var(--border-color); display: flex; gap: 10px; flex-shrink: 0; align-items: center; }
    .action-bar button { margin: 0; cursor: pointer; padding: 5px 15px; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; font-weight: bold; font-size: 0.9em; }
    .action-bar button:hover { background: var(--link-hover); color: #fff; border-color: var(--link-hover); }
    
    .output { 
        height: 300px;
        flex-shrink: 0;
        overflow: auto; 
        background: var(--primary-bg); 
        padding: 0; /* Remove padding from container to maximize space */
        margin-top: 0; 
        font-family: monospace; 
        font-size: 0.9em; 
    }
    .output table { width: 100%; border-collapse: collapse; }
    .output th, .output td { border: 1px solid var(--border-color); padding: 4px 8px; text-align: left; white-space: nowrap; }
    .output th { background: var(--tertiary-bg); position: sticky; top: 0; z-index: 1; }
    .output tr:nth-child(even) { background-color: rgba(255,255,255,0.02); }
    .output pre { padding: 5px; margin: 0; }
    
    /* Autocomplete Suggestions */
    #autocomplete-list { 
        position: absolute; 
        border: 1px solid var(--border-color); 
        background: var(--tertiary-bg); 
        z-index: 1000; 
        max-height: 200px; 
        overflow-y: auto; 
        display: none; 
        box-shadow: 0 4px 12px rgba(0,0,0,0.5);
        border-radius: 4px;
        min-width: 200px;
    }
    #autocomplete-list div { padding: 4px 8px; cursor: pointer; border-bottom: 1px solid var(--border-color); font-size: 0.9em; color: var(--text-color); }
    #autocomplete-list div:last-child { border-bottom: none; }
    #autocomplete-list div:hover, #autocomplete-list div.autocomplete-active { 
        background-color: var(--link-hover); 
        color: white; 
    }
    #autocomplete-list strong { color: #49cc90; }
    #autocomplete-list div.autocomplete-active strong { color: #fff; }
</style>
"###;

    let body_content = format!(r###"
    <div class="sql-view-container">
      <div id="sidebar">
        <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        <ul id="table-list"></ul>
    
        <div class="sidebar-search"><input type="text" id="query-search-input" placeholder="Search queries..."></div>
        <ul id="saved-queries-list">{saved_query_list}</ul>
        
        <form id="save-query-form" method="POST" action="/sql/save" class="query-save-form">
            <input type="text" id="query-name" name="query_name" placeholder="Name query to save" required>
            <input type="hidden" id="query-sql" name="sql">
            <input type="hidden" name="connection" value="{nickname}">
            <button type="submit">Save Current Query</button>
        </form>
      </div>
      
      <div id="sidebar-resizer" title="Drag to resize, Click to toggle sidebar"></div>
      
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
            <a href="/sql/export" target="_blank"><button type="button">Export CSV</button></a>
            <button type="button" id="save-sql-file-btn">Save SQL File</button>
          </div>
        </form>
        
        <div id="output-resizer" title="Drag to resize output"></div>
        <div class="output" id="output"><pre>Click a table name or enter a query and press 'Run Query'.</pre></div>
      </div>
    </div>
    
    <!-- Autocomplete container attached to body for proper floating behavior -->
    <div id="autocomplete-list"></div>
    
    <script>
      // Schema Data injected from Rust
      const dbSchema = {table_schema_json};
      
      const sidebar = document.getElementById('sidebar');
      const resizer = document.getElementById('sidebar-resizer');
      let isResizing = false;
      let lastDownX = 0;
      let savedSidebarWidth = 250; // Default width

      const mainContent = document.getElementById('main');
      const editor = document.getElementById('sql-editor');
      const sidebarSearchInput = document.getElementById('sidebar-search-input');
      const sidebarTableList = document.getElementById('table-list');
      const querySearchInput = document.getElementById('query-search-input');
      const savedQueriesList = document.getElementById('saved-queries-list');
      const saveQueryForm = document.getElementById('save-query-form');
      const queryNameInput = document.getElementById('query-name');
      const querySqlInput = document.getElementById('query-sql');
      const variablesSection = document.getElementById('variables-section');
      const autocompleteList = document.getElementById('autocomplete-list');
      const saveSqlFileBtn = document.getElementById('save-sql-file-btn');
      let currentFocus = -1;
      
      // Highlighting Elements
      const backdrop = document.getElementById('sql-backdrop');
      const highlights = backdrop.querySelector('.highlights');

      // --- Save SQL File ---
      saveSqlFileBtn.addEventListener('click', () => {{
          const content = editor.value;
          if (!content) return;
          
          const blob = new Blob([content], {{ type: 'text/plain' }});
          const link = document.createElement('a');
          link.href = URL.createObjectURL(blob);
          
          // Try to use the saved query name if available, else default
          let filename = queryNameInput.value.trim() || 'query';
          if (!filename.toLowerCase().endsWith('.sql')) filename += '.sql';
          
          link.download = filename;
          link.click();
          URL.revokeObjectURL(link.href);
      }});

      // --- Sidebar Toggle Button (if exists) ---
      const toggleArrow = document.getElementById('toggle-arrow');
      if(toggleArrow) {{
          toggleArrow.addEventListener('click', () => {{
            if (!sidebar.classList.contains('collapsed')) {{
              sidebar.classList.add('collapsed');
              sidebar.style.width = '';
            }} else {{
              sidebar.classList.remove('collapsed');
              sidebar.style.width = savedSidebarWidth + 'px';
            }}
          }});
      }}

      // --- Resizer Logic ---
      resizer.addEventListener('mousedown', (e) => {{
          isResizing = true;
          lastDownX = e.clientX;
          resizer.classList.add('resizing');
          document.body.style.cursor = 'col-resize';
          document.body.style.userSelect = 'none'; 
      }});

      document.addEventListener('mousemove', (e) => {{
          if (!isResizing) return;
          let newWidth = e.clientX;
          if (newWidth < 10) newWidth = 0; // Snap close
          if (newWidth > 600) newWidth = 600; // Max width
          
          if (newWidth === 0) {{
             sidebar.classList.add('collapsed');
             sidebar.style.width = '';
          }} else {{
             sidebar.classList.remove('collapsed');
             sidebar.style.width = newWidth + 'px';
          }}
      }});

      document.addEventListener('mouseup', (e) => {{
          if (!isResizing) return;
          isResizing = false;
          resizer.classList.remove('resizing');
          document.body.style.cursor = '';
          document.body.style.userSelect = '';
          
          // Click detection: if moved less than 5px, toggle
          if (Math.abs(e.clientX - lastDownX) < 5) {{
              toggleSidebar();
          }} else {{
              if (sidebar.offsetWidth > 0) {{
                  savedSidebarWidth = sidebar.offsetWidth;
              }}
          }}
      }});

      function toggleSidebar() {{
          if (sidebar.offsetWidth === 0 || sidebar.classList.contains('collapsed')) {{
              sidebar.classList.remove('collapsed');
              sidebar.style.width = savedSidebarWidth + 'px';
          }} else {{
              savedSidebarWidth = sidebar.offsetWidth;
              sidebar.classList.add('collapsed');
              sidebar.style.width = '';
          }}
      }}
      
      // --- Output Resizer Logic ---
      const outputResizer = document.getElementById('output-resizer');
      const outputPane = document.getElementById('output');
      let isOutputResizing = false;
      let lastOutputDownY = 0;
      let startOutputHeight = 0;

      outputResizer.addEventListener('mousedown', (e) => {{
          isOutputResizing = true;
          lastOutputDownY = e.clientY;
          startOutputHeight = outputPane.offsetHeight;
          outputResizer.classList.add('resizing');
          document.body.style.cursor = 'row-resize';
          document.body.style.userSelect = 'none';
      }});

      document.addEventListener('mousemove', (e) => {{
          if (isOutputResizing) {{
              const dy = lastOutputDownY - e.clientY; // Drag up increases height
              let newHeight = startOutputHeight + dy;
              if (newHeight < 50) newHeight = 50; 
              
              const containerHeight = mainContent.clientHeight;
              if (newHeight > containerHeight - 100) newHeight = containerHeight - 100;
              
              outputPane.style.height = newHeight + 'px';
          }}
      }});

      document.addEventListener('mouseup', (e) => {{
          if (isOutputResizing) {{
              isOutputResizing = false;
              outputResizer.classList.remove('resizing');
              document.body.style.cursor = '';
              document.body.style.userSelect = '';
          }}
      }});

      // --- Render Sidebar Table List ---
      function renderTableList() {{
          const filter = sidebarSearchInput.value.toUpperCase();
          sidebarTableList.innerHTML = '';
          
          Object.keys(dbSchema).sort().forEach(tableName => {{
              if (tableName.toUpperCase().indexOf(filter) > -1) {{
                  const li = document.createElement('li');
                  // Create link for better semantics and hover effect
                  const a = document.createElement('a');
                  a.className = 'table-list-item';
                  a.textContent = tableName;
                  a.href = '#';
                  a.title = "Click to SELECT * LIMIT 100";
                  a.onclick = (e) => {{ e.preventDefault(); editor.value = "SELECT * FROM \\\"" + tableName + "\\\" LIMIT 100;"; handleInput(); }};
                  li.appendChild(a);
                  sidebarTableList.appendChild(li);
              }}
          }});
      }}
      renderTableList();
      sidebarSearchInput.addEventListener('keyup', renderTableList);

      // --- Saved Queries Filter ---
      function filterSavedQueries() {{
          const filter = querySearchInput.value.toUpperCase();
          const listItems = savedQueriesList.getElementsByTagName('li');
          for (let i = 0; i < listItems.length; i++) {{
              const itemText = listItems[i].querySelector('.query-link').textContent || listItems[i].querySelector('.query-link').innerText;
              if (itemText.toUpperCase().indexOf(filter) > -1) {{ listItems[i].style.display = 'flex'; }} else {{ listItems[i].style.display = 'none'; }}
          }}
      }}
      querySearchInput.addEventListener('keyup', filterSavedQueries);

      // --- Form Submission (Run Query) ---
      const form = document.getElementById('sql-form');
      const output = document.getElementById('output');
      
      form.addEventListener('submit', async (e) => {{
        e.preventDefault();
        output.innerHTML = '<pre style="padding:10px;">Loading...</pre>';
        
        const variables = {{}};
        const varInputs = variablesSection.querySelectorAll('input');
        varInputs.forEach(input => {{
            if(input.name && input.value) {{
                variables[input.name] = input.value;
            }}
        }});

        const payload = {{
            sql: editor.value,
            connection: form.querySelector('input[name="connection"]').value,
            variables: variables
        }};

        try {{
            const resp = await fetch('/sql/run', {{ 
                method: 'POST', 
                headers: {{ 'Content-Type': 'application/json' }}, 
                body: JSON.stringify(payload) 
            }});
            const html = await resp.text();
            output.innerHTML = html;
        }} catch(e) {{
            output.innerHTML = '<pre style="padding:10px; color:#ff6b6b;">Error: ' + e.message + '</pre>';
        }}
      }});

      // --- Load Saved Query ---
      savedQueriesList.addEventListener('click', (e) => {{
          const target = e.target.closest('a');
          if (target) {{ 
              e.preventDefault(); 
              const sql = target.getAttribute('data-sql'); 
              const name = target.getAttribute('data-name'); 
              editor.value = sql; 
              queryNameInput.value = name; 
              scanForVariables(); // Update inputs
              handleInput(); // Trigger Highlight
          }}
      }});

      saveQueryForm.addEventListener('submit', (e) => {{
          querySqlInput.value = editor.value;
          if (queryNameInput.value.trim() === '') {{ e.preventDefault(); }}
      }});

      // --- Variables Logic ---
      function addVariable(name = '', value = '') {{
          const div = document.createElement('div');
          div.className = 'var-input-group';
          const label = document.createElement('label');
          label.innerText = name || 'New Var';
          const input = document.createElement('input');
          input.type = 'text';
          input.name = name;
          input.value = value;
          input.placeholder = 'Value';
          
          if(!name) {{
             input.placeholder = 'Name';
             input.onchange = (e) => {{ input.name = e.target.value; label.innerText = e.target.value; }};
          }}
          
          div.appendChild(label);
          div.appendChild(input);
          
          // Insert before the button
          const btn = variablesSection.querySelector('.add-var-btn');
          variablesSection.insertBefore(div, btn);
      }}
      window.addVariable = addVariable;

      function scanForVariables() {{
          const regex = /{{{{([^}}]+)}}}}/g;
          const text = editor.value;
          let match;
          const foundVars = new Set();
          
          while ((match = regex.exec(text)) !== null) {{
              foundVars.add(match[1]);
          }}
          
          const currentInputs = Array.from(variablesSection.querySelectorAll('input'));
          const currentValues = {{}};
          currentInputs.forEach(i => currentValues[i.name] = i.value);
          
          // Clear existing vars but keep button
          const existingGroups = variablesSection.querySelectorAll('.var-input-group');
          existingGroups.forEach(g => g.remove());
          
          foundVars.forEach(v => {{
              addVariable(v, currentValues[v] || '');
          }});
      }}
      
      // --- Syntax Highlighting Logic ---
      const escapeHtml = (unsafe) => {{
          return unsafe
               .replace(/&/g, "&amp;")
               .replace(/</g, "&lt;")
               .replace(/>/g, "&gt;")
               .replace(/"/g, "&quot;");
      }};

      const applyHighlights = (text) => {{
          let html = escapeHtml(text);
          
          const tokens = [];
          const pushToken = (text, type) => {{
              const id = "___TOKEN" + tokens.length + "___";
              tokens.push({{ id, text, type }});
              return id;
          }};
          
          // 1. Hide comments
          html = html.replace(/(--.*$)/gm, (m) => pushToken(m, 'hl-comment'));
          
          // 2. Hide strings
          html = html.replace(/('([^'\\]|\\.)*')/g, (m) => pushToken(m, 'hl-string'));
          
          // 3. Highlight Keywords (Case Insensitive)
          const keywords = ["SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "JOIN", "INNER", "OUTER", "LEFT", "RIGHT", "ON", "GROUP", "BY", "ORDER", "LIMIT", "OFFSET", "AND", "OR", "NOT", "NULL", "AS", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN", "LIKE", "ILIKE", "IN", "IS", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END", "HAVING", "UNION", "ALL"];
          
          const rxKeyword = new RegExp(`\\b(${{keywords.join('|')}})\\b`, 'gi');
          html = html.replace(rxKeyword, '<span class="hl-keyword">$1</span>');
          
          // 4. Highlight Numbers
          html = html.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');
          
          // 5. Restore tokens
          tokens.forEach(t => {{
              html = html.replace(t.id, `<span class="${{t.type}}">${{t.text}}</span>`);
          }});
          
          if (text[text.length-1] === "\n") {{
              html += " "; 
          }}
          
          return html;
      }};

      const handleInput = () => {{
          const text = editor.value;
          highlights.innerHTML = applyHighlights(text);
          scanForVariables();
      }};

      const syncScroll = () => {{
          backdrop.scrollTop = editor.scrollTop;
          backdrop.scrollLeft = editor.scrollLeft;
      }};

      editor.addEventListener('input', handleInput);
      editor.addEventListener('scroll', syncScroll);
      if (editor.value) handleInput();


      // --- Autocomplete Helper: Get Caret Coordinates ---
      function getCaretCoordinates() {{
        const div = document.createElement('div');
        const style = window.getComputedStyle(editor);
        for (const prop of style) {{
          div.style[prop] = style.getPropertyValue(prop);
        }}
        div.style.position = 'absolute';
        div.style.top = '0';
        div.style.left = '0';
        div.style.visibility = 'hidden';
        div.style.height = 'auto';
        div.style.width = editor.offsetWidth + 'px';
        div.style.overflow = 'hidden';
        div.style.whiteSpace = 'pre-wrap';

        const text = editor.value.substring(0, editor.selectionStart);
        div.textContent = text;
        const span = document.createElement('span');
        span.textContent = '.';
        div.appendChild(span);
        
        document.body.appendChild(div);
        
        const coordinates = {{
          top: span.offsetTop + parseInt(style.borderTopWidth) + parseInt(style.paddingTop) - editor.scrollTop,
          left: span.offsetLeft + parseInt(style.borderLeftWidth) + parseInt(style.paddingLeft) - editor.scrollLeft,
          lineHeight: parseInt(style.lineHeight) || 20 
        }};
        document.body.removeChild(div);
        return coordinates;
      }}

      // --- Autocomplete Logic ---
      editor.addEventListener('input', function(e) {{
          const val = this.value;
          const cursorPosition = this.selectionStart;
          const textBeforeCursor = val.substring(0, cursorPosition);
          
          const words = textBeforeCursor.split(/[\s,()]+/);
          const currentWord = words[words.length - 1];
          
          if (!currentWord) {{
              closeAutocomplete();
              return;
          }}

          let matches = [];
          
          // Case 1: Dot notation (Table.Column)
          if (currentWord.includes('.')) {{
              const parts = currentWord.split('.');
              const tableName = parts[0];
              const colPrefix = parts[1] || '';
              
              const realTableName = Object.keys(dbSchema).find(t => t.toUpperCase() === tableName.toUpperCase());
              
              if (realTableName && dbSchema[realTableName]) {{
                  matches = dbSchema[realTableName]
                      .filter(col => col.toUpperCase().startsWith(colPrefix.toUpperCase()))
                      .map(col => ({{ display: col, insert: col, type: 'column' }}));
              }}
          }} 
          // Case 2: Table Suggestions
          else {{
              matches = Object.keys(dbSchema)
                  .filter(t => t.toUpperCase().startsWith(currentWord.toUpperCase()))
                  .map(t => ({{ display: t, insert: t, type: 'table' }}));
          }}

          if (matches.length > 0) {{
              currentFocus = -1;
              showAutocomplete(matches, currentWord);
          }} else {{
              closeAutocomplete();
          }}
      }});

      function showAutocomplete(matches, currentWord) {{
          autocompleteList.innerHTML = "";
          const coords = getCaretCoordinates();
          const rect = editor.getBoundingClientRect();
          
          autocompleteList.style.display = "block";
          autocompleteList.style.left = (rect.left + coords.left + window.scrollX) + "px";
          autocompleteList.style.top = (rect.top + coords.top + coords.lineHeight + window.scrollY) + "px";
          
          matches.forEach(match => {{
              const div = document.createElement("div");
              div.innerHTML = `<strong>${{match.display.substr(0, currentWord.length)}}</strong>${{match.display.substr(currentWord.length)}} <small style='float:right; opacity:0.6;'>${{match.type}}</small>`;
              div.addEventListener("click", function(e) {{
                  insertAtCursor(editor, match.insert, currentWord);
                  closeAutocomplete();
              }});
              autocompleteList.appendChild(div);
          }});
      }}

      function closeAutocomplete() {{
          autocompleteList.innerHTML = "";
          autocompleteList.style.display = "none";
      }}
      
      document.addEventListener("click", function (e) {{
          if (e.target !== editor) {{ closeAutocomplete(); }}
      }});

      editor.addEventListener('keydown', function(e) {{
          const list = document.getElementById('autocomplete-list');
          if (!list || list.style.display === 'none') return;
          
          const items = list.getElementsByTagName('div');
          
          if (e.key === 'ArrowDown') {{
              currentFocus++;
              addActive(items);
              e.preventDefault(); 
          }} else if (e.key === 'ArrowUp') {{
              currentFocus--;
              addActive(items);
              e.preventDefault(); 
          }} else if (e.key === 'Enter') {{
              e.preventDefault(); 
              if (currentFocus > -1) {{
                  if (items[currentFocus]) items[currentFocus].click();
              }}
          }} else if (e.key === 'Escape') {{
              closeAutocomplete();
          }}
      }});

      function addActive(items) {{
          if (!items) return;
          removeActive(items);
          if (currentFocus >= items.length) currentFocus = 0;
          if (currentFocus < 0) currentFocus = (items.length - 1);
          items[currentFocus].classList.add('autocomplete-active');
          items[currentFocus].scrollIntoView({{block: 'nearest'}});
      }}

      function removeActive(items) {{
          for (let i = 0; i < items.length; i++) {{
              items[i].classList.remove('autocomplete-active');
          }}
      }}

      function insertAtCursor(field, value, typedWord) {{
          let prefix = "";
          if (typedWord.includes('.')) {{
             prefix = typedWord.split('.')[0] + '.';
          }}
          
          const valToInsert = value; 
          
          const cursorPos = field.selectionStart;
          const textBefore = field.value.substring(0, cursorPos);
          const textAfter = field.value.substring(cursorPos);
          
          const cleanBefore = textBefore.substring(0, textBefore.length - (typedWord.length - prefix.length));
          
          field.value = cleanBefore + valToInsert + textAfter;
          field.selectionStart = field.selectionEnd = cleanBefore.length + valToInsert.length;
          field.focus();
          
          handleInput();
      }}

      if (editor.value === "") {{ editor.value = "SELECT 1;"; handleInput(); }}
    </script>
    "###, nickname = nickname_safe, table_schema_json = table_schema_json, saved_query_list = saved_query_list);

    render_base_page(
        &format!("SQL View: {}", nickname),
        &format!("{}{}", page_styles, body_content),
        current_theme
    )
}

#[get("/sql/{nickname}")]
pub async fn sql_view(path: web::Path<String>, state: web::Data<Arc<AppState>>) -> impl Responder {
    use sqlx::{Row, postgres::PgPoolOptions};

    let nickname = path.into_inner();
    let conn_opt = {
        let conns = state.connections.lock().unwrap();
        conns.iter().find(|c| c.nickname == nickname).cloned()
    };
    let conn = match conn_opt {
        Some(c) => c,
        None => {
            let current_theme = state.current_theme.lock().unwrap();
            let error_content = format!(r#"<h1>Error</h1><p>Connection '{nickname}' not found.</p>"#, nickname = htmlescape::encode_minimal(&nickname));
            return HttpResponse::BadRequest().body(render_base_page("Error", &error_content, &current_theme));
        }
    };

    let dsn = format!("postgres://{}:{}@{}/{}", conn.user, conn.password, conn.host, conn.db_name);
    let pool = match PgPoolOptions::new().max_connections(5).connect(&dsn).await {
        Ok(p) => p,
        Err(e) => {
            let current_theme = state.current_theme.lock().unwrap();
            let error_content = format!(r#"<h1>DB Connection Error</h1><pre class="error-message">Could not connect to {nickname}: {e}</pre>"#, nickname = htmlescape::encode_minimal(&nickname), e = htmlescape::encode_minimal(&e.to_string()));
            return HttpResponse::InternalServerError().body(render_base_page("Connection Error", &error_content, &current_theme));
        }
    };

    // --- NEW: Fetch Full Schema (Tables AND Columns) ---
    // Postgres specific query to get table and column names
    let schema_query = r#"
        SELECT table_name, column_name 
        FROM information_schema.columns 
        WHERE table_schema = 'public' 
        ORDER BY table_name, ordinal_position
    "#;

    let rows = match sqlx::query(schema_query).fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => {
            let current_theme = state.current_theme.lock().unwrap();
            let error_content = format!(r#"<h1>SQL Error</h1><pre class="error-message">Failed to fetch schema: {e}</pre>"#, e = htmlescape::encode_minimal(&e.to_string()));
            return HttpResponse::InternalServerError().body(render_base_page("SQL Error", &error_content, &current_theme));
        }
    };

    // Construct HashMap<TableName, Vec<ColumnName>>
    let mut schema_map: HashMap<String, Vec<String>> = HashMap::new();
    
    for row in rows {
        let table: String = row.get("table_name");
        let col: String = row.get("column_name");
        schema_map.entry(table).or_default().push(col);
    }

    let schema_json = serde_json::to_string(&schema_map).unwrap_or_else(|_| "{}".to_string());
        
    let current_theme = state.current_theme.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_query_view(&nickname, &schema_json, &current_theme))
}