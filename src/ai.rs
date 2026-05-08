use actix_web::{HttpResponse, Responder, get, post, web};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, io};

use crate::app_db::{self, AiProviderSettingsRecord};

const AI_KEY_FILE: &str = "ai_settings.key";
const NONCE_LEN: usize = 12;
const MAX_CONTEXT_CHARS: usize = 24_000;
const MAX_MESSAGE_CHARS: usize = 8_000;

#[derive(Debug, Deserialize)]
pub struct AiSettingsSaveRequest {
    id: Option<String>,
    name: Option<String>,
    provider: String,
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiActiveProfileRequest {
    id: String,
}

#[derive(Debug, Serialize)]
pub struct AiSettingsResponse {
    id: String,
    name: String,
    provider: String,
    model: String,
    base_url: String,
    has_api_key: bool,
    profiles: Vec<AiProfileResponse>,
}

#[derive(Debug, Serialize)]
pub struct AiProfileResponse {
    id: String,
    name: String,
    provider: String,
    model: String,
    base_url: String,
    has_api_key: bool,
    is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct AiChatRequest {
    message: String,
    page: Option<String>,
    context: Option<Value>,
    context_summary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiChatResponse {
    message: String,
    provider: String,
    model: String,
}

#[derive(Debug, Serialize)]
pub struct AiModelsResponse {
    provider: String,
    models: Vec<String>,
}

#[get("/ai/settings")]
pub async fn ai_settings_get() -> impl Responder {
    let profiles = app_db::list_ai_provider_settings().await;
    let active = profiles
        .iter()
        .find(|settings| settings.is_active)
        .or_else(|| profiles.first());
    let response = match active {
        Some(settings) => AiSettingsResponse {
            id: settings.id.clone(),
            name: ai_profile_name(settings),
            provider: settings.provider.clone(),
            model: settings.model.clone(),
            base_url: settings.base_url.clone(),
            has_api_key: !settings.encrypted_api_key.trim().is_empty(),
            profiles: profiles.iter().map(ai_profile_response).collect(),
        },
        None => AiSettingsResponse {
            id: "default".to_string(),
            name: "Default".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4.1-mini".to_string(),
            base_url: String::new(),
            has_api_key: false,
            profiles: Vec::new(),
        },
    };

    HttpResponse::Ok().json(response)
}

#[post("/ai/settings")]
pub async fn ai_settings_save(payload: web::Json<AiSettingsSaveRequest>) -> impl Responder {
    let provider = normalize_provider(&payload.provider);
    let model = payload.model.trim();
    let id = normalize_profile_id(payload.id.as_deref().unwrap_or("default"));
    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Default");
    if provider.is_empty() || model.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "Provider and model are required."
        }));
    }
    if id.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "AI agent name is required."
        }));
    }

    let existing = app_db::get_ai_provider_settings_by_id(&id).await;
    let encrypted_api_key = if provider == "ollama" {
        String::new()
    } else {
        match payload.api_key.as_deref().map(str::trim) {
            Some("") | None => existing
                .map(|settings| settings.encrypted_api_key)
                .unwrap_or_default(),
            Some(api_key) => match encrypt_secret(api_key.as_bytes()) {
                Ok(value) => value,
                Err(err) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": format!("Failed to encrypt API key: {err}")
                    }));
                }
            },
        }
    };

    let settings = AiProviderSettingsRecord {
        id,
        name: name.to_string(),
        provider,
        model: model.to_string(),
        base_url: payload
            .base_url
            .clone()
            .unwrap_or_default()
            .trim()
            .to_string(),
        encrypted_api_key,
        is_active: true,
    };

    match app_db::save_ai_provider_settings(&settings).await {
        Ok(()) => HttpResponse::Ok().json(json!({
            "status": "saved",
            "id": settings.id,
            "has_api_key": !settings.encrypted_api_key.is_empty()
        })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to save AI settings: {err}")
        })),
    }
}

#[post("/ai/settings/active")]
pub async fn ai_settings_active(payload: web::Json<AiActiveProfileRequest>) -> impl Responder {
    let id = normalize_profile_id(&payload.id);
    if id.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "AI agent is required." }));
    }

    if app_db::get_ai_provider_settings_by_id(&id).await.is_none() {
        return HttpResponse::NotFound().json(json!({ "error": "AI agent was not found." }));
    }

    match app_db::set_active_ai_provider_settings(&id).await {
        Ok(()) => HttpResponse::Ok().json(json!({ "status": "active", "id": id })),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "error": format!("Failed to activate AI agent: {err}")
        })),
    }
}

