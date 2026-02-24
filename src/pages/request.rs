use actix_web::{get, post, web::{self, Data, Form, Json}, HttpResponse, Responder};
use std::{fs, io, process::Command, collections::HashMap, time::{Instant, SystemTime, UNIX_EPOCH}, sync::{Mutex, OnceLock}};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;
use htmlescape::encode_minimal;

const REQUESTS_FILE: &str = "saved_requests.json";
static RUNNING_REQUESTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

fn running_requests() -> &'static Mutex<HashMap<String, u32>> {
    RUNNING_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize, Deserialize, Clone)]
struct SavedRequest {
    name: String,
    method: String,
    url: String,
    headers: String,
    body: String,
    auth_type: Option<String>, 
    oauth_token_url: Option<String>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_scope: Option<String>,
}

#[derive(Deserialize)]
struct SaveRequestForm {
    name: String,
    method: String,
    url: String,
    headers: String, 
    body: String,
    auth_type: Option<String>,
    oauth_token_url: Option<String>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_scope: Option<String>,
}

#[derive(Deserialize)]
struct DeleteRequestForm {
    name: String,
}

#[derive(Deserialize)]
struct ProxyRequest {
    method: String,
    url: String,
    headers: Vec<HeaderPair>,
    body: String,
    request_id: Option<String>,
}

