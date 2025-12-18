use actix_web::{get, post, web::{self, Data, Form, Json}, HttpResponse, Responder};
use std::{fs, io, process::Command, collections::HashMap};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;
use htmlescape::encode_minimal;

const REQUESTS_FILE: &str = "saved_requests.json";

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
    headers: HashMap<String, String>,
    body: String,
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
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_request_page(&current_theme))
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
        let mut cmd = Command::new("curl");
        
        cmd.arg("-i").arg("-s").arg("-X").arg(&payload.method);

        for (key, value) in &payload.headers {
            cmd.arg("-H").arg(format!("{}: {}", key, value));
        }

        if !payload.body.is_empty() && payload.method != "GET" && payload.method != "HEAD" {
            cmd.arg("-d").arg(&payload.body);
        }

        cmd.arg(&payload.url);
        cmd.output()
    }).await;

    match res {
        Ok(Ok(output)) => {
            let result = String::from_utf8_lossy(&output.stdout).to_string();
            if result.is_empty() {
                let err = String::from_utf8_lossy(&output.stderr).to_string();
                HttpResponse::Ok().body(if err.is_empty() { "No response".to_string() } else { err })
            } else {
                HttpResponse::Ok().body(result)
            }
        },
        Ok(Err(e)) => HttpResponse::InternalServerError().body(format!("Failed to execute curl: {}", e)),
        _ => HttpResponse::InternalServerError().body("Blocked execution error"),
    }
}


// --- Rendering ---