fn ai_profile_name(settings: &AiProviderSettingsRecord) -> String {
    let name = settings.name.trim();
    if name.is_empty() {
        settings.id.clone()
    } else {
        name.to_string()
    }
}

fn ai_profile_response(settings: &AiProviderSettingsRecord) -> AiProfileResponse {
    AiProfileResponse {
        id: settings.id.clone(),
        name: ai_profile_name(settings),
        provider: settings.provider.clone(),
        model: settings.model.clone(),
        base_url: settings.base_url.clone(),
        has_api_key: !settings.encrypted_api_key.trim().is_empty(),
        is_active: settings.is_active,
    }
}

#[post("/ai/test")]
pub async fn ai_test() -> impl Responder {
    let Some(settings) = load_settings_with_key().await else {
        return HttpResponse::BadRequest().json(json!({
            "error": "Save AI provider settings before testing."
        }));
    };

    let messages = vec![
        json!({"role": "system", "content": "Reply with a short confirmation that the AI provider is reachable."}),
        json!({"role": "user", "content": "Say OK for OGdevDesk."}),
    ];

    match call_provider(&settings, messages).await {
        Ok(message) => HttpResponse::Ok().json(json!({ "status": "ok", "message": message })),
        Err(err) => HttpResponse::BadGateway().json(json!({ "error": err })),
    }
}

#[get("/ai/models")]
pub async fn ai_models_get() -> impl Responder {
    let Some(settings) = load_settings_with_key().await else {
        return HttpResponse::BadRequest().json(json!({
            "error": "Save AI provider settings before loading models."
        }));
    };

    match list_provider_models(&settings).await {
        Ok(models) => HttpResponse::Ok().json(AiModelsResponse {
            provider: settings.provider,
            models,
        }),
        Err(err) => HttpResponse::BadGateway().json(json!({ "error": err })),
    }
}

#[post("/ai/chat")]
pub async fn ai_chat(payload: web::Json<AiChatRequest>) -> impl Responder {
    let Some(settings) = load_settings_with_key().await else {
        return HttpResponse::BadRequest().json(json!({
            "error": "Save AI provider settings before using the assistant."
        }));
    };

    let user_message = truncate_chars(payload.message.trim(), MAX_MESSAGE_CHARS);
    if user_message.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "error": "Message is required." }));
    }

    let context = payload
        .context
        .as_ref()
        .map(redact_value)
        .unwrap_or_else(|| json!({}));
    let context_text = serde_json::to_string_pretty(&context).unwrap_or_else(|_| "{}".to_string());
    let context_text = truncate_chars(&context_text, MAX_CONTEXT_CHARS);
    let page = payload.page.as_deref().unwrap_or("unknown");
    let context_summary = truncate_chars(payload.context_summary.as_deref().unwrap_or(""), 1200);

    let system_prompt = r#"You are the read-only AI Assistant inside OGdevDesk, a developer tool for SQL, HTTP requests, JSON inspection, web aliases, and scratch notes.
You can use only the context included in the prompt. You must not claim that you ran a query, sent a request, changed settings, saved data, or inspected secrets.
Help write SQL, explain schemas or errors, design HTTP requests, interpret responses, and suggest next steps.
Warn clearly before destructive SQL such as UPDATE, DELETE, DROP, TRUNCATE, ALTER, or INSERT.
If context contains redacted values, do not ask the user to reveal secrets unless absolutely necessary. Keep answers practical and concise."#;

    let user_prompt = format!(
        "Current page: {page}\nContext summary: {context_summary}\n\nIncluded context JSON:\n{context_text}\n\nUser request:\n{user_message}"
    );

    let messages = vec![
        json!({"role": "system", "content": system_prompt}),
        json!({"role": "user", "content": user_prompt}),
    ];

    match call_provider(&settings, messages).await {
        Ok(message) => HttpResponse::Ok().json(AiChatResponse {
            message,
            provider: settings.provider,
            model: settings.model,
        }),
        Err(err) => HttpResponse::BadGateway().json(json!({ "error": err })),
    }
}

struct RuntimeSettings {
    provider: String,
    model: String,
    base_url: String,
    api_key: String,
}

async fn load_settings_with_key() -> Option<RuntimeSettings> {
    let settings = app_db::get_ai_provider_settings().await?;
    let provider = normalize_provider(&settings.provider);
    let api_key = decrypt_secret(&settings.encrypted_api_key).ok()?;
    if api_key.trim().is_empty() && provider != "ollama" {
        return None;
    }

    Some(RuntimeSettings {
        provider,
        model: settings.model,
        base_url: settings.base_url,
        api_key,
    })
}

