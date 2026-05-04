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
    let sidebar_content = format!(r#"
        <h2>Saved Requests</h2>
        <ul class="saved-list" id="saved-list">
            {}
        </ul>
    "#, saved_list_html);
    
    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);

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
                    <p style="font-size:var(--font-size-small); color:#888; margin: calc(var(--element-margin) / 2) var(--element-margin);">Query Parameters</p>
                    <div id="params-container">
                        <!-- Dynamic Rows -->
                    </div>
                    <button class="save-btn btn-small" onclick="addKvRow('params-container')">+ Add Param</button>
                </div>

                <!-- Path Tab -->
                <div id="tab-path" class="tab-content">
                    <p style="font-size:var(--font-size-small); color:#888; margin: calc(var(--element-margin) / 2) var(--element-margin);">Path Variables (Auto-detected from URL like {{id}})</p>
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
                    <p style="font-size:var(--font-size-small); color:#888; margin: calc(var(--element-margin) / 2) var(--element-margin);">HTTP Headers</p>
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
                        <button type="button" id="format-json-btn" class="save-btn btn-small" style="width: auto; align-self: flex-start; margin: calc(var(--element-margin) / 2) var(--element-margin);">Format JSON</button>
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
                    <h3 style="margin: calc(var(--element-margin) / 2) var(--element-margin); font-size:var(--font-size-medium);">Response</h3>
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
    <script src="/static/requests.js" defer></script>
    "#, sidebar_html = sidebar_html);

    render_base_page("Request Builder", &format!(r#"<link rel="stylesheet" href="/static/requests.css">{}"#, content), current_theme, saved_themes)
}