#[derive(Deserialize)]
struct HeaderPair {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct CancelRequest {
    request_id: String,
}

#[derive(Serialize)]
struct ProxyResponse {
    status: u16,
    headers: String,
    body: String,
    stderr: String,
    curl_exit: i32,
    duration_ms: u128,
}

fn load_requests() -> Vec<SavedRequest> {
    fs::read_to_string(REQUESTS_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_requests_to_file(requests: &[SavedRequest]) -> io::Result<()> {
    let data = serde_json::to_string_pretty(requests)?;
    fs::write(REQUESTS_FILE, data)
}

// --- Handlers ---

#[get("/requests")]
pub async fn request_get(state: Data<std::sync::Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_request_page(&current_theme, &saved_themes))
}

#[post("/requests/save")]
pub async fn request_save(form: Form<SaveRequestForm>) -> impl Responder {
    let mut requests = load_requests();
    let new_req = SavedRequest {
        name: form.name.clone(),
        method: form.method.clone(),
        url: form.url.clone(),
        headers: form.headers.clone(),
        body: form.body.clone(),
        auth_type: form.auth_type.clone(),
        oauth_token_url: form.oauth_token_url.clone(),
        oauth_client_id: form.oauth_client_id.clone(),
        oauth_client_secret: form.oauth_client_secret.clone(),
        oauth_scope: form.oauth_scope.clone(),
    };

    if let Some(idx) = requests.iter().position(|r| r.name == new_req.name) {
        requests[idx] = new_req;
    } else {
        requests.push(new_req);
    }

    let _ = save_requests_to_file(&requests);
    
    HttpResponse::Found()
        .append_header(("Location", "/requests"))
        .finish()
}

#[post("/requests/delete")]
pub async fn request_delete(form: Form<DeleteRequestForm>) -> impl Responder {
    let mut requests = load_requests();
    if let Some(idx) = requests.iter().position(|r| r.name == form.name) {
        requests.remove(idx);
        let _ = save_requests_to_file(&requests);
    }
    
    HttpResponse::Found()
        .append_header(("Location", "/requests"))
        .finish()
}

#[post("/requests/run")]
pub async fn request_run(payload: Json<ProxyRequest>) -> impl Responder {
    let res = web::block(move || {
        let started = Instant::now();
        let mut cmd = Command::new("curl");
        let request_id = payload.request_id.clone().unwrap_or_else(|| {
            format!(
                "req_{}_{}",
                std::process::id(),
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
            )
        });

        let run_id = format!(
            "{}_{}_{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos(),
            payload.method
        );
        let mut header_path = std::env::temp_dir();
        header_path.push(format!("go_service_headers_{run_id}.txt"));
        let mut body_path = std::env::temp_dir();
        body_path.push(format!("go_service_body_{run_id}.txt"));

        cmd.arg("-sS")
            .arg("--connect-timeout").arg("15")
            .arg("--max-time").arg("60")
            .arg("-X").arg(&payload.method);

        for header in &payload.headers {
            if !header.key.trim().is_empty() {
                cmd.arg("-H").arg(format!("{}: {}", header.key, header.value));
            }
        }

        if !payload.body.is_empty() && payload.method != "GET" && payload.method != "HEAD" {
            cmd.arg("-d").arg(&payload.body);
        }

        cmd.arg("-D").arg(&header_path)
            .arg("-o").arg(&body_path)
            .arg("-w").arg("%{http_code}")
            .arg(&payload.url);

        let child = cmd.spawn()?;
        if let Ok(mut map) = running_requests().lock() {
            map.insert(request_id.clone(), child.id());
        }
        let output = child.wait_with_output()?;
        if let Ok(mut map) = running_requests().lock() {
            map.remove(&request_id);
        }

        let headers = fs::read_to_string(&header_path).unwrap_or_default();
        let body = fs::read(&body_path)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or_default();
        let _ = fs::remove_file(&header_path);
        let _ = fs::remove_file(&body_path);

        let status = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u16>()
            .unwrap_or(0);

        Ok::<ProxyResponse, io::Error>(ProxyResponse {
            status,
            headers,
            body,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            curl_exit: output.status.code().unwrap_or(-1),
            duration_ms: started.elapsed().as_millis(),
        })
    }).await;

    match res {
        Ok(Ok(output)) => HttpResponse::Ok().json(output),
        Ok(Err(e)) => HttpResponse::InternalServerError().body(format!("Failed to execute curl: {}", e)),
        _ => HttpResponse::InternalServerError().body("Blocked execution error"),
    }
}

#[post("/requests/cancel")]
pub async fn request_cancel(payload: Json<CancelRequest>) -> impl Responder {
    let pid = {
        let mut map = running_requests().lock().unwrap();
        map.remove(&payload.request_id)
    };

    if let Some(pid) = pid {
        let result = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
        return match result {
            Ok(status) if status.success() => HttpResponse::Ok().body("Cancelled"),
            Ok(_) => HttpResponse::InternalServerError().body("Failed to cancel request"),
            Err(err) => HttpResponse::InternalServerError().body(format!("Cancel error: {}", err)),
        };
    }

    HttpResponse::NotFound().body("Request not found")
}


// --- Rendering ---

fn render_request_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let saved_requests = load_requests();
    
    let saved_list_html = saved_requests.iter().map(|r| {
        let safe_name = encode_minimal(&r.name);
        // Use standard strings with escaped quotes
        format!(r##"
                <li class="saved-req-item">
                    <div style="display:flex; align-items:center; min-width:0; flex-grow:1;">
                        <span class="req-method {}">{}</span>
                        <a href="#" class="req-link" 
                        data-name="{}" 
                        data-method="{}" 
                        data-url="{}" 
                        data-headers="{}" 
                        data-body="{}" 
                        data-auth-type="{}" 
                        data-oauth-token-url="{}" 
                        data-oauth-client-id="{}" 
                        data-oauth-client-secret="{}" 
                        data-oauth-scope="{}">{}</a>
                    </div>
                    <form method="POST" action="/requests/delete" class="delete-form">
                        <input type="hidden" name="name" value="{}">
                        <button type="submit" class="btn-danger-text" title="Delete">×</button>
                    </form>
                </li>"##,
            r.method.to_lowercase(), r.method, 
            safe_name, 
            encode_minimal(&r.method), 
            encode_minimal(&r.url), 
            encode_minimal(&r.headers), 
            encode_minimal(&r.body), 
            r.auth_type.as_deref().unwrap_or("none"),
            encode_minimal(r.oauth_token_url.as_deref().unwrap_or("")),
            encode_minimal(r.oauth_client_id.as_deref().unwrap_or("")),
            encode_minimal(r.oauth_client_secret.as_deref().unwrap_or("")),
            encode_minimal(r.oauth_scope.as_deref().unwrap_or("")),
            safe_name,
            safe_name
        )
    }).collect::<Vec<_>>().join("\n");

    let style = format!(r#"
<style>
    .req-container {{ 
        display: flex; 
        height: calc(100vh - 65px); /* Increased offset to account for header/margins */
        overflow: hidden; 
        position: relative;
    }}
    
    .saved-list {{ 
        list-style: none; 
        padding: 0; 
        margin: 0; 
        overflow-y: auto; 
        flex: 1; 
        min-height: 0; 
    }}
    
    .saved-req-item {{ display: flex; align-items: center; justify-content: space-between; padding: 1px 5px; min-height: 24px; background-color: transparent; cursor: pointer; transition: background 0.2s; }}
    .saved-req-item .delete-form {{ margin: 0; display: flex; align-items: center; flex: 0 0 auto; align-self: center; }}
    .saved-req-item .btn-danger-text {{ width: 18px; height: 18px; min-height: 18px; min-width: 18px; font-size: var(--font-size-small); line-height: 1; padding: 0; margin: 0; display: inline-flex; align-items: center; justify-content: center; transform: translateY(-1px); }}
    .saved-req-item:hover {{ background-color: var(--tertiary-bg); }}
    .saved-req-item.selected {{ background-color: var(--tertiary-bg); border-left: 3px solid var(--link-color); padding-left: 2px; }}

    .req-method {{ font-size: calc(var(--font-size-small) * 0.8); font-weight: bold; padding: 1px 4px; border-radius: 2px; margin-right: 5px; min-width: 35px; text-align: center; color: #fff; flex-shrink: 0; }}
    .req-method.get {{ background-color: #61affe; }}
    .req-method.post {{ background-color: #49cc90; }}
    .req-method.put {{ background-color: #fca130; }}
    .req-method.delete {{ background-color: #f93e3e; }}
    .req-method.patch {{ background-color: #50e3c2; }}
    
    .req-link {{ text-decoration: none; color: var(--text-color); flex-grow: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: var(--font-size-small); min-width: 0; transition: color 0.2s; }}
    .req-link:hover {{ color: var(--link-hover); }}
    
    /* Shared styles for delete-btn removed - now using .btn-danger-text in static/style.css */

    /* Resizers */
    /* Sidebar & Response Resizer styles removed - now using shared .resizer-v/.resizer-h classes */

    /* Main Area */
    .main-area {{ flex-grow: 1; padding: 0; display: flex; flex-direction: column; overflow: hidden; height: 100%; box-sizing: border-box; background-color: var(--primary-bg); }}
    
    /* Request Bar */
    .request-bar {{ display: flex; gap: 5px; padding: 10px; background: var(--secondary-bg); border-bottom: 1px solid var(--border-color); flex-shrink: 0; align-items: center; }}
    .method-select {{ padding: 4px 8px; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; font-weight: bold; height: 30px; }}
    .url-input {{ flex-grow: 1; padding: 4px 8px; background: var(--primary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 4px; height: 30px; box-sizing: border-box; }}
    .send-btn {{ padding: 0 15px; background-color: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: bold; height: 30px; }}
    .send-btn:hover {{ background-color: #0056b3; }}
    .save-btn {{ padding: 0 10px; background-color: var(--tertiary-bg); color: var(--text-color); border: 1px solid var(--border-color); border-radius: 4px; cursor: pointer; height: 30px; }}
    
    /* Tabs */
    /* Tabs System styles removed - now using shared .tabs class in static/style.css */
    
    .input-container {{
        flex: 1; /* Default flex grow */
        display: flex;
        flex-direction: column;
        overflow: hidden;
        min-height: 100px;
        background: var(--primary-bg);
    }}
    
    .tab-content {{ display: none; flex-direction: column; gap: 10px; flex: 1; overflow-y: auto; padding: 10px; min-height: 0; }}
    .tab-content.active {{ display: flex; }}
    .body-type-row {{ display:flex; gap:15px; margin-bottom:10px; align-items:center; }}
    .body-type-row .title {{ font-size: var(--font-size-small); color:#888; }}
    .body-type-row label {{ font-size: var(--font-size-small); color: var(--text-color); display: inline-flex; align-items: center; gap: 4px; }}
    
    textarea.code-editor {{
        width: 100%; height: 100%; background: var(--secondary-bg); color: var(--text-color); border: 1px solid var(--border-color); border-radius: 4px; padding: 10px; font-family: monospace; font-size: var(--font-size-medium); box-sizing: border-box; resize: none;
    }}

    /* Key-Value Tables (Params & Headers) removed - now using shared .kv-row class */
    .kv-remove {{ background: none; border: none; color: #f93e3e; font-weight: bold; cursor: pointer; padding: 0 8px; }}
    
    /* Read-only key for Path Params */
    .kv-input.key.readonly {{ background-color: var(--tertiary-bg); color: #aaa; }}

    /* Auth Section removed - now using shared .form-group logic where possible */
    .auth-section {{ display: flex; flex-direction: column; gap: 10px; padding: 0; background: transparent; border: none; }}
    .auth-row {{ display: flex; gap: 10px; align-items: center; flex-wrap: wrap;}}
    .auth-row label {{ width: 100px; flex-shrink: 0; font-size: var(--font-size-small); color: #aaa;}}
    .auth-row input, .auth-row select {{ flex: 1; padding: 5px; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 3px; }}
    .oauth-btn {{ background-color: #fca130; color: #000; border: none; padding: 6px 12px; border-radius: 3px; cursor: pointer; font-weight: bold; font-size: var(--font-size-small); }}
    .oauth-btn:hover {{ background-color: #e59029; }}
    .token-display {{ width: 100%; margin-top: 5px; }}

    /* Response Area */
    .response-section {{ 
        height: 40%; /* Initial height */
        padding: 0; 
        display: flex; 
        flex-direction: column; 
        min-height: 50px; 
        overflow: hidden; 
        background: var(--secondary-bg);
    }}
    .response-header {{ padding: 5px 10px; background: var(--tertiary-bg); border-bottom: 1px solid var(--border-color); display: flex; justify-content: space-between; align-items: center; flex-shrink: 0; }}
    .response-meta {{ display: flex; gap: 15px; font-size: var(--font-size-small); color: #aaa; flex-shrink: 0; margin: 0; }}
    
    /* Debug Info */
    #request-debug-info {{ margin-bottom: 5px; color: #888; font-family: monospace; font-size: var(--font-size-small); white-space: pre-wrap; overflow-x: auto; display: none; background: var(--primary-bg); padding: 5px; border-bottom: 1px solid var(--border-color); flex-shrink: 0;}}

    #response-body {{ flex: 1; white-space: pre-wrap; overflow: auto; font-family: monospace; background: var(--primary-bg); padding: 10px; border: none; min-height: 60px; color: var(--text-color); }}
    #response-headers {{ height: 160px; overflow: auto; white-space: pre-wrap; font-family: monospace; background: var(--primary-bg); padding: 10px; border-bottom: 1px solid var(--border-color); margin: 0; color: var(--text-color); min-height: 50px; }}
    #headers-body-resizer {{ height: 6px; background: var(--tertiary-bg); border-top: 1px solid var(--border-color); border-bottom: 1px solid var(--border-color); cursor: row-resize; flex-shrink: 0; }}
    #headers-body-resizer.resizing {{ background: var(--link-color); }}
    
    /* Save Modal */
    .save-controls {{ display: flex; gap: 5px; align-items: center; background: var(--tertiary-bg); padding: 10px; border-bottom: 1px solid var(--border-color); margin-bottom: 0; display: none; flex-shrink: 0; }}
    .save-controls input {{ flex-grow: 1; padding: 4px; border: 1px solid var(--border-color); border-radius: 3px; }}
</style>
"#);

    let sidebar_content = format!(r#"
        <h2>Saved Requests</h2>
        <ul class="saved-list" id="saved-list">
            {}
        </ul>
    "#, saved_list_html);
    
    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);
    let sidebar_js = crate::elements::sidebar::get_js();

    let content = format!(r#"
    <div class="req-container">

        {sidebar_html}
        
        <div class="main-area">
            <!-- Request Bar -->
            <div class="request-bar">
                <select id="method" class="method-select">
                    <option value="GET">GET</option>
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="DELETE">DELETE</option>
                    <option value="PATCH">PATCH</option>
                </select>
                <input type="text" id="url" class="url-input" placeholder="Enter request URL" value="">
                <button id="send-btn" class="send-btn btn-small">Send</button>
                <button id="cancel-btn" class="save-btn btn-small" disabled>Cancel</button>
                <button id="toggle-save-btn" class="save-btn btn-small">Save</button>
            </div>

            <!-- Save Form (Hidden by default) -->
            <form method="POST" action="/requests/save" class="save-controls" id="save-controls">
                <input type="text" name="name" id="req-name" placeholder="Request Name" required>
                <input type="hidden" name="method" id="save-method">
                <input type="hidden" name="url" id="save-url">
                <input type="hidden" name="headers" id="save-headers">
                <input type="hidden" name="body" id="save-body">
                <input type="hidden" name="auth_type" id="save-auth-type">
                <input type="hidden" name="oauth_token_url" id="save-oauth-token-url">
                <input type="hidden" name="oauth_client_id" id="save-oauth-client-id">
                <input type="hidden" name="oauth_client_secret" id="save-oauth-client-secret">
                <input type="hidden" name="oauth_scope" id="save-oauth-scope">
                <button type="submit" class="save-btn btn-small">Confirm Save</button>
            </form>
            
            <!-- Wrapper for Input Section to control height -->
            <div class="input-container">
                <!-- Input Tabs -->
                <div class="tabs">
                    <div class="tab active" onclick="openTab('tab-params')">Params</div>
                    <div class="tab" onclick="openTab('tab-path')">Path Variables</div>
                    <div class="tab" onclick="openTab('tab-auth')">Auth</div>
                    <div class="tab" onclick="openTab('tab-headers')">Headers</div>
                    <div class="tab" onclick="openTab('tab-body')">Body</div>
                </div>

                <!-- Params Tab -->
                <div id="tab-params" class="tab-content active">
                    <p style="font-size:var(--font-size-small); color:#888; margin:0;">Query Parameters</p>
                    <div id="params-container">
                        <!-- Dynamic Rows -->
                    </div>
                    <button class="save-btn btn-small" onclick="addKvRow('params-container')">+ Add Param</button>
                </div>

                <!-- Path Tab -->
                <div id="tab-path" class="tab-content">
                    <p style="font-size:var(--font-size-small); color:#888; margin:0;">Path Variables (Auto-detected from URL like {{id}})</p>
                    <div id="path-container"></div>
                </div>

                <!-- Auth Tab -->
                <div id="tab-auth" class="tab-content">
                    <div class="auth-section">
                        <div class="auth-row">
                            <label>Type</label>
                            <select id="auth-type">
                                <option value="none">No Auth</option>
                                <option value="bearer">Bearer Token</option>
                                <option value="basic">Basic Auth</option>
                                <option value="apikey">API Key</option>
                                <option value="oauth2">OAuth 2.0</option> <!-- NEW -->
                            </select>
                        </div>
                        <div id="auth-inputs"></div>
                    </div>
                </div>
                
                <!-- Headers Tab -->
                <div id="tab-headers" class="tab-content">
                    <p style="font-size:var(--font-size-small); color:#888; margin:0;">HTTP Headers</p>
                    <div id="headers-container"></div>
                    <button class="save-btn btn-small" onclick="addKvRow('headers-container')">+ Add Header</button>
                </div>
                
                <!-- Body Tab -->
                <div id="tab-body" class="tab-content">
                    <!-- Body Type Selector -->
                    <div class="body-type-row">
                        <label class="title">Type:</label>
                        <label><input type="radio" name="body-type" value="raw" checked onchange="toggleBodyType()"> Raw (JSON)</label>
                        <label><input type="radio" name="body-type" value="form" onchange="toggleBodyType()"> Form URL Encoded</label>
                    </div>

                    <!-- Raw Editor -->
                    <div id="body-raw-container" style="display:flex; flex-direction:column; flex-grow:1;">
                        <textarea id="body-input" class="code-editor" placeholder="{{ \"key\": \"value\" }}"></textarea>
                        <button type="button" id="format-json-btn" class="save-btn btn-small" style="width: auto; align-self: flex-start; margin-top:5px;">Format JSON</button>
                    </div>

                    <!-- Form URL Encoded Editor -->
                    <div id="body-form-container" style="display:none; flex-direction:column;">
                        <div id="form-body-rows"></div>
                        <button class="save-btn btn-small" onclick="addKvRow('form-body-rows')">+ Add Field</button>
                    </div>
                </div>
            </div>
            
            <!-- Response -->
            <!-- Response -->
            <div id="response-resizer" class="resizer-h" title="Drag to resize"></div>
            <div class="response-section" id="response-section">
                <div class="response-header">
                    <h3 style="margin:0; font-size:var(--font-size-medium);">Response</h3>
                    <div class="response-meta">
                        <span id="res-status">Status: -</span>
                        <span id="res-time">Time: - ms</span>
                        <span id="res-size">Size: -</span>
                    </div>
                    <button id="download-res-btn" class="save-btn btn-small">JSON</button>
                </div>
                
                <div id="request-debug-info"></div>
                <pre id="response-headers">Response headers will appear here...</pre>
                <div id="headers-body-resizer" title="Drag to resize headers/body"></div>
                <div id="response-body">Response body will appear here...</div>
            </div>
        </div>
    </div>

    <script>
        const methodSelect = document.getElementById('method');
        const urlInput = document.getElementById('url');
        const bodyInput = document.getElementById('body-input');
        const sendBtn = document.getElementById('send-btn');
        const cancelBtn = document.getElementById('cancel-btn');
        const responseBody = document.getElementById('response-body');
        const responseHeaders = document.getElementById('response-headers');
        const headersBodyResizer = document.getElementById('headers-body-resizer');
        const resStatus = document.getElementById('res-status');
        const resTime = document.getElementById('res-time');
        const resSize = document.getElementById('res-size');
        const downloadResBtn = document.getElementById('download-res-btn');
        const requestDebugInfo = document.getElementById('request-debug-info');
        const RESPONSE_HEIGHT_KEY = 'request-response-height';
        const RESPONSE_HEADERS_HEIGHT_KEY = 'request-response-headers-height';
        
        // Save Logic Elements
        const toggleSaveBtn = document.getElementById('toggle-save-btn');
        const saveControls = document.getElementById('save-controls');
        const saveMethod = document.getElementById('save-method');
        const saveUrl = document.getElementById('save-url');
        const saveHeaders = document.getElementById('save-headers');
        const saveBody = document.getElementById('save-body');
        const saveAuthType = document.getElementById('save-auth-type');
        const saveOAuthTokenUrl = document.getElementById('save-oauth-token-url');
        const saveOAuthClientId = document.getElementById('save-oauth-client-id');
        const saveOAuthClientSecret = document.getElementById('save-oauth-client-secret');
        const saveOAuthScope = document.getElementById('save-oauth-scope');

        // Auth Elements
        const authTypeSelect = document.getElementById('auth-type');
        const authInputs = document.getElementById('auth-inputs');
        let fetchedOAuthToken = ''; // Store the token here
        let currentRequestId = null;
        let currentAbortController = null;

        // --- Helper: Create Key-Value Row ---
        function addKvRow(containerId, key = '', val = '', isReadOnlyKey = false) {{
            const container = document.getElementById(containerId);
            const row = document.createElement('div');
            row.className = 'kv-row';
            const removeBtn = isReadOnlyKey ? '' : `<button class="kv-remove" onclick="this.parentElement.remove(); onKvChange('${{containerId}}')">x</button>`;
            const readOnlyAttr = isReadOnlyKey ? 'readonly' : '';
            const keyClass = isReadOnlyKey ? 'kv-input key readonly' : 'kv-input key';
            
            row.innerHTML = `
                <input type="text" class="${{keyClass}}" placeholder="Key" value="${{key}}" ${{readOnlyAttr}} oninput="onKvChange('${{containerId}}')">
                <input type="text" class="kv-input val" placeholder="Value" value="${{val}}" oninput="onKvChange('${{containerId}}')">
                ${{removeBtn}}
            `;
            container.appendChild(row);
        }}

        function getKvMap(containerId) {{
            const container = document.getElementById(containerId);
            const map = {{}};
            container.querySelectorAll('.kv-row').forEach(row => {{
                const k = row.querySelector('.key').value.trim();
                const v = row.querySelector('.val').value.trim();
                if(k) map[k] = v;
            }});
            return map;
        }}

        function getKvPairs(containerId) {{
            const container = document.getElementById(containerId);
            const pairs = [];
            container.querySelectorAll('.kv-row').forEach(row => {{
                const k = row.querySelector('.key').value.trim();
                const v = row.querySelector('.val').value.trim();
                if (k) pairs.push([k, v]);
            }});
            return pairs;
        }}

        // --- Body Type Toggle ---
        function toggleBodyType() {{
            const type = document.querySelector('input[name="body-type"]:checked').value;
            if (type === 'raw') {{
                document.getElementById('body-raw-container').style.display = 'flex';
                document.getElementById('body-form-container').style.display = 'none';
            }} else {{
                document.getElementById('body-raw-container').style.display = 'none';
                document.getElementById('body-form-container').style.display = 'flex';
                if(document.getElementById('form-body-rows').children.length === 0) {{
                     addKvRow('form-body-rows');
                }}
            }}
        }}
        
        // --- Sync URL <-> Params ---
        function onKvChange(containerId) {{
            if (containerId === 'params-container') {{
                updateUrlFromParams();
            }}
        }}

        function updateUrlFromParams() {{
            try {{
                const parts = urlInput.value.split('?');
                const baseUrl = parts[0];
                const params = getKvPairs('params-container');
                const search = new URLSearchParams();
                params.forEach(([k, v]) => search.append(k, v));
                const queryString = search.toString();
                if (queryString) {{
                    urlInput.value = `${{baseUrl}}?${{queryString}}`;
                }} else {{
                    urlInput.value = baseUrl;
                }}
                detectPathVariables();
            }} catch(e) {{}}
        }}

        function parseUrlToParams() {{
            try {{
                let urlStr = urlInput.value;
                if (!urlStr.startsWith('http')) urlStr = 'http://placeholder.com' + (urlStr.startsWith('/') ? '' : '/') + urlStr;
                
                const urlObj = new URL(urlStr);
                const container = document.getElementById('params-container');
                container.innerHTML = ''; 
                urlObj.searchParams.forEach((val, key) => {{
                    addKvRow('params-container', key, val);
                }});
                addKvRow('params-container'); 
            }} catch (e) {{}}
        }}

        function detectPathVariables() {{
            const url = urlInput.value;
            const regex = /\{{([^}}]+)\}}/g;
            let match;
            const foundKeys = new Set();
            while ((match = regex.exec(url)) !== null) {{ foundKeys.add(match[1]); }}

            const container = document.getElementById('path-container');
            const currentValues = getKvMap('path-container');
            container.innerHTML = '';

            if (foundKeys.size === 0) {{
                container.innerHTML = '<p style="padding:10px; color:#888;">No path variables detected.</p>';
                return;
            }}

            foundKeys.forEach(key => {{
                const val = currentValues[key] || '';
                addKvRow('path-container', key, val, true);
            }});
        }}
        
        urlInput.addEventListener('input', () => {{ parseUrlToParams(); detectPathVariables(); }});

        // --- Auth UI ---
        authTypeSelect.addEventListener('change', () => renderAuthInputs());

        function renderAuthInputs(savedData = null) {{
            const type = authTypeSelect.value;
            let html = '';
            // Helper to get value securely
            const val = (key) => savedData ? (savedData[key] || '') : '';

            // Pre-fill specific OAuth defaults for testing
            const defTokenUrl = val('oauth_token_url') || '';
            const defClientId = val('oauth_client_id') || '';
            const defClientSecret = val('oauth_client_secret') || '';
            const defScope = val('oauth_scope') || 'event_write';

            if (type === 'bearer') {{
                html = `<div class="auth-row"><label>Token</label><input type="text" id="auth-bearer-token" placeholder="Bearer Token" value="${{fetchedOAuthToken}}"></div>`;
            }} else if (type === 'basic') {{
                html = `
                    <div class="auth-row"><label>Username</label><input type="text" id="auth-basic-user"></div>
                    <div class="auth-row"><label>Password</label><input type="password" id="auth-basic-pass"></div>
                `;
            }} else if (type === 'apikey') {{
                html = `
                    <div class="auth-row"><label>Key</label><input type="text" id="auth-api-key" placeholder="Key Name (e.g. X-API-Key)"></div>
                    <div class="auth-row"><label>Value</label><input type="text" id="auth-api-val" placeholder="Key Value"></div>
                    <div class="auth-row"><label>Add To</label><select id="auth-api-loc"><option value="header">Header</option></select></div>
                `;
            }} else if (type === 'oauth2') {{
                html = `
                    <div class="auth-row"><label>Token URL</label><input type="text" id="oauth-token-url" value="${{defTokenUrl}}"></div>
                    <div class="auth-row"><label>Client ID</label><input type="text" id="oauth-client-id" value="${{defClientId}}"></div>
                    <div class="auth-row"><label>Client Secret</label><input type="password" id="oauth-client-secret" value="${{defClientSecret}}"></div>
                    <div class="auth-row"><label>Scope</label><input type="text" id="oauth-scope" value="${{defScope}}"></div>
                    <div class="auth-row">
                        <button type="button" class="oauth-btn" onclick="fetchOAuthToken()">Get New Access Token</button>
                    </div>
                    <div class="auth-row" id="oauth-status-row" style="display:none; flex-direction:column; align-items:flex-start;">
                         <label>Current Token</label>
                         <div class="token-display" id="oauth-token-display"></div>
                    </div>
                `;
            }}
            authInputs.innerHTML = html;
        }}
        
        async function fetchOAuthToken() {{
            const tokenUrl = document.getElementById('oauth-token-url').value;
            const clientId = document.getElementById('oauth-client-id').value;
            const clientSecret = document.getElementById('oauth-client-secret').value;
            const scope = document.getElementById('oauth-scope').value;
            const display = document.getElementById('oauth-token-display');
            const statusRow = document.getElementById('oauth-status-row');
            
            display.innerText = "Fetching...";
            statusRow.style.display = 'flex';
            
            // Construct JSON payload for this specific API structure
            const payload = {{
                clientId: clientId,
                clientSecret: clientSecret,
                scopes: [scope], // Using array format as requested
                grant_type: 'client_credentials'
            }};
            
             // Create CURL debug command string
            const debugCurl = `curl -X POST "${{tokenUrl}}" \\\n  -H "Content-Type: application/json" \\\n  -d '${{JSON.stringify(payload)}}'`;

            // Display debug info immediately
            display.innerHTML = `<div style="white-space: pre-wrap; margin-bottom: 10px; color: #888; border-bottom: 1px solid #444; padding-bottom: 5px; font-size: var(--font-size-small); overflow-x: auto;">${{debugCurl}}</div><div id="token-status-msg">Fetching...</div>`;
            
            try {{
                const resp = await fetch('/requests/run', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        method: 'POST',
                        url: tokenUrl,
                        headers: [{{ key: 'Content-Type', value: 'application/json' }}],
                        body: JSON.stringify(payload)
                    }})
                }});
                const run = await resp.json();
                const cleanJson = (run.body || '').trim();
                
                const msgDiv = document.getElementById('token-status-msg');

                try {{
                    const data = JSON.parse(cleanJson);
                    if (data.access_token) {{
                        fetchedOAuthToken = data.access_token;
                        // NEW: Show token in an input field with a copy button
                        msgDiv.innerHTML = `
                            <div style="color: #49cc90; margin-bottom: 5px;">Token received!</div>
                            <div style="display:flex; gap:5px; width:100%; margin-top:5px;">
                                <input type="text" id="token-input-field" value="${{fetchedOAuthToken}}" readonly style="flex-grow:1; padding:5px; background:var(--primary-bg); color:var(--text-color); border:1px solid var(--border-color); border-radius:0;">
                                <button type="button" class="save-btn" onclick="copyToClipboard('token-input-field')" style="padding:5px 10px;">Copy</button>
                            </div>
                        `;
                    }} else {{
                        msgDiv.innerText = "Error: No access_token in response.";
                        console.error(data);
                    }}
                }} catch(e) {{
                    msgDiv.innerText = "Error parsing response JSON.";
                    console.error("Parse error:", e);
                    console.log("Raw body:", run.body);
                }}
                
            }} catch (e) {{
                 const msgDiv = document.getElementById('token-status-msg');
                 if(msgDiv) msgDiv.innerText = "Error: " + e.message;
            }}
        }}

        function copyToClipboard(elementId) {{
            const copyText = document.getElementById(elementId);
            copyText.select();
            copyText.setSelectionRange(0, 99999);
            document.execCommand("copy");
        }}

        function constructHeaders() {{
            const headers = getKvPairs('headers-container');
            const authType = authTypeSelect.value;

            if (authType === 'bearer' || authType === 'oauth2') {{
                const token = authType === 'oauth2' ? fetchedOAuthToken : document.getElementById('auth-bearer-token')?.value;
                if(token) headers.push(['Authorization', `Bearer ${{token}}`]);
            }}
            if (authType === 'basic') {{
                const user = document.getElementById('auth-basic-user')?.value || '';
                const pass = document.getElementById('auth-basic-pass')?.value || '';
                headers.push(['Authorization', `Basic ${{btoa(`${{user}}:${{pass}}`)}}`]);
            }}
            if (authType === 'apikey') {{
                const key = document.getElementById('auth-api-key')?.value?.trim() || '';
                const val = document.getElementById('auth-api-val')?.value || '';
                if (key) headers.push([key, val]);
            }}
            return headers;
        }}

        function getRequestBodyAndHeaders(method, headerPairs) {{
            if (method === 'GET' || method === 'HEAD') return {{ body: '', headerPairs }};
            const bodyType = document.querySelector('input[name="body-type"]:checked')?.value || 'raw';
            if (bodyType === 'form') {{
                const formPairs = getKvPairs('form-body-rows');
                const encoded = new URLSearchParams(formPairs).toString();
                const hasContentType = headerPairs.some(([k]) => k.toLowerCase() === 'content-type');
                if (!hasContentType) {{
                    headerPairs.push(['Content-Type', 'application/x-www-form-urlencoded']);
                }}
                return {{ body: encoded, headerPairs }};
            }}
            return {{ body: bodyInput.value, headerPairs }};
        }}

        function headerPairsToPayload(headerPairs) {{
            return headerPairs.map(([key, value]) => ({{ key, value }}));
        }}

        function headersToString() {{
            const h = constructHeaders();
            return h.map(([k, v]) => `${{k}}: ${{v}}`).join('\\n');
        }}
        
        function stringToHeadersTable(headerStr) {{
            const container = document.getElementById('headers-container');
            container.innerHTML = '';
            if (!headerStr) return;
            const lines = headerStr.split('\\n');
            lines.forEach(line => {{
                const parts = line.split(':');
                if (parts.length >= 2) addKvRow('headers-container', parts[0].trim(), parts.slice(1).join(':').trim());
            }});
            addKvRow('headers-container'); 
        }}

        function getHeaderValueInsensitive(name) {{
            const target = (name || '').toLowerCase();
            const pairs = getKvPairs('headers-container');
            for (const [k, v] of pairs) {{
                if ((k || '').toLowerCase() === target) return v || '';
            }}
            return '';
        }}

        function inferAuthTypeFromHeaders() {{
            const authHeader = getHeaderValueInsensitive('Authorization');
            if (!authHeader) return 'none';
            const lower = authHeader.toLowerCase();
            if (lower.startsWith('bearer ')) return 'bearer';
            if (lower.startsWith('basic ')) return 'basic';
            return 'apikey';
        }}

        function applyAuthDefaultsFromHeaders(authType) {{
            const authHeader = getHeaderValueInsensitive('Authorization');
            if (!authHeader) return;

            if (authType === 'bearer') {{
                const token = authHeader.replace(/^Bearer\s+/i, '').trim();
                const tokenInput = document.getElementById('auth-bearer-token');
                if (tokenInput && token) tokenInput.value = token;
            }}

            if (authType === 'basic') {{
                const raw = authHeader.replace(/^Basic\s+/i, '').trim();
                try {{
                    const decoded = atob(raw);
                    const idx = decoded.indexOf(':');
                    const user = idx >= 0 ? decoded.slice(0, idx) : decoded;
                    const pass = idx >= 0 ? decoded.slice(idx + 1) : '';
                    const userInput = document.getElementById('auth-basic-user');
                    const passInput = document.getElementById('auth-basic-pass');
                    if (userInput) userInput.value = user;
                    if (passInput) passInput.value = pass;
                }} catch (_) {{}}
            }}
        }}

        function openTab(id) {{
            document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.tab').forEach(el => el.classList.remove('active'));
            document.getElementById(id).classList.add('active');
            const tabs = document.querySelectorAll('.tab');
            for(let t of tabs) {{
                if(t.getAttribute('onclick').includes(id)) t.classList.add('active');
            }}
        }}
        
        // Init
        window.addKvRow = addKvRow; window.openTab = openTab; window.onKvChange = onKvChange; window.fetchOAuthToken = fetchOAuthToken; window.toggleBodyType = toggleBodyType; window.copyToClipboard = copyToClipboard;
        addKvRow('params-container'); addKvRow('headers-container'); parseUrlToParams(); detectPathVariables();

        toggleSaveBtn.addEventListener('click', () => {{
            saveControls.style.display = saveControls.style.display === 'flex' ? 'none' : 'flex';
            if (saveControls.style.display === 'flex') {{
                document.getElementById('req-name').focus();
            }}
        }});

        saveControls.addEventListener('submit', () => {{
            saveMethod.value = methodSelect.value;
            saveUrl.value = urlInput.value;
            const headers = constructHeaders();
            const reqPayload = getRequestBodyAndHeaders(methodSelect.value, headers);
            saveHeaders.value = reqPayload.headerPairs.map(([k, v]) => `${{k}}: ${{v}}`).join('\\n');
            saveBody.value = reqPayload.body;
            saveAuthType.value = authTypeSelect.value;
            
            if (authTypeSelect.value === 'oauth2') {{
                saveOAuthTokenUrl.value = document.getElementById('oauth-token-url')?.value || '';
                saveOAuthClientId.value = document.getElementById('oauth-client-id')?.value || '';
                saveOAuthClientSecret.value = document.getElementById('oauth-client-secret')?.value || '';
                saveOAuthScope.value = document.getElementById('oauth-scope')?.value || '';
            }}
        }});

        document.getElementById('saved-list').addEventListener('click', (e) => {{
            const link = e.target.closest('.req-link');
            if (link) {{
                e.preventDefault();
                
                // Toggle selection class
                document.querySelectorAll('.saved-req-item').forEach(el => el.classList.remove('selected'));
                link.closest('.saved-req-item').classList.add('selected');

                methodSelect.value = link.dataset.method;
                urlInput.value = link.dataset.url;
                stringToHeadersTable(link.dataset.headers);
                bodyInput.value = link.dataset.body;
                document.getElementById('req-name').value = link.dataset.name; 
                
                let savedAuthType = link.dataset.authType || 'none';
                if (savedAuthType === 'none') {{
                    const inferred = inferAuthTypeFromHeaders();
                    if (inferred !== 'none') savedAuthType = inferred;
                }}
                authTypeSelect.value = savedAuthType;
                const savedAuthData = {{
                    oauth_token_url: link.dataset.oauthTokenUrl,
                    oauth_client_id: link.dataset.oauthClientId,
                    oauth_client_secret: link.dataset.oauthClientSecret,
                    oauth_scope: link.dataset.oauthScope
                }};
                renderAuthInputs(savedAuthData);
                applyAuthDefaultsFromHeaders(savedAuthType);
                parseUrlToParams(); 
                detectPathVariables();
            }}
        }});

        document.getElementById('format-json-btn').addEventListener('click', () => {{
            try {{ bodyInput.value = JSON.stringify(JSON.parse(bodyInput.value), null, 4); }} catch(e) {{ alert('Invalid JSON'); }}
        }});

        sendBtn.addEventListener('click', async () => {{
            responseBody.innerText = 'Loading...';
            responseHeaders.innerText = 'Loading headers...';
            resStatus.innerText = 'Status: -';
            resTime.innerText = 'Time: -';
            resSize.innerText = 'Size: -';
            requestDebugInfo.innerHTML = ''; // Clear old debug info
            currentAbortController = new AbortController();
            currentRequestId = `${{Date.now()}}-${{Math.random().toString(16).slice(2)}}`;
            cancelBtn.disabled = false;
            
            const startTime = performance.now();
            
            // 1. Substitute Path Variables
            let finalUrl = urlInput.value;
            const pathMap = getKvMap('path-container');
            for (const [key, val] of Object.entries(pathMap)) {{
                finalUrl = finalUrl.split(`{{${{key}}}}`).join(val);
            }}

            const requestParts = getRequestBodyAndHeaders(methodSelect.value, constructHeaders());
            const headers = requestParts.headerPairs;
            const body = requestParts.body;

            const options = {{
                method: methodSelect.value,
                url: finalUrl, // Use substituted URL
                headers: headers,
                body: ''
            }};

            let curlCmd = `curl -X ${{methodSelect.value}} "${{finalUrl}}"`;
            for (const [key, val] of headers) {{
                curlCmd += ` \\\n  -H "${{key}}: ${{val}}"`;
            }}
            
            if (methodSelect.value !== 'GET' && methodSelect.value !== 'HEAD') {{
                options.body = body;
                if (body) {{
                    // Simple escape for single quotes for display purposes
                    const safeBody = body.replace(/'/g, "'\\''");
                    curlCmd += ` \\\n  -d '${{safeBody}}'`;
                }}
            }}
            
            // Display the debug info
            requestDebugInfo.style.display = 'block';
            requestDebugInfo.innerText = curlCmd;

            try {{
                const resp = await fetch('/requests/run', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    signal: currentAbortController.signal,
                    body: JSON.stringify({{
                        ...options,
                        headers: headerPairsToPayload(headers),
                        request_id: currentRequestId
                    }})
                }});
                
                if (!resp.ok) {{
                    const errText = await resp.text();
                    throw new Error(errText || 'Request proxy failed');
                }}

                const run = await resp.json();
                const duration = run.duration_ms ?? (performance.now() - startTime).toFixed(0);
                const statusCode = Number(run.status || 0);
                const ok = run.curl_exit === 0 && statusCode >= 200 && statusCode < 400;

                resStatus.innerText = `Status: ${{statusCode || '0'}}`;
                resStatus.className = 'status-badge ' + (ok ? 'success' : 'error');
                resTime.innerText = `Time: ${{duration}} ms`;
                responseHeaders.innerText = run.headers || '(no headers)';

                const bodyText = run.body || '';
                resSize.innerText = 'Size: ' + (bodyText.length / 1024).toFixed(2) + ' KB';

                if (run.curl_exit !== 0) {{
                    responseBody.innerText = (run.stderr || 'Request failed').trim();
                    return;
                }}

                try {{
                    const json = JSON.parse(bodyText);
                    responseBody.innerText = JSON.stringify(json, null, 4);
                }} catch(e) {{
                    responseBody.innerText = bodyText;
                }}
                
            }} catch (err) {{
                responseHeaders.innerText = '';
                if (err && err.name === 'AbortError') {{
                    responseBody.innerText = 'Request cancelled.';
                    resStatus.innerText = 'Status: cancelled';
                }} else {{
                    responseBody.innerText = 'Error: ' + err.message;
                    resStatus.innerText = 'Error';
                }}
                resStatus.className = 'status-badge error';
            }} finally {{
                cancelBtn.disabled = true;
                currentRequestId = null;
                currentAbortController = null;
            }}
        }});

        cancelBtn.addEventListener('click', async () => {{
            const requestId = currentRequestId;
            if (!requestId) return;
            if (currentAbortController) currentAbortController.abort();
            cancelBtn.disabled = true;
            try {{
                await fetch('/requests/cancel', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ request_id: requestId }})
                }});
            }} catch (_) {{}}
        }});
        
        downloadResBtn.addEventListener('click', () => {{
            const content = responseBody.innerText;
            if (!content) return;
            
            const blob = new Blob([content], {{ type: 'application/json' }});
            const link = document.createElement('a');
            link.href = URL.createObjectURL(blob);
            link.download = 'response.json';
            link.click();
            URL.revokeObjectURL(link.href);
        }});

        // Response Resizing Only
        const respSection = document.getElementById('response-section');
        const respResizer = document.getElementById('response-resizer');
        let isRespResizing = false;
        let isHeadersResizing = false;

        // Restore persisted heights
        const savedRespHeight = localStorage.getItem(RESPONSE_HEIGHT_KEY);
        if (savedRespHeight) {{
            respSection.style.height = savedRespHeight;
        }}
        const savedHeadersHeight = localStorage.getItem(RESPONSE_HEADERS_HEIGHT_KEY);
        if (savedHeadersHeight) {{
            responseHeaders.style.height = savedHeadersHeight;
        }}
        
        respResizer.addEventListener('mousedown', (e) => {{
            isRespResizing = true;
            respResizer.classList.add('resizing');
            document.body.style.cursor = 'row-resize';
            document.body.style.userSelect = 'none';
        }});

        headersBodyResizer.addEventListener('mousedown', (e) => {{
            isHeadersResizing = true;
            headersBodyResizer.classList.add('resizing');
            document.body.style.cursor = 'row-resize';
            document.body.style.userSelect = 'none';
        }});

        document.addEventListener('mousemove', (e) => {{
            if (isRespResizing) {{
                const containerHeight = document.querySelector('.main-area').offsetHeight;
                
                const distFromBottom = window.innerHeight - e.clientY - 20; // 20 padding
                if (distFromBottom > 50 && distFromBottom < containerHeight - 100) {{
                    respSection.style.height = distFromBottom + 'px';
                }}
            }}

            if (isHeadersResizing) {{
                const respRect = respSection.getBoundingClientRect();
                const topOffset = e.clientY - respRect.top;
                const headerOffset = document.querySelector('.response-header').offsetHeight;
                const debugOffset = requestDebugInfo.style.display === 'none' ? 0 : requestDebugInfo.offsetHeight;
                const minHeaders = 50;
                const minBody = 80;
                const maxHeaders = respRect.height - headerOffset - debugOffset - minBody - headersBodyResizer.offsetHeight;
                const nextHeaders = Math.max(minHeaders, Math.min(topOffset - headerOffset - debugOffset, maxHeaders));
                responseHeaders.style.height = `${{Math.floor(nextHeaders)}}px`;
            }}
        }});

        document.addEventListener('mouseup', (e) => {{
            if (isRespResizing) {{
                isRespResizing = false;
                respResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem(RESPONSE_HEIGHT_KEY, respSection.style.height);
            }}

            if (isHeadersResizing) {{
                isHeadersResizing = false;
                headersBodyResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem(RESPONSE_HEADERS_HEIGHT_KEY, responseHeaders.style.height);
            }}
        }});
        
        // Inject Shared Sidebar JS
        {sidebar_js}
    </script>
    "#, sidebar_html = sidebar_html, sidebar_js = sidebar_js);

    render_base_page("Request Builder", &format!("{}{}{}", style, crate::elements::sidebar::get_css(), content), current_theme, saved_themes)
}