async fn call_provider(settings: &RuntimeSettings, messages: Vec<Value>) -> Result<String, String> {
    match settings.provider.as_str() {
        "gemini" => call_gemini(settings, messages).await,
        "ollama" => call_ollama(settings, messages).await,
        "openai" | "custom_openai_compatible" | "anthropic" => {
            call_openai_compatible(settings, messages).await
        }
        _ => Err("Unsupported AI provider.".to_string()),
    }
}

async fn list_provider_models(settings: &RuntimeSettings) -> Result<Vec<String>, String> {
    match settings.provider.as_str() {
        "gemini" => list_gemini_models(settings).await,
        "ollama" => list_ollama_models(settings).await,
        "openai" | "custom_openai_compatible" => list_openai_compatible_models(settings).await,
        "anthropic" => Err(
            "Model discovery is not implemented for Anthropic yet. Enter the model manually."
                .to_string(),
        ),
        _ => Err("Unsupported AI provider.".to_string()),
    }
}

async fn list_ollama_models(settings: &RuntimeSettings) -> Result<Vec<String>, String> {
    let endpoint = format!("{}/api/tags", ollama_base_url(settings));
    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|err| format!("Ollama model list request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("Ollama model list response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    let mut models: Vec<String> = body
        .get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    models.dedup();
    Ok(models)
}

async fn list_openai_compatible_models(settings: &RuntimeSettings) -> Result<Vec<String>, String> {
    let mut base_url = openai_base_url(settings);
    if base_url.ends_with("/chat/completions") {
        base_url = base_url.trim_end_matches("/chat/completions").to_string();
    }
    let endpoint = if base_url.ends_with("/models") {
        base_url
    } else {
        format!("{base_url}/models")
    };

    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .bearer_auth(&settings.api_key)
        .send()
        .await
        .map_err(|err| format!("Model list request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("Model list response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    let mut models: Vec<String> = body
        .get("data")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    models.dedup();
    Ok(models)
}

async fn list_gemini_models(settings: &RuntimeSettings) -> Result<Vec<String>, String> {
    let base_url = gemini_base_url(settings);
    let endpoint = format!("{base_url}/models?key={}", settings.api_key);

    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|err| format!("Gemini model list request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("Gemini model list response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    let mut models: Vec<String> = body
        .get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let can_generate = item
                        .get("supportedGenerationMethods")
                        .and_then(Value::as_array)
                        .map(|methods| {
                            methods
                                .iter()
                                .any(|method| method.as_str() == Some("generateContent"))
                        })
                        .unwrap_or(true);
                    if !can_generate {
                        return None;
                    }

                    item.get("name")
                        .and_then(Value::as_str)
                        .map(|name| name.strip_prefix("models/").unwrap_or(name).to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    models.dedup();
    Ok(models)
}

fn openai_base_url(settings: &RuntimeSettings) -> String {
    if settings.base_url.trim().is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        settings.base_url.trim().trim_end_matches('/').to_string()
    }
}

fn gemini_base_url(settings: &RuntimeSettings) -> String {
    if settings.base_url.trim().is_empty() {
        "https://generativelanguage.googleapis.com/v1beta".to_string()
    } else {
        settings.base_url.trim().trim_end_matches('/').to_string()
    }
}

fn ollama_base_url(settings: &RuntimeSettings) -> String {
    if settings.base_url.trim().is_empty() {
        "http://localhost:11434".to_string()
    } else {
        settings.base_url.trim().trim_end_matches('/').to_string()
    }
}

async fn call_openai_compatible(
    settings: &RuntimeSettings,
    messages: Vec<Value>,
) -> Result<String, String> {
    let base_url = openai_base_url(settings);
    let endpoint = if base_url.ends_with("/chat/completions") {
        base_url
    } else {
        format!("{base_url}/chat/completions")
    };

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .bearer_auth(&settings.api_key)
        .json(&json!({
            "model": settings.model,
            "messages": messages,
            "temperature": 0.2
        }))
        .send()
        .await
        .map_err(|err| format!("AI request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("AI response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    body.pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "AI provider returned no assistant message.".to_string())
}

async fn call_ollama(settings: &RuntimeSettings, messages: Vec<Value>) -> Result<String, String> {
    let endpoint = format!("{}/api/chat", ollama_base_url(settings));
    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .json(&json!({
            "model": settings.model,
            "messages": messages,
            "stream": false
        }))
        .send()
        .await
        .map_err(|err| format!("Ollama request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("Ollama response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    body.pointer("/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "Ollama returned no assistant message.".to_string())
}

async fn call_gemini(settings: &RuntimeSettings, messages: Vec<Value>) -> Result<String, String> {
    let base_url = gemini_base_url(settings);
    let endpoint = format!(
        "{base_url}/models/{}:generateContent?key={}",
        settings.model, settings.api_key
    );

    let mut text = String::new();
    for message in messages {
        if let Some(content) = message.get("content").and_then(Value::as_str) {
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(content);
        }
    }

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .json(&json!({
            "contents": [{
                "role": "user",
                "parts": [{ "text": text }]
            }],
            "generationConfig": {
                "temperature": 0.2
            }
        }))
        .send()
        .await
        .map_err(|err| format!("Gemini request failed: {err}"))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|err| format!("Gemini response was not JSON: {err}"))?;
    if !status.is_success() {
        return Err(provider_error_message(status.as_u16(), &body));
    }

    body.pointer("/candidates/0/content/parts/0/text")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| "Gemini returned no assistant message.".to_string())
}

fn provider_error_message(status: u16, body: &Value) -> String {
    let message = body
        .pointer("/error/message")
        .or_else(|| body.pointer("/error"))
        .and_then(Value::as_str)
        .unwrap_or("Provider returned an error.");
    format!("AI provider error {status}: {message}")
}

fn normalize_provider(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "gemini" | "google" | "google_gemini" => "gemini".to_string(),
        "ollama" | "ollama_local" | "local_ollama" => "ollama".to_string(),
        "custom" | "openai_compatible" | "custom_openai_compatible" => {
            "custom_openai_compatible".to_string()
        }
        "anthropic" | "claude" => "anthropic".to_string(),
        _ => "openai".to_string(),
    }
}

fn normalize_profile_id(id: &str) -> String {
    let normalized: String = id
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    normalized.trim_matches('-').to_string()
}

fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let redacted = map
                .iter()
                .map(|(key, value)| {
                    let next = if is_sensitive_key(key) {
                        Value::String("[redacted]".to_string())
                    } else {
                        redact_value(value)
                    };
                    (key.clone(), next)
                })
                .collect();
            Value::Object(redacted)
        }
        Value::Array(values) => Value::Array(values.iter().map(redact_value).collect()),
        Value::String(text) if looks_like_secret(text) => Value::String("[redacted]".to_string()),
        _ => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lowered = key.to_lowercase();
    lowered.contains("authorization")
        || lowered.contains("cookie")
        || lowered.contains("token")
        || lowered.contains("secret")
        || lowered.contains("password")
        || lowered.contains("api_key")
        || lowered.contains("apikey")
        || lowered.contains("x-api-key")
}

fn looks_like_secret(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("Bearer ") || trimmed.starts_with("Basic ") || looks_like_jwt(trimmed)
}

fn looks_like_jwt(value: &str) -> bool {
    if value.chars().any(char::is_whitespace) {
        return false;
    }

    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            part.len() > 10
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '=')
        })
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars).collect();
    truncated.push_str("\n[truncated]");
    truncated
}

