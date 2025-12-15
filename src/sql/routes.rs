use actix_web::{get, post, web::{self, Data, Form}, HttpResponse, Responder};
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
pub async fn sql_run(form: Form<SqlForm>, state: Data<Arc<AppState>>) -> impl Responder {
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

    let rows = match sqlx::query(&form.sql).fetch_all(&pool).await {
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
fn render_query_view(nickname: &str, table_list: &str, current_theme: &crate::app_state::Theme) -> String {
    let saved_queries = load_queries();
    let nickname_safe = htmlescape::encode_minimal(nickname);
    
    let saved_query_list = saved_queries.iter()
        .map(|q| {
            let sql_safe = htmlescape::encode_minimal(&q.sql);
            let name_safe = htmlescape::encode_minimal(&q.name);
            
            // FIX: Use standard format! with escaped quotes for stable compilation
            format!(
                "<li class=\"saved-query-item\">\
                    <a href=\"#\" data-sql=\"{}\" data-name=\"{}\" class=\"query-link\">{}</a>\
                    <form method=\"POST\" action=\"/sql/delete\" style=\"display:inline;\">\
                        <input type=\"hidden\" name=\"query_name\" value=\"{}\">\
                        <input type=\"hidden\" name=\"connection\" value=\"{}\">\
                        <button type=\"submit\" class=\"delete-btn\" title=\"Delete\">x</button>\
                    </form>\
                </li>",
                sql_safe, name_safe, name_safe, name_safe, nickname_safe
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let page_styles = r#"
<style>
    .sql-view-container { display: flex; height: calc(100vh - 70px); position: relative; }
    #sidebar { width: 200px; background: var(--secondary-bg); color: var(--text-color); padding: 10px; overflow-y: auto; transition: width 0.3s, padding 0.3s; flex-shrink: 0; border-right: 1px solid var(--border-color); }
    #sidebar h2 { margin: 0; padding-bottom: 5px; border-bottom: 1px solid var(--border-color); }
    #sidebar ul { list-style: none; padding: 0; margin: 5px 0 0 0; }
    #sidebar li { padding: 5px 0; cursor: pointer; }
    
    /* Updated sidebar styles for saved queries */
    .saved-query-item { display: flex; justify-content: space-between; align-items: center; padding-right: 5px; }
    .query-link { flex-grow: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-right: 5px; }
    .delete-btn { background: none; border: none; color: #ff6b6b; font-weight: bold; padding: 0 5px; margin: 0; cursor: pointer; }
    .delete-btn:hover { color: #ff3b3b; background: rgba(255,0,0,0.1); border-radius: 3px; }
    
    .sidebar-search input { width: 95%; padding: 5px; margin-bottom: 10px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; }
    #sidebar ul a { display: block; }
    .query-save-form { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border-color); }
    .query-save-form input[type="text"] { width: 100%; padding: 5px; margin-bottom: 5px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; }
    #toggle-arrow { position: absolute; top: 10px; left: 200px; cursor: pointer; font-size: 18px; user-select: none; background: var(--tertiary-bg); color: var(--text-color); padding: 4px; border-radius: 4px; transition: left 0.3s, background-color 0.2s; line-height: 1; z-index: 10; }
    #toggle-arrow:hover { background: var(--border-color); }
    #main { flex: 1; display: flex; flex-direction: column; padding: 10px; }
    #sql-form { display: flex; flex-direction: column; flex-grow: 1; }
    .editor-container { flex: 1; min-height: 200px; margin-bottom: 10px; }
    #sql-editor { height: 100%; }
    .output { margin-top: 10px; flex-shrink: 0; max-height: 50%; overflow-y: auto; }
    .action-buttons { display: flex; gap: 10px; flex-shrink: 0; }
    .action-buttons button { margin-top: 0; }
    .grid { width: 100%; }
</style>
"#;

    let body_content = format!(r#"
    <div class="sql-view-container">
      <div id="sidebar">
        <h2>Tables</h2>
        <div class="sidebar-search"><input type="text" id="sidebar-search-input" placeholder="Search tables..."></div>
        <ul id="table-list">{table_list}</ul>
        <h2 style="margin-top: 20px;">Saved Queries</h2>
        <div class="sidebar-search"><input type="text" id="query-search-input" placeholder="Search queries..."></div>
        <ul id="saved-queries-list">{saved_query_list}</ul>
        <form id="save-query-form" method="POST" action="/sql/save" class="query-save-form">
            <input type="text" id="query-name" name="query_name" placeholder="Name query to save" required>
            <input type="hidden" id="query-sql" name="sql">
            <!-- Add hidden connection input so redirect works -->
            <input type="hidden" name="connection" value="{nickname}">
            <button type="submit">Save Current Query</button>
        </form>
      </div>
      <span id="toggle-arrow">&#x25C0;</span>
      <div id="main">
        <form id="sql-form" method="POST" action="/sql/run">
          <input type="hidden" name="connection" value="{nickname}">
          <div class="editor-container">
            <textarea id="sql-editor" name="sql" placeholder="SELECT * FROM table_name WHERE..."></textarea>
          </div>
          <div class="action-buttons">
            <button type="submit">Run Query</button>
            <a href="/sql/export" target="_blank"><button type="button">Save Results as CSV</button></a>
          </div>
        </form>
        <div class="output" id="output"><pre>Click a table name or enter a query and press 'Run Query'.</pre></div>
      </div>
    </div>
    <script>
      const toggleArrow = document.getElementById('toggle-arrow');
      const sidebar = document.getElementById('sidebar');
      const mainContent = document.getElementById('main');
      let collapsed = false;
      const editor = document.getElementById('sql-editor');
      const sidebarSearchInput = document.getElementById('sidebar-search-input');
      const sidebarTableList = document.getElementById('table-list');
      const querySearchInput = document.getElementById('query-search-input');
      const savedQueriesList = document.getElementById('saved-queries-list');
      const saveQueryForm = document.getElementById('save-query-form');
      const queryNameInput = document.getElementById('query-name');
      const querySqlInput = document.getElementById('query-sql');

      toggleArrow.addEventListener('click', () => {{
        if (!collapsed) {{
          sidebar.style.width = '0px'; sidebar.style.padding = '0'; toggleArrow.innerHTML = '&#x25B6;'; toggleArrow.style.left = '0px'; mainContent.style.width = '100%'; collapsed = true;
        }} else {{
          sidebar.style.width = '200px'; sidebar.style.padding = '10px'; toggleArrow.innerHTML = '&#x25C0;'; toggleArrow.style.left = '200px'; mainContent.style.width = 'auto'; collapsed = false;
        }}
      }});
      toggleArrow.style.left = sidebar.style.width;

      const form = document.getElementById('sql-form');
      const output = document.getElementById('output');
      form.addEventListener('submit', async (e) => {{
        e.preventDefault();
        output.innerHTML = 'Loading...';
        const formData = new FormData(form);
        const body = new URLSearchParams(formData).toString();
        const resp = await fetch('/sql/run', {{ method: 'POST', headers: {{ 'Content-Type': 'application/x-www-form-urlencoded' }}, body: body }});
        const html = await resp.text();
        output.innerHTML = html;
        queryNameInput.value = '';
      }});

      sidebarTableList.addEventListener('click', (e) => {{
          const target = e.target.closest('a');
          if (target) {{ e.preventDefault(); const table_name = target.textContent; editor.value = "SELECT * FROM \\\"" + table_name + "\\\" LIMIT 100;"; }}
      }});

      savedQueriesList.addEventListener('click', (e) => {{
          const target = e.target.closest('a');
          if (target) {{ e.preventDefault(); const sql = target.getAttribute('data-sql'); const name = target.getAttribute('data-name'); editor.value = sql; queryNameInput.value = name; }}
      }});

      saveQueryForm.addEventListener('submit', (e) => {{
          querySqlInput.value = editor.value;
          if (queryNameInput.value.trim() === '') {{ e.preventDefault(); }}
      }});

      function filterSidebarTables() {{
          const filter = sidebarSearchInput.value.toUpperCase();
          const listItems = sidebarTableList.getElementsByTagName('li');
          for (let i = 0; i < listItems.length; i++) {{
              const itemText = listItems[i].textContent || listItems[i].innerText;
              if (itemText.toUpperCase().indexOf(filter) > -1) {{ listItems[i].style.display = ''; }} else {{ listItems[i].style.display = 'none'; }}
          }}
      }}

      function filterSavedQueries() {{
          const filter = querySearchInput.value.toUpperCase();
          const listItems = savedQueriesList.getElementsByTagName('li');
          for (let i = 0; i < listItems.length; i++) {{
              const itemText = listItems[i].querySelector('.query-link').textContent || listItems[i].querySelector('.query-link').innerText;
              if (itemText.toUpperCase().indexOf(filter) > -1) {{ listItems[i].style.display = 'flex'; }} else {{ listItems[i].style.display = 'none'; }}
          }}
      }}

      sidebarSearchInput.addEventListener('keyup', filterSidebarTables);
      querySearchInput.addEventListener('keyup', filterSavedQueries);

      if (editor.value === "") {{ editor.value = "SELECT 1;"; }}
    </script>
    "#, nickname = nickname_safe, table_list = table_list, saved_query_list = saved_query_list);

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

    let rows = match sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema='public'").fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => {
            let current_theme = state.current_theme.lock().unwrap();
            let error_content = format!(r#"<h1>SQL Error</h1><pre class="error-message">Failed to list tables: {e}</pre>"#, e = htmlescape::encode_minimal(&e.to_string()));
            return HttpResponse::InternalServerError().body(render_base_page("SQL Error", &error_content, &current_theme));
        }
    };

    let tables: Vec<String> = rows.into_iter().filter_map(|row| row.try_get::<String, _>("table_name").ok()).collect();

    let table_list = tables.iter().map(|t| {
        let safe = htmlescape::encode_minimal(t);
        format!("<li><a href=\"#\">{}</a></li>", safe)
    }).collect::<Vec<_>>().join("\n");
        
    let current_theme = state.current_theme.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_query_view(&nickname, &table_list, &current_theme))
}