fn render_request_page(current_theme: &Theme) -> String {
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
                        <button type="submit" class="btn-danger-text" title="Delete">x</button>
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
    /* FIX: Adjusted height calculation and ensures border box model */
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
    
    .saved-req-item {{ display: flex; align-items: center; justify-content: space-between; padding: 3px 5px; border-bottom: 1px solid var(--border-color); cursor: pointer; }}
    .saved-req-item:last-child {{ border-bottom: none; }}
    .saved-req-item:hover {{ background-color: var(--tertiary-bg); }}

    .req-method {{ font-size: 0.65em; font-weight: bold; padding: 1px 4px; border-radius: 2px; margin-right: 5px; min-width: 35px; text-align: center; color: #fff; flex-shrink: 0; }}
    .req-method.get {{ background-color: #61affe; }}
    .req-method.post {{ background-color: #49cc90; }}
    .req-method.put {{ background-color: #fca130; }}
    .req-method.delete {{ background-color: #f93e3e; }}
    .req-method.patch {{ background-color: #50e3c2; }}
    
    .req-link {{ text-decoration: none; color: var(--text-color); flex-grow: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 0.9em; min-width: 0; }}
    
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
    
    textarea.code-editor {{
        width: 100%; height: 100%; background: var(--secondary-bg); color: var(--text-color); border: 1px solid var(--border-color); border-radius: 4px; padding: 10px; font-family: monospace; box-sizing: border-box; resize: none;
    }}

    /* Key-Value Tables (Params & Headers) removed - now using shared .kv-row class */
    .kv-remove {{ background: none; border: none; color: #f93e3e; font-weight: bold; cursor: pointer; padding: 0 8px; }}
    
    /* Read-only key for Path Params */
    .kv-input.key.readonly {{ background-color: var(--tertiary-bg); color: #aaa; }}

    /* Auth Section removed - now using shared .form-group logic where possible */
    .auth-section {{ display: flex; flex-direction: column; gap: 10px; padding: 0; background: transparent; border: none; }}
    .auth-row {{ display: flex; gap: 10px; align-items: center; flex-wrap: wrap;}}
    .auth-row label {{ width: 100px; flex-shrink: 0; font-size: 0.9em; color: #aaa;}}
    .auth-row input, .auth-row select {{ flex: 1; padding: 5px; background: var(--tertiary-bg); border: 1px solid var(--border-color); color: var(--text-color); border-radius: 3px; }}
    .oauth-btn {{ background-color: #fca130; color: #000; border: none; padding: 6px 12px; border-radius: 3px; cursor: pointer; font-weight: bold; font-size: 0.9em; }}
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
    .response-meta {{ display: flex; gap: 15px; font-size: 0.85em; color: #aaa; flex-shrink: 0; margin: 0; }}
    
    /* Debug Info */
    #request-debug-info {{ margin-bottom: 5px; color: #888; font-family: monospace; font-size: 0.8em; white-space: pre-wrap; overflow-x: auto; display: none; background: var(--primary-bg); padding: 5px; border-bottom: 1px solid var(--border-color); flex-shrink: 0;}}

    #response-body {{ flex: 1; white-space: pre-wrap; overflow: auto; font-family: monospace; background: var(--primary-bg); padding: 10px; border: none; min-height: 0; color: var(--text-color); }}
    
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
                    <p style="font-size:0.8em; color:#888; margin:0;">Query Parameters</p>
                    <div id="params-container">
                        <!-- Dynamic Rows -->
                    </div>
                    <button class="save-btn btn-small" onclick="addKvRow('params-container')">+ Add Param</button>
                </div>

                <!-- Path Tab -->
                <div id="tab-path" class="tab-content">
                    <p style="font-size:0.8em; color:#888; margin:0;">Path Variables (Auto-detected from URL like {{id}})</p>
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
                    <p style="font-size:0.8em; color:#888; margin:0;">HTTP Headers</p>
                    <div id="headers-container"></div>
                    <button class="save-btn btn-small" onclick="addKvRow('headers-container')">+ Add Header</button>
                </div>
                
                <!-- Body Tab -->
                <div id="tab-body" class="tab-content">
                    <!-- Body Type Selector -->
                    <div style="display:flex; gap:15px; margin-bottom:10px; align-items:center;">
                        <label style="font-size:0.9em; color:#888;">Type:</label>
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
                    <h3 style="margin:0; font-size:1em;">Response</h3>
                    <div class="response-meta">
                        <span id="res-status">Status: -</span>
                        <span id="res-time">Time: - ms</span>
                        <span id="res-size">Size: -</span>
                    </div>
                    <button id="download-res-btn" class="save-btn btn-small">JSON</button>
                </div>
                
                <div id="request-debug-info"></div>
                <div id="response-body">Response body will appear here...</div>
            </div>
        </div>
    </div>

    <script>
        const methodSelect = document.getElementById('method');
        const urlInput = document.getElementById('url');
        const bodyInput = document.getElementById('body-input');
        const sendBtn = document.getElementById('send-btn');
        const responseBody = document.getElementById('response-body');
        const resStatus = document.getElementById('res-status');
        const resTime = document.getElementById('res-time');
        const resSize = document.getElementById('res-size');
        const downloadResBtn = document.getElementById('download-res-btn');
        const requestDebugInfo = document.getElementById('request-debug-info');
        
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
                const params = getKvMap('params-container');
                const queryString = new URLSearchParams(params).toString();
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

            // FIX: Pre-fill specific OAuth defaults for testing
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
            display.innerHTML = `<div style="white-space: pre-wrap; margin-bottom: 10px; color: #888; border-bottom: 1px solid #444; padding-bottom: 5px; font-size: 0.8em; overflow-x: auto;">${{debugCurl}}</div><div id="token-status-msg">Fetching...</div>`;
            
            try {{
                const resp = await fetch('/requests/run', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        method: 'POST',
                        url: tokenUrl,
                        headers: {{ 'Content-Type': 'application/json' }}, // Using JSON content type
                        body: JSON.stringify(payload)
                    }})
                }});
                
                const text = await resp.text();
                // Robust parsing of curl output (splitting headers/body by double newlines)
                const parts = text.split(/(?:\r\n\r\n|\n\n)/g); 
                const jsonStr = parts.length > 1 ? parts[parts.length - 1] : text;
                const cleanJson = jsonStr.trim();
                
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
                    console.log("Raw text:", text);
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
            const headers = getKvMap('headers-container');
            const authType = authTypeSelect.value;
            // ... (Auth injection logic same as before) ...
            if (authType === 'bearer' || authType === 'oauth2') {{
                const token = authType === 'oauth2' ? fetchedOAuthToken : document.getElementById('auth-bearer-token')?.value;
                if(token) headers['Authorization'] = `Bearer ${{token}}`;
            }}
            return headers;
        }}

        function headersToString() {{
            const h = constructHeaders();
            return Object.entries(h).map(([k, v]) => `${{k}}: ${{v}}`).join('\\n');
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
            saveHeaders.value = headersToString(); 
            saveBody.value = bodyInput.value;
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
                methodSelect.value = link.dataset.method;
                urlInput.value = link.dataset.url;
                stringToHeadersTable(link.dataset.headers);
                bodyInput.value = link.dataset.body;
                document.getElementById('req-name').value = link.dataset.name; 
                
                authTypeSelect.value = link.dataset.authType || 'none';
                const savedAuthData = {{
                    oauth_token_url: link.dataset.oauthTokenUrl,
                    oauth_client_id: link.dataset.oauthClientId,
                    oauth_client_secret: link.dataset.oauthClientSecret,
                    oauth_scope: link.dataset.oauthScope
                }};
                renderAuthInputs(savedAuthData);
                parseUrlToParams(); 
                detectPathVariables();
            }}
        }});

        document.getElementById('format-json-btn').addEventListener('click', () => {{
            try {{ bodyInput.value = JSON.stringify(JSON.parse(bodyInput.value), null, 4); }} catch(e) {{ alert('Invalid JSON'); }}
        }});

        sendBtn.addEventListener('click', async () => {{
            responseBody.innerText = 'Loading...';
            resStatus.innerText = 'Status: -';
            resTime.innerText = 'Time: -';
            requestDebugInfo.innerHTML = ''; // Clear old debug info
            
            const startTime = performance.now();
            
            // 1. Substitute Path Variables
            let finalUrl = urlInput.value;
            const pathMap = getKvMap('path-container');
            for (const [key, val] of Object.entries(pathMap)) {{
                finalUrl = finalUrl.replace(`{{${{key}}}}`, val);
            }}

            const headers = constructHeaders();
            const body = bodyInput.value;

            const options = {{
                method: methodSelect.value,
                url: finalUrl, // Use substituted URL
                headers: headers,
                body: ''
            }};

            // NEW: Build the curl command for display
            let curlCmd = `curl -X ${{methodSelect.value}} "${{finalUrl}}"`;
            for (const [key, val] of Object.entries(headers)) {{
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
                    body: JSON.stringify(options)
                }});
                
                const endTime = performance.now();
                const duration = (endTime - startTime).toFixed(0);
                
                resStatus.innerText = 'Status: ' + resp.status + ' ' + resp.statusText;
                resStatus.className = 'status-badge ' + (resp.ok ? 'success' : 'error');
                resTime.innerText = 'Time: ' + duration + ' ms';
                
                const text = await resp.text();
                resSize.innerText = 'Size: ' + (text.length / 1024).toFixed(2) + ' KB';

                // Try to strip headers from curl output if -i is used
                const parts = text.split(/(?:\r\n\r\n|\n\n)/g); 
                const bodyText = parts.length > 1 ? parts[parts.length - 1] : text;

                try {{
                    const json = JSON.parse(bodyText);
                    responseBody.innerText = JSON.stringify(json, null, 4);
                }} catch(e) {{
                    responseBody.innerText = bodyText;
                }}
                
            }} catch (err) {{
                responseBody.innerText = 'Error: ' + err.message;
                resStatus.innerText = 'Error';
                resStatus.className = 'status-badge error';
            }}
        }});
        
        // NEW: Download Response as JSON
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
        
        respResizer.addEventListener('mousedown', (e) => {{
            isRespResizing = true;
            respResizer.classList.add('resizing');
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
        }});

        document.addEventListener('mouseup', (e) => {{
            if (isRespResizing) {{
                isRespResizing = false;
                respResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
            }}
        }});
        
        // Inject Shared Sidebar JS
        {sidebar_js}
    </script>
    "#, sidebar_html = sidebar_html, sidebar_js = sidebar_js);

    render_base_page("Request Builder", &format!("{}{}{}", style, crate::elements::sidebar::get_css(), content), current_theme)
}