fn load_or_create_key() -> io::Result<Vec<u8>> {
    if let Ok(key) = fs::read(AI_KEY_FILE) {
        if key.len() == 32 {
            return Ok(key);
        }
    }

    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    fs::write(AI_KEY_FILE, &key)?;
    Ok(key)
}

fn encrypt_secret(plaintext: &[u8]) -> io::Result<String> {
    let key = load_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "bad key length"))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "encryption failure"))?;

    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(to_hex(&blob))
}

fn decrypt_secret(encrypted_hex: &str) -> io::Result<String> {
    if encrypted_hex.trim().is_empty() {
        return Ok(String::new());
    }

    let data = from_hex(encrypted_hex)?;
    if data.len() <= NONCE_LEN {
        return Ok(String::new());
    }

    let key = load_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "bad key length"))?;
    let nonce = Nonce::from_slice(&data[..NONCE_LEN]);
    let plaintext = cipher
        .decrypt(nonce, &data[NONCE_LEN..])
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "decryption failure"))?;

    String::from_utf8(plaintext)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn from_hex(input: &str) -> io::Result<Vec<u8>> {
    let input = input.trim();
    if input.len() % 2 != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "odd hex length"));
    }

    let mut bytes = Vec::with_capacity(input.len() / 2);
    for index in (0..input.len()).step_by(2) {
        let byte = u8::from_str_radix(&input[index..index + 2], 16)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
        bytes.push(byte);
    }
    Ok(bytes)
}
