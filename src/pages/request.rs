use actix_web::{get, post, web::{self, Data, Form, Json}, HttpResponse, Responder};
use std::{fs, io, process::Command, collections::HashMap, time::{Instant, SystemTime, UNIX_EPOCH}, sync::{Mutex, OnceLock}};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;
use htmlescape::encode_minimal;

const REQUESTS_FILE: &str = "saved_requests.json";
const REQUEST_FOLDERS_FILE: &str = "saved_request_folders.json";
const REQUEST_VARIABLES_FILE: &str = "request_variables.json";
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
    #[serde(default)]
    folder: Option<String>,
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
    folder: Option<String>,
}

#[derive(Deserialize)]
struct DeleteRequestForm {
    name: String,
    folder: Option<String>,
}

#[derive(Deserialize)]
struct CreateRequestFolderForm {
    folder_name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct RequestVariableSet {
    name: String,
    #[serde(default)]
    values: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct RequestVariables {
    #[serde(default)]
    active_set: String,
    #[serde(default)]
    sets: Vec<RequestVariableSet>,
    #[serde(default)]
    global: HashMap<String, String>,
}

#[derive(Deserialize)]
struct PostmanImportPayload {
    collection: serde_json::Value,
    duplicate_mode: Option<String>,
}

#[derive(Serialize)]
struct PostmanImportResponse {
    imported: usize,
    folders: usize,
    variables: usize,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct ParsedPostmanRequest {
    request: SavedRequest,
    folder: Option<String>,
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

fn load_request_folders() -> Vec<String> {
    fs::read_to_string(REQUEST_FOLDERS_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_request_folders(folders: &[String]) -> io::Result<()> {
    let data = serde_json::to_string_pretty(folders)?;
    fs::write(REQUEST_FOLDERS_FILE, data)
}

fn normalize_folder(folder: Option<&str>) -> String {
    folder.unwrap_or("").trim().to_string()
}

fn request_identity_matches(request: &SavedRequest, name: &str, folder: Option<&str>) -> bool {
    request.name == name && normalize_folder(request.folder.as_deref()) == normalize_folder(folder)
}

fn load_request_variables() -> RequestVariables {
    let variables = fs::read_to_string(REQUEST_VARIABLES_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default();
    normalize_request_variables(variables)
}

fn save_request_variables(variables: &RequestVariables) -> io::Result<()> {
    let normalized = normalize_request_variables(variables.clone());
    let data = serde_json::to_string_pretty(&normalized)?;
    fs::write(REQUEST_VARIABLES_FILE, data)
}

fn normalize_request_variables(mut variables: RequestVariables) -> RequestVariables {
    if variables.sets.is_empty() && !variables.global.is_empty() {
        variables.sets.push(RequestVariableSet {
            name: "Default".to_string(),
            values: variables.global.clone(),
        });
    }

    let mut normalized_sets = Vec::new();
    for mut set in variables.sets {
        let name = set.name.trim();
        if name.is_empty() {
            continue;
        }
        let values = set
            .values
            .drain()
            .filter_map(|(key, value)| {
                let key = key.trim();
                if key.is_empty() {
                    None
                } else {
                    Some((key.to_string(), value))
                }
            })
            .collect::<HashMap<_, _>>();
        if let Some(existing_idx) = normalized_sets
            .iter()
            .position(|existing: &RequestVariableSet| existing.name.eq_ignore_ascii_case(name))
        {
            normalized_sets[existing_idx].values.extend(values);
        } else {
            normalized_sets.push(RequestVariableSet {
                name: name.to_string(),
                values,
            });
        }
    }

    let active_set = variables.active_set.trim();
    let active_set = if normalized_sets.iter().any(|set| set.name == active_set) {
        active_set.to_string()
    } else {
        normalized_sets
            .first()
            .map(|set| set.name.clone())
            .unwrap_or_default()
    };

    RequestVariables {
        active_set,
        sets: normalized_sets,
        global: HashMap::new(),
    }
}

fn upsert_variable_set(variables: &mut RequestVariables, name: &str, values: HashMap<String, String>) {
    let normalized_name = name.trim();
    if normalized_name.is_empty() {
        return;
    }

    if let Some(existing) = variables
        .sets
        .iter_mut()
        .find(|set| set.name.eq_ignore_ascii_case(normalized_name))
    {
        existing.values.extend(values);
        variables.active_set = existing.name.clone();
    } else {
        variables.sets.push(RequestVariableSet {
            name: normalized_name.to_string(),
            values,
        });
        variables.active_set = normalized_name.to_string();
    }
}

fn normalize_saved_folder(folder: Option<&String>) -> Option<String> {
    folder
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
    let folder = normalize_saved_folder(form.folder.as_ref());
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
        folder: folder.clone(),
    };

    if let Some(idx) = requests.iter().position(|r| {
        request_identity_matches(r, &new_req.name, folder.as_deref())
    }) {
        requests[idx] = new_req;
    } else {
        requests.push(new_req);
    }

    let _ = save_requests_to_file(&requests);
    
    HttpResponse::Found()
        .append_header(("Location", "/requests"))
        .finish()
}

#[post("/requests/folder")]
pub async fn request_create_folder(form: Form<CreateRequestFolderForm>) -> impl Responder {
    let folder_name = form.folder_name.trim();
    if !folder_name.is_empty() {
        let mut folders = load_request_folders();
        if !folders.iter().any(|folder| folder.eq_ignore_ascii_case(folder_name)) {
            folders.push(folder_name.to_string());
            folders.sort_by_key(|folder| folder.to_lowercase());
            let _ = save_request_folders(&folders);
        }
    }

    HttpResponse::Found()
        .append_header(("Location", "/requests"))
        .finish()
}

#[post("/requests/delete")]
pub async fn request_delete(form: Form<DeleteRequestForm>) -> impl Responder {
    let mut requests = load_requests();
    if let Some(idx) = requests.iter().position(|r| {
        request_identity_matches(r, &form.name, form.folder.as_deref())
    }) {
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

#[post("/requests/variables")]
pub async fn request_save_variables(payload: Json<RequestVariables>) -> impl Responder {
    let variables = normalize_request_variables(payload.into_inner());

    match save_request_variables(&variables) {
        Ok(_) => HttpResponse::Ok().json(normalize_request_variables(variables)),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to save variables: {err}")),
    }
}

#[post("/requests/import/postman")]
pub async fn request_import_postman(payload: Json<PostmanImportPayload>) -> impl Responder {
    let mut warnings = Vec::new();
    let duplicate_mode = payload.duplicate_mode.as_deref().unwrap_or("rename");
    let collection_name = payload
        .collection
        .get("info")
        .and_then(|info| info.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or("Postman Import")
        .trim()
        .to_string();

    let mut parsed = Vec::new();
    let collection_auth = payload.collection.get("auth");
    if let Some(items) = payload.collection.get("item").and_then(|items| items.as_array()) {
        parse_postman_items(
            items,
            &mut Vec::new(),
            collection_auth,
            &mut parsed,
            &mut warnings,
        );
    } else {
        return HttpResponse::BadRequest().body("Postman collection has no item array");
    }

    let mut imported_folders = load_request_folders();
    for parsed_request in &parsed {
        if let Some(folder) = &parsed_request.folder {
            if !imported_folders.iter().any(|existing| existing.eq_ignore_ascii_case(folder)) {
                imported_folders.push(folder.clone());
            }
        }
    }
    imported_folders.sort_by_key(|folder| folder.to_lowercase());

    let mut requests = load_requests();
    let mut imported_count = 0;
    for parsed_request in parsed {
        let mut request = parsed_request.request;
        let folder = request.folder.clone();
        if let Some(existing_idx) = requests.iter().position(|existing| {
            request_identity_matches(existing, &request.name, folder.as_deref())
        }) {
            match duplicate_mode {
                "skip" => continue,
                "overwrite" => requests[existing_idx] = request,
                _ => {
                    request.name = unique_request_name(&requests, &request.name, folder.as_deref());
                    requests.push(request);
                }
            }
        } else {
            requests.push(request);
        }
        imported_count += 1;
    }

    let mut request_variables = load_request_variables();
    let imported_variables = extract_postman_variables(&payload.collection, &collection_name);
    let variable_count = imported_variables.len();
    upsert_variable_set(&mut request_variables, &collection_name, imported_variables);

    if let Err(err) = save_requests_to_file(&requests) {
        return HttpResponse::InternalServerError().body(format!("Failed to save imported requests: {err}"));
    }
    if let Err(err) = save_request_folders(&imported_folders) {
        return HttpResponse::InternalServerError().body(format!("Failed to save request folders: {err}"));
    }
    if let Err(err) = save_request_variables(&request_variables) {
        return HttpResponse::InternalServerError().body(format!("Failed to save request variables: {err}"));
    }

    HttpResponse::Ok().json(PostmanImportResponse {
        imported: imported_count,
        folders: imported_folders.len(),
        variables: variable_count,
        warnings,
    })
}

fn parse_postman_items(
    items: &[serde_json::Value],
    folder_path: &mut Vec<String>,
    inherited_auth: Option<&serde_json::Value>,
    parsed: &mut Vec<ParsedPostmanRequest>,
    warnings: &mut Vec<String>,
) {
    for item in items {
        let item_name = item
            .get("name")
            .and_then(|name| name.as_str())
            .unwrap_or("Untitled")
            .trim();
        if item_name.is_empty() {
            continue;
        }

        let item_auth = item.get("auth").or(inherited_auth);
        if let Some(children) = item.get("item").and_then(|children| children.as_array()) {
            folder_path.push(item_name.to_string());
            parse_postman_items(children, folder_path, item_auth, parsed, warnings);
            folder_path.pop();
            continue;
        }

        let Some(request_value) = item.get("request") else {
            continue;
        };
        let folder = if folder_path.is_empty() {
            None
        } else {
            Some(folder_path.join(" / "))
        };
        let mut headers = postman_headers_to_lines(request_value.get("header"));
        apply_postman_auth(
            item_auth,
            request_value.get("url"),
            &mut headers,
            warnings,
            item_name,
        );
        let (body, body_warnings) = postman_body_to_string(request_value.get("body"), item_name);
        warnings.extend(body_warnings);

        let request = SavedRequest {
            name: item_name.to_string(),
            method: request_value
                .get("method")
                .and_then(|method| method.as_str())
                .unwrap_or("GET")
                .to_uppercase(),
            url: postman_url_to_string(request_value.get("url")),
            headers: headers.join("\n"),
            body,
            auth_type: postman_auth_type(item_auth).map(|value| value.to_string()),
            oauth_token_url: postman_oauth_value(item_auth, "accessTokenUrl"),
            oauth_client_id: postman_oauth_value(item_auth, "clientId"),
            oauth_client_secret: postman_oauth_value(item_auth, "clientSecret"),
            oauth_scope: postman_oauth_value(item_auth, "scope"),
            folder: folder.clone(),
        };
        parsed.push(ParsedPostmanRequest { request, folder });
    }
}

fn postman_headers_to_lines(headers: Option<&serde_json::Value>) -> Vec<String> {
    headers
        .and_then(|headers| headers.as_array())
        .map(|headers| {
            headers
                .iter()
                .filter(|header| !header.get("disabled").and_then(|value| value.as_bool()).unwrap_or(false))
                .filter_map(|header| {
                    let key = header.get("key").and_then(|key| key.as_str())?.trim();
                    if key.is_empty() {
                        return None;
                    }
                    let value = header.get("value").and_then(|value| value.as_str()).unwrap_or("");
                    Some(format!("{key}: {value}"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn postman_url_to_string(url: Option<&serde_json::Value>) -> String {
    let Some(url) = url else {
        return String::new();
    };
    if let Some(raw) = url.as_str() {
        return raw.to_string();
    }
    if let Some(raw) = url.get("raw").and_then(|raw| raw.as_str()) {
        return raw.to_string();
    }

    let protocol = url
        .get("protocol")
        .and_then(|protocol| protocol.as_str())
        .unwrap_or("https");
    let host = postman_string_or_array(url.get("host"), ".");
    let path = postman_string_or_array(url.get("path"), "/");
    let mut rendered = if host.is_empty() {
        path
    } else if path.is_empty() {
        format!("{protocol}://{host}")
    } else {
        format!("{protocol}://{host}/{path}")
    };

    let query = url
        .get("query")
        .and_then(|query| query.as_array())
        .map(|items| {
            items
                .iter()
                .filter(|item| !item.get("disabled").and_then(|value| value.as_bool()).unwrap_or(false))
                .filter_map(|item| {
                    let key = item.get("key").and_then(|key| key.as_str())?;
                    let value = item.get("value").and_then(|value| value.as_str()).unwrap_or("");
                    Some(format!("{}={}", percent_encode(key), percent_encode(value)))
                })
                .collect::<Vec<_>>()
                .join("&")
        })
        .unwrap_or_default();
    if !query.is_empty() {
        rendered.push('?');
        rendered.push_str(&query);
    }
    rendered
}

fn postman_string_or_array(value: Option<&serde_json::Value>, separator: &str) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.to_string(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| part.as_str())
            .collect::<Vec<_>>()
            .join(separator),
        _ => String::new(),
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn postman_body_to_string(body: Option<&serde_json::Value>, request_name: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let Some(body) = body else {
        return (String::new(), warnings);
    };
    match body.get("mode").and_then(|mode| mode.as_str()).unwrap_or("") {
        "raw" => (
            body.get("raw")
                .and_then(|raw| raw.as_str())
                .unwrap_or("")
                .to_string(),
            warnings,
        ),
        "urlencoded" => {
            let encoded = body
                .get("urlencoded")
                .and_then(|items| items.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| !item.get("disabled").and_then(|value| value.as_bool()).unwrap_or(false))
                        .filter_map(|item| {
                            let key = item.get("key").and_then(|key| key.as_str())?;
                            let value = item.get("value").and_then(|value| value.as_str()).unwrap_or("");
                            Some(format!("{}={}", percent_encode(key), percent_encode(value)))
                        })
                        .collect::<Vec<_>>()
                        .join("&")
                })
                .unwrap_or_default();
            (encoded, warnings)
        }
        "formdata" => {
            warnings.push(format!("{request_name}: imported form-data body as text; file fields are skipped"));
            let text = body
                .get("formdata")
                .and_then(|items| items.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter(|item| !item.get("disabled").and_then(|value| value.as_bool()).unwrap_or(false))
                        .filter_map(|item| {
                            if item.get("type").and_then(|value| value.as_str()) == Some("file") {
                                return None;
                            }
                            let key = item.get("key").and_then(|key| key.as_str())?;
                            let value = item.get("value").and_then(|value| value.as_str()).unwrap_or("");
                            Some(format!("{key}={value}"))
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            (text, warnings)
        }
        "graphql" => {
            let graphql = body.get("graphql").cloned().unwrap_or_default();
            let text = serde_json::to_string_pretty(&graphql).unwrap_or_default();
            (text, warnings)
        }
        "file" => {
            warnings.push(format!("{request_name}: file body import is not supported"));
            (String::new(), warnings)
        }
        _ => (String::new(), warnings),
    }
}

fn postman_auth_type(auth: Option<&serde_json::Value>) -> Option<&str> {
    match auth.and_then(|auth| auth.get("type")).and_then(|value| value.as_str()) {
        Some("bearer") => Some("bearer"),
        Some("basic") => Some("basic"),
        Some("apikey") => Some("apikey"),
        Some("oauth2") => Some("oauth2"),
        _ => None,
    }
}

fn postman_auth_array_value(auth: Option<&serde_json::Value>, auth_type: &str, key: &str) -> Option<String> {
    auth
        .and_then(|auth| auth.get(auth_type))
        .and_then(|items| items.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                if item.get("key").and_then(|item_key| item_key.as_str()) == Some(key) {
                    item.get("value").and_then(|value| match value {
                        serde_json::Value::String(text) => Some(text.to_string()),
                        _ => Some(value.to_string()),
                    })
                } else {
                    None
                }
            })
        })
}

fn postman_oauth_value(auth: Option<&serde_json::Value>, key: &str) -> Option<String> {
    postman_auth_array_value(auth, "oauth2", key)
}

fn apply_postman_auth(
    auth: Option<&serde_json::Value>,
    request_url: Option<&serde_json::Value>,
    headers: &mut Vec<String>,
    warnings: &mut Vec<String>,
    request_name: &str,
) {
    let Some(auth_type) = auth.and_then(|auth| auth.get("type")).and_then(|value| value.as_str()) else {
        return;
    };
    match auth_type {
        "bearer" => {
            if let Some(token) = postman_auth_array_value(auth, "bearer", "token") {
                headers.push(format!("Authorization: Bearer {token}"));
            }
        }
        "basic" => {
            let username = postman_auth_array_value(auth, "basic", "username").unwrap_or_default();
            let password = postman_auth_array_value(auth, "basic", "password").unwrap_or_default();
            if !username.is_empty() || !password.is_empty() {
                headers.push(format!("Authorization: Basic {{basic_auth:{username}:{password}}}"));
                warnings.push(format!("{request_name}: basic auth was imported as a placeholder header"));
            }
        }
        "apikey" => {
            let key = postman_auth_array_value(auth, "apikey", "key").unwrap_or_default();
            let value = postman_auth_array_value(auth, "apikey", "value").unwrap_or_default();
            let location = postman_auth_array_value(auth, "apikey", "in").unwrap_or_else(|| "header".to_string());
            if key.is_empty() {
                return;
            }
            if location == "query" {
                let raw_url = postman_url_to_string(request_url);
                let separator = if raw_url.contains('?') { "&" } else { "?" };
                headers.push(format!("X-Postman-Imported-Query-Auth: {raw_url}{separator}{}={}", percent_encode(&key), percent_encode(&value)));
                warnings.push(format!("{request_name}: query API key was noted in headers; add it to the URL if needed"));
            } else {
                headers.push(format!("{key}: {value}"));
            }
        }
        "oauth2" => {
            if let Some(token) = postman_auth_array_value(auth, "oauth2", "accessToken") {
                headers.push(format!("Authorization: Bearer {token}"));
            }
        }
        other => warnings.push(format!("{request_name}: auth type '{other}' is not fully supported")),
    }
}

fn extract_postman_variables(collection: &serde_json::Value, collection_name: &str) -> HashMap<String, String> {
    let mut variables = HashMap::new();
    if let Some(items) = collection.get("variable").and_then(|items| items.as_array()) {
        for item in items {
            let Some(key) = item.get("key").and_then(|key| key.as_str()).map(str::trim) else {
                continue;
            };
            if key.is_empty() {
                continue;
            }
            let value = item
                .get("value")
                .or_else(|| item.get("initialValue"))
                .and_then(|value| match value {
                    serde_json::Value::String(text) => Some(text.to_string()),
                    serde_json::Value::Null => Some(String::new()),
                    _ => Some(value.to_string()),
                })
                .unwrap_or_default();
            variables.insert(key.to_string(), value);
        }
    }
    if !variables.contains_key("collection_name") {
        variables.insert("collection_name".to_string(), collection_name.to_string());
    }
    variables
}

fn unique_request_name(requests: &[SavedRequest], base_name: &str, folder: Option<&str>) -> String {
    let mut count = 2;
    loop {
        let candidate = format!("{base_name} ({count})");
        if !requests.iter().any(|request| request_identity_matches(request, &candidate, folder)) {
            return candidate;
        }
        count += 1;
    }
}


// --- Rendering ---

fn render_request_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let saved_requests = load_requests();
    let request_variables_json = serde_json::to_string(&load_request_variables())
        .unwrap_or_else(|_| r#"{"global":{}}"#.to_string())
        .replace("</", "<\\/");
    let mut request_folders = load_request_folders();
    for request in &saved_requests {
        if let Some(folder) = request.folder.as_ref().filter(|folder| !folder.trim().is_empty()) {
            if !request_folders.iter().any(|existing| existing.eq_ignore_ascii_case(folder)) {
                request_folders.push(folder.clone());
            }
        }
    }
    request_folders.sort_by_key(|folder| folder.to_lowercase());

    let render_saved_request = |r: &SavedRequest| {
        let safe_name = encode_minimal(&r.name);
        let name_attr = htmlescape::encode_attribute(&r.name);
        let folder_attr = htmlescape::encode_attribute(r.folder.as_deref().unwrap_or(""));
        // Use standard strings with escaped quotes
        format!(r##"
                <li class="saved-req-item">
                    <div class="saved-req-link-wrap">
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
                        data-oauth-scope="{}"
                        data-folder="{}">{}</a>
                    </div>
                    <form method="POST" action="/requests/delete" class="delete-form">
                        <input type="hidden" name="name" value="{}">
                        <input type="hidden" name="folder" value="{}">
                        <button type="submit" class="btn-danger-text" title="Delete">×</button>
                    </form>
                </li>"##,
            r.method.to_lowercase(), r.method, 
            name_attr, 
            htmlescape::encode_attribute(&r.method), 
            htmlescape::encode_attribute(&r.url), 
            htmlescape::encode_attribute(&r.headers), 
            htmlescape::encode_attribute(&r.body), 
            r.auth_type.as_deref().unwrap_or("none"),
            htmlescape::encode_attribute(r.oauth_token_url.as_deref().unwrap_or("")),
            htmlescape::encode_attribute(r.oauth_client_id.as_deref().unwrap_or("")),
            htmlescape::encode_attribute(r.oauth_client_secret.as_deref().unwrap_or("")),
            htmlescape::encode_attribute(r.oauth_scope.as_deref().unwrap_or("")),
            folder_attr,
            safe_name,
            name_attr,
            folder_attr
        )
    };

    let mut saved_list_parts = Vec::new();
    let unfiled_requests = saved_requests
        .iter()
        .filter(|request| request.folder.as_deref().unwrap_or("").trim().is_empty())
        .map(render_saved_request)
        .collect::<Vec<_>>();
    if !unfiled_requests.is_empty() {
        saved_list_parts.push(
            r#"<li class="saved-req-folder" data-folder=""> <span class="saved-req-folder-toggle">▾</span><span class="saved-req-folder-name">Unfiled</span></li>"#.to_string()
        );
        saved_list_parts.extend(unfiled_requests);
    }
    for folder in &request_folders {
        let folder_attr = htmlescape::encode_attribute(folder);
        saved_list_parts.push(format!(
            r#"<li class="saved-req-folder" data-folder="{}"><span class="saved-req-folder-toggle">▾</span><span class="saved-req-folder-name">{}</span></li>"#,
            folder_attr,
            encode_minimal(folder)
        ));
        saved_list_parts.extend(
            saved_requests
                .iter()
                .filter(|request| request.folder.as_deref() == Some(folder.as_str()))
                .map(render_saved_request)
        );
    }
    let saved_list_html = saved_list_parts.join("\n");

    let request_folder_options = request_folders
        .iter()
        .map(|folder| {
            let folder_attr = htmlescape::encode_attribute(folder);
            let folder_safe = encode_minimal(folder);
            format!(r#"<option value="{folder_attr}">{folder_safe}</option>"#)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sidebar_content = format!(r#"
        <div class="requests-sidebar-fixed">
            <div class="requests-sidebar-heading">
                <h2 class="requests-sidebar-title">Saved Requests</h2>
                <div class="requests-sidebar-actions">
                    <button type="button" id="import-postman-btn" class="requests-sidebar-icon-btn" title="Import Postman collection" aria-label="Import Postman collection">
                        <svg class="requests-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M12 3v11M8 10l4 4 4-4M5 17.5V20h14v-2.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                    <button type="button" id="new-request-btn" class="requests-sidebar-icon-btn" title="New request" aria-label="New request">
                        <svg class="requests-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M6 3.5h7.5L18 8v12.5H6V3.5Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                            <path d="M13.5 3.5V8H18M12 11v6M9 14h6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                    </button>
                    <form id="create-request-folder-form" method="POST" action="/requests/folder" class="create-request-folder-form">
                        <input type="hidden" id="new-request-folder-name" name="folder_name">
                        <button type="button" id="create-request-folder-btn" class="requests-sidebar-icon-btn" title="New request folder" aria-label="New request folder">
                            <svg class="requests-sidebar-button-icon" viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M3 6.5A2.5 2.5 0 0 1 5.5 4h4.1l2 2H18.5A2.5 2.5 0 0 1 21 8.5v9A2.5 2.5 0 0 1 18.5 20h-13A2.5 2.5 0 0 1 3 17.5v-11Z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
                                <path d="M12 10v6M9 13h6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                            </svg>
                        </button>
                    </form>
                </div>
            </div>
            <div class="sidebar-search requests-sidebar-search"><input type="text" id="saved-request-search" placeholder="Search requests..."></div>
        </div>
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
                <button id="request-variables-btn" class="save-btn btn-small">Variables</button>
            </div>

            <!-- Save Form (Hidden by default) -->
            <form method="POST" action="/requests/save" class="save-controls" id="save-controls">
                <input type="text" name="name" id="req-name" placeholder="Request Name" required>
                <select name="folder" id="req-folder" title="Request folder">
                    <option value="">Unfiled</option>
                    {request_folder_options}
                </select>
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
                    <div class="response-actions">
                        <button id="view-curl-btn" class="save-btn btn-small" type="button" disabled>View Curl</button>
                        <button id="download-res-btn" class="save-btn btn-small" type="button">JSON</button>
                    </div>
                </div>
                
                <div id="request-debug-info"></div>
                <pre id="response-headers">Response headers will appear here...</pre>
                <div id="headers-body-resizer" title="Drag to resize headers/body"></div>
                <div id="response-body">Response body will appear here...</div>
            </div>
        </div>
    </div>
    <dialog id="curl-view-modal" class="curl-view-modal">
        <form method="dialog" class="curl-view-dialog">
            <div class="curl-view-heading">
                <h3>Curl Equivalent</h3>
            </div>
            <pre id="curl-view-output">Run a request to generate curl.</pre>
            <div class="curl-view-actions">
                <button type="button" id="close-curl-view-btn" class="save-btn btn-small request-vars-cancel-btn">Close</button>
            </div>
        </form>
    </dialog>
    <dialog id="request-variables-modal" class="request-variables-modal">
        <form method="dialog" class="request-variables-dialog">
            <div class="request-variables-heading">
                <h3>Request Variables</h3>
                <p>Use values like <code>{{{{base_url}}}}</code> in URLs, headers, and bodies. The active set is used when the request runs.</p>
            </div>
            <label class="request-variable-set-row">
                <select id="request-variable-set-select"></select>
                <button type="button" id="add-request-variable-set-btn" class="save-btn btn-small">New Set</button>
                <button type="button" id="rename-request-variable-set-btn" class="save-btn btn-small">Rename</button>
                <button type="button" id="copy-request-variable-set-btn" class="save-btn btn-small">Copy</button>
                <button type="button" id="delete-request-variable-set-btn" class="save-btn btn-small">Delete</button>
            </label>
            <div class="request-vars-toolbar">
                <span>Variables</span>
                <button type="button" class="save-btn btn-small" id="add-request-variable-btn">+ Add Variable</button>
            </div>
            <div id="request-variables-container"></div>
            <div id="request-variables-status" class="request-vars-status"></div>
            <div class="request-variables-actions">
                <button type="button" id="close-request-variables-btn" class="save-btn btn-small request-vars-cancel-btn">Cancel</button>
                <button type="button" class="save-btn btn-small" id="save-request-variables-btn">Save</button>
            </div>
        </form>
    </dialog>
    <dialog id="new-request-variable-set-modal" class="request-new-variable-set-modal">
        <form method="dialog" class="request-new-variable-set-dialog">
            <div class="request-variables-heading">
                <h3 id="request-variable-set-modal-title">New Variable Set</h3>
                <p id="request-variable-set-modal-description">Name the set, then add variables for that environment.</p>
            </div>
            <label class="request-variable-set-name-row">
                <span>Set Name</span>
                <input type="text" id="new-request-variable-set-name" placeholder="Local, Staging, Production">
            </label>
            <div class="request-variables-actions">
                <button type="button" id="cancel-request-variable-set-btn" class="save-btn btn-small request-vars-cancel-btn">Cancel</button>
                <button type="button" id="create-request-variable-set-btn" class="save-btn btn-small">Create</button>
            </div>
        </form>
    </dialog>
    <dialog id="postman-import-modal" class="postman-import-modal">
        <form method="dialog" class="postman-import-dialog">
            <div class="postman-import-heading">
                <h3>Import Postman Collection</h3>
                <p>Import requests, folders, and collection variables from a Postman JSON export.</p>
            </div>
            <label class="postman-import-field">
                <span>Collection JSON</span>
                <input type="file" id="postman-import-file" accept=".json,application/json">
            </label>
            <label class="postman-import-field">
                <span>Duplicates</span>
                <select id="postman-duplicate-mode">
                    <option value="rename">Rename imported copies</option>
                    <option value="overwrite">Overwrite matching folder/name</option>
                    <option value="skip">Skip matching folder/name</option>
                </select>
            </label>
            <div id="postman-import-preview" class="postman-import-preview">Choose a Postman collection export to preview it.</div>
            <div class="postman-import-actions">
                <button type="button" id="close-postman-import-btn" class="save-btn btn-small postman-cancel-btn">Cancel</button>
                <button type="button" id="confirm-postman-import-btn" class="save-btn btn-small" disabled>Import</button>
            </div>
        </form>
    </dialog>
    <script type="application/json" id="request-variables-data">{request_variables_json}</script>
    <script src="/static/requests.js" defer></script>
    "#, sidebar_html = sidebar_html, request_folder_options = request_folder_options, request_variables_json = request_variables_json);

    render_base_page("Request Builder", &format!(r#"<link rel="stylesheet" href="/static/requests.css">{}"#, content), current_theme, saved_themes)
}
