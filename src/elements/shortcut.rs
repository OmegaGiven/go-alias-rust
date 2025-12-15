use actix_web::{
    post,
    web::{Data, Form}, 
    HttpResponse, Responder,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io,
    sync::Arc,
};

use crate::app_state::AppState;

// File constants
static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static WORK_SHORTCUTS_FILE: &str = "work-shortcuts.json"; // Added constant for work shortcuts file

// Struct to capture the shortcut form data
#[derive(Deserialize)]
pub struct AddShortcutForm {
    pub shortcut: String,
    pub url: String,
    pub hidden: Option<String>,
}

// Struct to capture the key for deletion
#[derive(Deserialize)]
pub struct DeleteShortcutForm {
    pub key: String,
}

// Helper function to save shortcuts back to JSON file
fn save_shortcuts(path: &str, shortcuts: &HashMap<String, String>) -> io::Result<()> {
    // Use serde_json::to_string_pretty for readable JSON
    let data = serde_json::to_string_pretty(shortcuts)?;
    fs::write(path, data)
}

// Handler for the new shortcut form
#[post("/add_shortcut")]
pub async fn add_shortcut(
    form: Form<AddShortcutForm>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    let is_hidden = form.hidden.is_some();
    let shortcut = form.shortcut.trim();
    let url = form.url.trim();

    // Basic validation
    if shortcut.is_empty() || url.is_empty() {
        return HttpResponse::BadRequest().body("Shortcut and URL cannot be empty.");
    }

    if is_hidden {
        // Add to hidden shortcuts
        let mut hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
        hidden_shortcuts.insert(shortcut.to_string(), url.to_string());
        
        // Persist to disk
        if let Err(e) = save_shortcuts(HIDDEN_SHORTCUTS_FILE, &hidden_shortcuts) {
            eprintln!("Failed to save hidden shortcuts: {}", e);
            return HttpResponse::InternalServerError().body("Failed to save hidden shortcut.");
        }
    } else {
        // Add to visible shortcuts (using the general 'shortcuts.json' as the default visible file)
        let mut shortcuts = state.shortcuts.lock().unwrap();
        shortcuts.insert(shortcut.to_string(), url.to_string());

        // Persist to disk
        if let Err(e) = save_shortcuts(SHORTCUTS_FILE, &shortcuts) {
            eprintln!("Failed to save shortcuts: {}", e);
            return HttpResponse::InternalServerError().body("Failed to save shortcut.");
        }
    }

    // Redirect back to the home page
    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}


// NEW: Handler for deleting a shortcut
#[post("/delete_shortcut")]
pub async fn delete_shortcut(
    form: Form<DeleteShortcutForm>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    let key = form.key.trim();
    if key.is_empty() {
        return HttpResponse::BadRequest().body("Shortcut key cannot be empty.");
    }

    // Attempt to delete from all three collections and save if successful.
    // We check `work_shortcuts` and `hidden_shortcuts` before `shortcuts` 
    // to ensure proper file persistence logic is isolated.

    // 1. Check and delete from work shortcuts
    {
        let mut work_shortcuts = state.work_shortcuts.lock().unwrap();
        if work_shortcuts.remove(key).is_some() {
            if let Err(e) = save_shortcuts(WORK_SHORTCUTS_FILE, &work_shortcuts) {
                eprintln!("Failed to save work shortcuts after deletion: {}", e);
            }
        }
    }
    
    // 2. Check and delete from hidden shortcuts
    {
        let mut hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
        if hidden_shortcuts.remove(key).is_some() {
            if let Err(e) = save_shortcuts(HIDDEN_SHORTCUTS_FILE, &hidden_shortcuts) {
                eprintln!("Failed to save hidden shortcuts after deletion: {}", e);
            }
        }
    }
    
    // 3. Check and delete from visible shortcuts
    {
        let mut shortcuts = state.shortcuts.lock().unwrap();
        if shortcuts.remove(key).is_some() {
            if let Err(e) = save_shortcuts(SHORTCUTS_FILE, &shortcuts) {
                eprintln!("Failed to save visible shortcuts after deletion: {}", e);
            }
        }
    }

    // Redirect back to the home page
    HttpResponse::Found().append_header(("Location", "/")).finish()
}