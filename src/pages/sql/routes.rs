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
    connection: String, 
}

#[derive(Deserialize)]
struct DeleteQueryForm {
    query_name: String,
    connection: String, 
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
// --- END: Saved Query Structures and Persistence ---


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
                r#"<li><a href="/sql/{nick}">{display_text}</a></li>"#,
                nick = htmlescape::encode_minimal(&c.nickname),
                display_text = htmlescape::encode_minimal(&display_text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(r#"
    <div class="sql-connections-page">
        <h1>SQL Connection Manager</h1>
        
        <div class="forms-container">
            <!-- Left: Add Existing -->
            <div class="connection-form-container">
                <h2>Add Connection</h2>
                <form method="POST" action="/sql/add" class="connection-form">
                  
                  <label for="db_type" style="display:block; margin-bottom:5px;">Database Type:</label>
                  <select name="db_type" id="db_type" onchange="toggleFields()" style="margin-bottom:10px; width:100%; padding:10px;">
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
            </div>

            <!-- Right: Create New SQLite -->
            <div class="connection-form-container">
                <h2>Create New SQLite DB</h2>
                <form method="POST" action="/sql/add" class="connection-form" onsubmit="prepareCreate(event)">
                    <input type="hidden" name="db_type" value="sqlite">
                    <input type="hidden" name="db_name" value="">
                    <input type="hidden" name="user" value="">
                    <input type="hidden" name="password" value="">
                    <!-- These will be populated by JS -->
                    <input type="hidden" name="host" id="create_host">
                    <input type="hidden" name="nickname" id="create_nick">

                    <label style="display:block; margin-bottom:5px;">New Filename:</label>
                    <input id="new_filename" placeholder="e.g., my_new_project" required>
                    
                    <button type="submit" style="background-color: var(--link-color); color: var(--primary-bg); font-weight: bold;">Create & Save</button>
                    <p style="font-size:0.85em; opacity:0.8; margin-top:10px; line-height: 1.4;">
                        This will register a new SQLite database file. 
                        The file will be created automatically when you first open it.
                    </p>
                </form>
            </div>
        </div>
        
        <div class="saved-connections-list">
            <h2>Saved Connections</h2>
            <ul>{conn_links}</ul>
        </div>
    </div>
    
    <script>
        function toggleFields() {{
            const type = document.getElementById('db_type').value;
            const pgFields = document.getElementById('pg_fields');
            const hostInput = document.getElementById('host_input');
            const inputs = pgFields.querySelectorAll('input');

            if (type === 'sqlite') {{
                pgFields.style.display = 'none';
                hostInput.placeholder = "File Path (e.g., ./my_data.db)";
                inputs.forEach(i => i.removeAttribute('required'));
            }} else {{
                pgFields.style.display = 'block';
                hostInput.placeholder = "Host (e.g., localhost:5432)";
            }}
        }}
        
        function prepareCreate(e) {{
            const input = document.getElementById('new_filename');
            let val = input.value.trim();
            if (!val) {{ e.preventDefault(); return; }}
            
            // Auto append extension if missing
            if (!val.toLowerCase().endsWith('.db') && !val.toLowerCase().endsWith('.sqlite')) {{
                val += '.db';
            }}
            
            // Set hidden fields for the shared /sql/add endpoint
            document.getElementById('create_host').value = val;
            document.getElementById('create_nick').value = val;
        }}

        // Run on load
        toggleFields();
    </script>

    <style>
        .sql-connections-page {{
            max-width: 900px;
            margin: 0 auto;
        }}
        .forms-container {{
            display: flex;
            gap: 20px;
            flex-wrap: wrap;
            margin-bottom: 20px;
        }}
        .connection-form-container {{
            flex: 1;
            min-width: 300px;
            background-color: var(--secondary-bg);
            padding: 20px;
            border-radius: 8px;
            border: 1px solid var(--border-color);
        }}
        .connection-form input, .connection-form select {{
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
            cursor: pointer;
            background-color: var(--tertiary-bg);
            color: var(--text-color);
            border: 1px solid var(--border-color);
            border-radius: 4px;
        }}
        .connection-form button:hover {{
            background-color: var(--link-hover);
            color: #fff;
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

    let page_styles = r###"
<style>
    .sql-view-container { display: flex; height: calc(100vh - 60px); position: relative; overflow: hidden; }
    
    /* Output resizer style using shared class */
    #output-resizer {
        flex-shrink: 0;
    }

    /* SQL-specific sidebar content styles */
    #sidebar ul { list-style: none; padding: 0; margin: 0; }
    #sidebar li { padding: 1px 0; cursor: pointer; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    
    .saved-query-item { display: flex; align-items: center; padding-left: 2px; }
    .delete-query-form { margin: 0; display: inline-flex; }
    .query-link { flex-grow: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-left: 5px; text-decoration: none; color: var(--text-color); font-size: var(--font-size-small); opacity: 0.9; }
    .query-link:hover { opacity: 1; text-decoration: underline; color: var(--link-hover); }
    
    .delete-btn { background: none; border: none; color: #666; font-weight: bold; padding: 0 5px; margin: 0; cursor: pointer; width: 20px; text-align: center; font-size: 1em;}
    .delete-btn:hover { color: #ff3b3b; background: rgba(255,0,0,0.1); border-radius: 3px; }
    
    /* Sidebar search style removed - now in static/style.css */
    
    .table-list-item { padding-left: 5px; display: block; color: var(--text-color); text-decoration: none; }
    .table-list-item:hover { color: var(--link-hover); background-color: var(--tertiary-bg); border-radius: 2px;}
    
    .query-save-form { margin-top: 10px; padding-top: 5px; border-top: 1px solid var(--border-color); }
    .query-save-form input[type="text"] { width: 100%; padding: 4px; margin-bottom: 5px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; font-size: var(--font-size-small); }
    .query-save-form button { width: 100%; padding: 4px; cursor: pointer; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; font-size: var(--font-size-small);}
    .query-save-form button:hover { background: var(--link-hover); color: #fff; border-color: var(--link-hover); }
    
    #main { flex: 1; display: flex; flex-direction: column; padding: 0; overflow: hidden; }
    #sql-form { display: flex; flex-direction: column; flex-grow: 1; height: 100%; }
    
    .variables-section { padding: 5px; background: var(--secondary-bg); border-bottom: 1px solid var(--border-color); display: flex; flex-wrap: wrap; gap: 5px; align-items: center; flex-shrink: 0; }
    .var-input-group { display: flex; align-items: center; gap: 5px; background: var(--tertiary-bg); padding: 0 5px; height: 26px; border-radius: 4px; border: 1px solid var(--border-color); box-sizing: border-box; }
    .var-input-group label { font-size: 0.8em; color: var(--text-color); font-weight: bold; white-space: nowrap; }
    .var-input-group input { padding: 2px 4px; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 2px; font-size: var(--font-size-small); width: 100px; height: 20px; box-sizing: border-box; }
    .add-var-btn { padding: 0 8px; height: 26px; font-size: 0.8em; cursor: pointer; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; line-height: 24px; box-sizing: border-box; display: inline-flex; align-items: center; justify-content: center; margin: 0; }
    .add-var-btn:hover { background: var(--link-hover); color: white; border-color: var(--link-hover); }
    
    .var-del-btn { cursor: pointer; color: #888; font-weight: bold; margin-left: 5px; font-size: 1.1em; line-height: 1; }
    .var-del-btn:hover { color: #ff6b6b; }

    .editor-container { 
        flex: 1; 
        min-height: 100px; 
        margin-bottom: 0; 
        position: relative; 
        display: flex; 
        flex-direction: column; 
        border-bottom: none; 
        background-color: var(--primary-bg);
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
        font-size: var(--font-size-medium);
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
    
    .hl-keyword { color: #ff79c6; font-weight: bold; }
    .hl-string { color: #f1fa8c; }
    .hl-number { color: #bd93f9; }
    .hl-comment { color: #6272a4; }
    
    .action-bar { padding: 2px 5px; background: var(--secondary-bg); border-top: 1px solid var(--border-color); display: flex; gap: 10px; flex-shrink: 0; align-items: center; }
    .action-bar button { margin: 0; font-weight: bold; }
    
    /* Result Tools (Filter/Export) */
    .result-tools { padding: 4px 5px; background: var(--secondary-bg); border-bottom: none; display: flex; gap: 10px; align-items: center; flex-shrink: 0; }
    .result-tools input { padding: 3px 6px; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 3px; font-size: var(--font-size-small); flex-grow: 1; max-width: 300px;}
    .result-tools label { font-size: 0.85em; color: var(--text-color); display: flex; align-items: center; gap: 3px; cursor: pointer; }
    
    .output { 
        height: 300px;
        flex-shrink: 0;
        overflow: auto; 
        background: var(--primary-bg); 
        padding: 0; 
        margin-top: 0; 
        font-family: monospace; 
        font-size: var(--font-size-small); 
    }
    .output table { width: 100%; border-collapse: collapse; margin: 0; }
    .output th, .output td { border: 1px solid var(--border-color); padding: 4px 8px; text-align: left; white-space: nowrap; user-select: none; }
    .output th { background: var(--tertiary-bg); position: sticky; top: 0; z-index: 1; cursor: pointer; }
    .output tr:nth-child(even) { background-color: rgba(255,255,255,0.02); }
    .output tr.selected-row td { background-color: rgba(73, 204, 144, 0.4) !important; color: #fff; }
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

    // Using r###" to avoid termination on "#" in html
    let sidebar_content = format!(r###"
        <div style="display:flex; justify-content:space-between; align-items:center; border-bottom:1px solid var(--border-color); padding-bottom:2px; margin: 5px 0 2px 0;">
             <h2 style="margin:0; border:none;">Tables</h2>
             <button id="refresh-schema-btn" type="button" class="delete-btn" style="width:auto; font-size:1.2em;" title="Refresh Tables">&#x21bb;</button>
        </div>
        <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        <ul id="table-list"></ul>
        
        <h2 style="margin-top: 15px;">Saved Queries</h2>
        <div class="sidebar-search"><input type="text" id="query-search-input" placeholder="Search queries..."></div>
        <ul id="saved-queries-list">{saved_query_list}</ul>
        
        <form id="save-query-form" method="POST" action="/sql/save" class="query-save-form">
            <input type="text" id="query-name" name="query_name" placeholder="Name query to save" required>
            <input type="hidden" id="query-sql" name="sql">
            <input type="hidden" name="connection" value="{nickname}">
            <button type="submit">Save Current Query</button>
        </form>
    "###, saved_query_list = saved_query_list, nickname = nickname_safe);
    
    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);
    let sidebar_js = crate::elements::sidebar::get_js();

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
            <span id="row-count" style="font-size: 0.9em; margin: 0 10px; color: var(--text-color);"></span>
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
    
    <script>
      const dbSchema = {table_schema_json};
      
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
      
      const backdrop = document.getElementById('sql-backdrop');
      const highlights = backdrop.querySelector('.highlights');

      const connectionNickname = "{nickname}";
      const varsStorageKey = "sql_vars_" + connectionNickname;

      // --- Clear Button Logic ---
      const clearBtn = document.getElementById('clear-editor-btn');
      if (clearBtn) {{
          clearBtn.addEventListener('click', () => {{
              if(editor.value.trim() === '') return;
              editor.value = '';
              handleInput();
              editor.focus();
          }});
      }}

      saveSqlFileBtn.addEventListener('click', () => {{
          const content = editor.value;
          if (!content) return;
          
          const blob = new Blob([content], {{ type: 'text/plain' }});
          const link = document.createElement('a');
          link.href = URL.createObjectURL(blob);
          
          let filename = queryNameInput.value.trim() || 'query';
          if (!filename.toLowerCase().endsWith('.sql')) filename += '.sql';
          
          link.download = filename;
          link.click();
          URL.revokeObjectURL(link.href);
      }});
      
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
              const dy = lastOutputDownY - e.clientY;
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

      function renderTableList() {{
          const filter = sidebarSearchInput.value.toUpperCase();
          sidebarTableList.innerHTML = '';
          
          Object.keys(dbSchema).sort().forEach(tableName => {{
              if (tableName.toUpperCase().indexOf(filter) > -1) {{
                  const li = document.createElement('li');
                  const a = document.createElement('a');
                  a.className = 'table-list-item';
                  a.textContent = tableName;
                  a.href = '#';
                  a.title = "Click to SELECT * LIMIT 100";
                  a.onclick = (e) => {{ e.preventDefault(); editor.value = "SELECT * FROM " + tableName + " LIMIT 100;"; handleInput(); }};
                  li.appendChild(a);
                  sidebarTableList.appendChild(li);
              }}
          }});
      }}
      renderTableList();
      sidebarSearchInput.addEventListener('keyup', renderTableList);

      const refreshBtn = document.getElementById('refresh-schema-btn');
      refreshBtn.addEventListener('click', refreshSchema);

      async function refreshSchema() {{
          refreshBtn.style.animation = "spin 1s linear infinite";
          try {{
              const resp = await fetch('/sql/' + connectionNickname + '/schema-json');
              if (resp.ok) {{
                  dbSchema = await resp.json();
                  renderTableList();
              }} else {{
                  console.error("Failed to refresh schema");
              }}
          }} catch(e) {{
              console.error(e);
          }} finally {{
              refreshBtn.style.animation = "none";
          }}
      }}
      
      // Inject spin animation
      const styleSheet = document.createElement("style");
      styleSheet.innerText = `@keyframes spin {{ 0% {{ transform: rotate(0deg); }} 100% {{ transform: rotate(360deg); }} }}`;
      document.head.appendChild(styleSheet);

      function filterSavedQueries() {{
          const filter = querySearchInput.value.toUpperCase();
          const listItems = savedQueriesList.getElementsByTagName('li');
          for (let i = 0; i < listItems.length; i++) {{
              const itemText = listItems[i].querySelector('.query-link').textContent || listItems[i].querySelector('.query-link').innerText;
              if (itemText.toUpperCase().indexOf(filter) > -1) {{ listItems[i].style.display = 'flex'; }} else {{ listItems[i].style.display = 'none'; }}
          }}
      }}
      querySearchInput.addEventListener('keyup', filterSavedQueries);

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
            
            const table = output.querySelector('table');
            if(table) {{
                makeTableInteractable(table);
                // UPDATE ROW COUNT
                const rows = table.querySelectorAll('tbody tr');
                const countSpan = document.getElementById('row-count');
                if(countSpan) countSpan.innerText = rows.length + " rows";
            }}

            // AUTO-REFRESH SCHEMA on DDL
            const upperSql = payload.sql.toUpperCase();
            if (upperSql.includes("CREATE TABLE") || 
                upperSql.includes("DROP TABLE") || 
                upperSql.includes("ALTER TABLE")) {{
                refreshSchema();
            }}

        }} catch(e) {{
            output.innerHTML = '<pre style="padding:10px; color:#ff6b6b;">Error: ' + e.message + '</pre>';
        }}
      }});
      
      // Client-Side Export Logic
      document.getElementById('export-client-btn').addEventListener('click', () => {{
          const table = output.querySelector('table');
          if(!table) return alert('No results to export');
          
          const includeHeaders = document.getElementById('export-headers').checked;
          const rows = Array.from(table.querySelectorAll('tr'));
          const selectedRows = Array.from(table.querySelectorAll('tr.selected-row'));
          
          // Use selected rows if any, otherwise all visible rows (respecting filter)
          let targetRows = selectedRows.length > 0 ? selectedRows : rows.filter(r => r.style.display !== 'none');
          
          // Ensure we don't duplicate headers if they happen to be selected or in the list
          // Actually, 'rows' includes the header row usually in thead. 
          // Let's grab headers separately.
          const theadRow = table.querySelector('thead tr');
          const tbodyRows = Array.from(table.querySelectorAll('tbody tr'));
          
          let csvContent = "";
          
          if(includeHeaders && theadRow) {{
              const headers = Array.from(theadRow.children).map(th => `"${{th.innerText.replace(/"/g, '""')}}"`);
              csvContent += headers.join(",") + "\n";
          }}
          
          // Filter body rows based on selection or visibility
          let rowsToExport = [];
          if(selectedRows.length > 0) {{
               rowsToExport = selectedRows;
          }} else {{
               rowsToExport = tbodyRows.filter(r => r.style.display !== 'none');
          }}
          
          rowsToExport.forEach(row => {{
              const cols = Array.from(row.children).map(td => `"${{td.innerText.replace(/"/g, '""')}}"`);
              csvContent += cols.join(",") + "\n";
          }});
          
          const blob = new Blob([csvContent], {{ type: 'text/csv;charset=utf-8;' }});
          const link = document.createElement("a");
          const url = URL.createObjectURL(blob);
          link.setAttribute("href", url);
          link.setAttribute("download", "export.csv");
          link.style.visibility = 'hidden';
          document.body.appendChild(link);
          link.click();
          document.body.removeChild(link);
      }});
      
      // Filtering Logic
      document.getElementById('output-filter').addEventListener('input', (e) => {{
          const term = e.target.value.toLowerCase();
          const table = output.querySelector('table');
          if(!table) return;
          const rows = table.querySelectorAll('tbody tr');
          
          let visibleCount = 0;
          rows.forEach(row => {{
              const text = row.innerText.toLowerCase();
              if (text.includes(term)) {{
                  row.style.display = '';
                  visibleCount++;
              }} else {{
                  row.style.display = 'none';
              }}
          }});
          
          const countSpan = document.getElementById('row-count');
          if(countSpan) countSpan.innerText = visibleCount + " rows";
      }});

      let isSelecting = false;
      let selectionMode = true;

      function updateSelectionCount() {{
          const selected = document.querySelectorAll('.output tr.selected-row').length;
          const btn = document.getElementById('clear-selection-btn');
          if (btn) {{
              if (selected > 0) {{
                  btn.style.display = 'inline-block';
                  btn.innerText = `Clear (${{selected}})`;
              }} else {{
                  btn.style.display = 'none';
              }}
          }}
      }}

      const clearSelectionBtn = document.getElementById('clear-selection-btn');
      if (clearSelectionBtn) {{
          clearSelectionBtn.addEventListener('click', () => {{
              document.querySelectorAll('.output tr.selected-row').forEach(row => row.classList.remove('selected-row'));
              updateSelectionCount();
          }});
      }}

      function makeTableInteractable(table) {{
        const ths = table.querySelectorAll('th');
        const tbody = table.querySelector('tbody');
        const rows = Array.from(tbody.querySelectorAll('tr'));
        
        rows.forEach((row, i) => {{
            row.dataset.originalIndex = i;
        }});

        tbody.addEventListener('mousedown', (e) => {{
            const tr = e.target.closest('tr');
            if (tr) {{
                isSelecting = true;
                selectionMode = !tr.classList.contains('selected-row');
                tr.classList.toggle('selected-row', selectionMode);
                updateSelectionCount();
                e.preventDefault(); // Prevent text selection while dragging
            }}
        }});

        tbody.addEventListener('mouseover', (e) => {{
            if (isSelecting) {{
                const tr = e.target.closest('tr');
                if (tr) {{
                    tr.classList.toggle('selected-row', selectionMode);
                    updateSelectionCount();
                }}
            }}
        }});

        // Global mouseup to stop selection even if released outside table
        if (!window._selectionHandlerBound) {{
            window.addEventListener('mouseup', () => {{
                isSelecting = false;
            }});
            window._selectionHandlerBound = true;
        }}

        let currentSortCol = -1;
        let currentSortDir = 'none'; 

        ths.forEach((th, colIndex) => {{
            th.addEventListener('click', () => {{
                if (currentSortCol === colIndex) {{
                    if (currentSortDir === 'none') currentSortDir = 'asc';
                    else if (currentSortDir === 'asc') currentSortDir = 'desc';
                    else currentSortDir = 'none';
                }} else {{
                    currentSortCol = colIndex;
                    currentSortDir = 'asc';
                }}

                ths.forEach(h => h.innerHTML = h.innerHTML.replace(/ [▲▼]$/, '')); 
                if (currentSortDir === 'asc') th.innerHTML += ' ▲';
                if (currentSortDir === 'desc') th.innerHTML += ' ▼';

                const newRows = Array.from(rows);
                if (currentSortDir !== 'none') {{
                    newRows.sort((rowA, rowB) => {{
                        const cellA = rowA.children[colIndex].innerText.trim();
                        const cellB = rowB.children[colIndex].innerText.trim();
                        
                        const numA = parseFloat(cellA.replace(/[$,]/g, ''));
                        const numB = parseFloat(cellB.replace(/[$,]/g, ''));
                        
                        let comparison = 0;
                        if (!isNaN(numA) && !isNaN(numB) && !/[a-zA-Z]/.test(cellA) && !/[a-zA-Z]/.test(cellB)) {{
                            comparison = numA - numB;
                        }} else {{
                            comparison = cellA.localeCompare(cellB, undefined, {{ numeric: true, sensitivity: 'base' }});
                        }}
                        
                        return currentSortDir === 'asc' ? comparison : -comparison;
                    }});
                }} else {{
                    newRows.sort((a, b) => a.dataset.originalIndex - b.dataset.originalIndex);
                }}

                tbody.innerHTML = '';
                newRows.forEach(row => tbody.appendChild(row));
                
                // Re-apply filter after sort
                document.getElementById('output-filter').dispatchEvent(new Event('input'));
            }});
        }});
      }}

      savedQueriesList.addEventListener('click', (e) => {{
          const target = e.target.closest('a');
          if (target) {{ 
              e.preventDefault(); 
              const sql = target.getAttribute('data-sql'); 
              const name = target.getAttribute('data-name'); 
              editor.value = sql; 
              queryNameInput.value = name; 
              scanForVariables(); 
              handleInput(); 
          }}
      }});

      saveQueryForm.addEventListener('submit', (e) => {{
          querySqlInput.value = editor.value;
          if (queryNameInput.value.trim() === '') {{ e.preventDefault(); }}
      }});

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
          
          const closeBtn = document.createElement('span');
          closeBtn.className = 'var-del-btn';
          closeBtn.innerHTML = '&times;';
          closeBtn.title = 'Remove Variable';
          closeBtn.onclick = function() {{
              div.remove();
          }};

          div.appendChild(label);
          div.appendChild(input);
          div.appendChild(closeBtn); 
          
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
          currentInputs.forEach(i => {{ if(i.name) currentValues[i.name] = i.value; }});
          
          const existingGroups = variablesSection.querySelectorAll('.var-input-group');
          existingGroups.forEach(g => g.remove());
          
          foundVars.forEach(v => {{
              addVariable(v, currentValues[v] || '');
          }});
      }}
      
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
          
          html = html.replace(/(--.*$)/gm, (m) => pushToken(m, 'hl-comment'));
          
          html = html.replace(/('([^'\\]|\\.)*')/g, (m) => pushToken(m, 'hl-string'));
          
          const keywords = ["SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "JOIN", "INNER", "OUTER", "LEFT", "RIGHT", "ON", "GROUP", "BY", "ORDER", "LIMIT", "OFFSET", "AND", "OR", "NOT", "NULL", "AS", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN", "LIKE", "ILIKE", "IN", "IS", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END", "HAVING", "UNION", "ALL"];
          
          const rxKeyword = new RegExp(`\\b(${{keywords.join('|')}})\\b`, 'gi');
          html = html.replace(rxKeyword, '<span class="hl-keyword">$1</span>');
          
          html = html.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');
          
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

      function getAliases(sql) {{
        const aliases = {{}};
        // Match: FROM table alias OR JOIN table alias
        // Also supports: table AS alias
        const regex = /\b(?:FROM|JOIN)\s+([a-zA-Z0-9_]+)(?:\s+AS)?\s+([a-zA-Z0-9_]+)\b/gi;
        let match;
        while ((match = regex.exec(sql)) !== null) {{
            const table = match[1];
            const alias = match[2];
            // Don't treat common SQL keywords as aliases if they appear after a table
            const keywords = ["WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "ON", "GROUP", "BY", "ORDER", "LIMIT", "OFFSET", "AND", "OR"];
            if (!keywords.includes(alias.toUpperCase())) {{
                aliases[alias.toLowerCase()] = table;
            }}
        }}
        return aliases;
      }}

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
          
          if (currentWord.includes('.')) {{
              const parts = currentWord.split('.');
              const prefix = parts[0];
              const colPrefix = parts[1] || '';
              
              // 1. Try to find a direct table match
              let targetTable = Object.keys(dbSchema).find(t => t.toUpperCase() === prefix.toUpperCase());
              
              // 2. If no direct match, try to resolve as an alias
              if (!targetTable) {{
                  const aliases = getAliases(val);
                  const aliasedTable = aliases[prefix.toLowerCase()];
                  if (aliasedTable) {{
                      targetTable = Object.keys(dbSchema).find(t => t.toUpperCase() === aliasedTable.toUpperCase());
                  }}
              }}
              
              if (targetTable && dbSchema[targetTable]) {{
                  matches = dbSchema[targetTable]
                      .filter(col => col.toUpperCase().startsWith(colPrefix.toUpperCase()))
                      .map(col => ({{ display: col, insert: col, type: 'column' }}));
              }}
          }} 
          else {{
              // Standard table suggestions
              matches = Object.keys(dbSchema)
                  .filter(t => t.toUpperCase().startsWith(currentWord.toUpperCase()))
                  .map(t => ({{ display: t, insert: t, type: 'table' }}));

              // Keyword suggestions (optional but helpful)
              const keywords = ["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "JOIN", "ORDER BY", "GROUP BY", "LIMIT", "CREATE TABLE", "DROP TABLE"];
              const kwMatches = keywords
                  .filter(k => k.startsWith(currentWord.toUpperCase()) && currentWord.length >= 2)
                  .map(k => ({{ display: k, insert: k, type: 'keyword' }}));
              matches = [...matches, ...kwMatches];
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
      
      // Inject Shared Sidebar JS
      {sidebar_js}
    </script>
    "###, nickname = nickname_safe, table_schema_json = table_schema_json, sidebar_html = sidebar_html, sidebar_js = sidebar_js);

    render_base_page(
        &format!("SQL View: {}", htmlescape::encode_minimal(&nickname)),
        &format!("{}{}{}", page_styles, crate::elements::sidebar::get_css(), body_content),
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