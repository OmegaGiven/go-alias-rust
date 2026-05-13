use actix_web::{
    HttpResponse, Responder, get, post,
    web::{Data, Form, Json},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io, sync::Arc};

use crate::app_db;
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
    pub scope: Option<String>,
    pub hidden: Option<String>,
}

// Struct to capture the key for deletion
#[derive(Deserialize)]
pub struct DeleteShortcutForm {
    pub key: String,
}

#[derive(Deserialize)]
pub struct CreateShortcutGroupForm {
    pub scope: Option<String>,
    pub group_name: String,
}

#[derive(Deserialize)]
pub struct MoveShortcutGroupForm {
    pub scope: Option<String>,
    pub key: String,
    pub group_name: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutServerExport {
    pub global: HashMap<String, String>,
    pub hidden_global: HashMap<String, String>,
    pub work_global: HashMap<String, String>,
    pub visible_groups: HashMap<String, String>,
    pub work_groups: HashMap<String, String>,
    pub visible_group_names: Vec<String>,
    pub work_group_names: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutExportResponse {
    pub version: u32,
    pub server: ShortcutServerExport,
}

// Helper function to save shortcuts back to JSON file
fn save_shortcuts(path: &str, shortcuts: &HashMap<String, String>) -> io::Result<()> {
    // Use serde_json::to_string_pretty for readable JSON
    let data = serde_json::to_string_pretty(shortcuts)?;
    fs::write(path, data)
}

fn merge_shortcut_map(target: &mut HashMap<String, String>, incoming: &HashMap<String, String>) {
    for (key, url) in incoming {
        let key = key.trim();
        let url = url.trim();
        if key.is_empty() || url.is_empty() {
            continue;
        }
        target.insert(key.to_string(), url.to_string());
    }
}

async fn persist_shortcut_scope(
    collection_key: &str,
    file_path: &str,
    shortcuts: &HashMap<String, String>,
) -> Result<(), String> {
    app_db::put_json("shortcuts", collection_key, shortcuts)
        .await
        .map_err(|err| format!("Failed to save aliases to app database: {err}"))?;
    save_shortcuts(file_path, shortcuts)
        .map_err(|err| format!("Failed to save aliases file {file_path}: {err}"))?;
    Ok(())
}

async fn import_shortcut_groups(
    scope: &str,
    group_map: &HashMap<String, String>,
    group_names: &[String],
) -> Result<(), String> {
    for group_name in group_names {
        let group_name = group_name.trim();
        if group_name.is_empty() {
            continue;
        }
        app_db::create_shortcut_group(scope, group_name)
            .await
            .map_err(|err| format!("Failed to create alias group {group_name}: {err}"))?;
    }

    for (key, group_name) in group_map {
        let key = key.trim();
        let group_name = group_name.trim();
        if key.is_empty() {
            continue;
        }
        app_db::set_shortcut_group(scope, key, group_name)
            .await
            .map_err(|err| format!("Failed to move alias {key} into group: {err}"))?;
    }

    Ok(())
}

#[get("/shortcuts/export")]
pub async fn export_shortcuts(state: Data<Arc<AppState>>) -> impl Responder {
    let global = state
        .shortcuts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let hidden_global = state
        .hidden_shortcuts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let work_global = state
        .work_shortcuts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();

    HttpResponse::Ok().json(ShortcutExportResponse {
        version: 1,
        server: ShortcutServerExport {
            global,
            hidden_global,
            work_global,
            visible_groups: app_db::get_shortcut_group_map("visible").await,
            work_groups: app_db::get_shortcut_group_map("work").await,
            visible_group_names: app_db::get_shortcut_groups("visible").await,
            work_group_names: app_db::get_shortcut_groups("work").await,
        },
    })
}

#[post("/shortcuts/import")]
pub async fn import_shortcuts(
    payload: Json<ShortcutServerExport>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    let incoming = payload.into_inner();

    let global = {
        let mut shortcuts = state
            .shortcuts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        merge_shortcut_map(&mut shortcuts, &incoming.global);
        shortcuts.clone()
    };
    if let Err(err) = persist_shortcut_scope("visible", SHORTCUTS_FILE, &global).await {
        return HttpResponse::InternalServerError().body(err);
    }

    let hidden_global = {
        let mut hidden_shortcuts = state
            .hidden_shortcuts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        merge_shortcut_map(&mut hidden_shortcuts, &incoming.hidden_global);
        hidden_shortcuts.clone()
    };
    if let Err(err) =
        persist_shortcut_scope("hidden", HIDDEN_SHORTCUTS_FILE, &hidden_global).await
    {
        return HttpResponse::InternalServerError().body(err);
    }

    let work_global = {
        let mut work_shortcuts = state
            .work_shortcuts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        merge_shortcut_map(&mut work_shortcuts, &incoming.work_global);
        work_shortcuts.clone()
    };
    if let Err(err) = persist_shortcut_scope("work", WORK_SHORTCUTS_FILE, &work_global).await {
        return HttpResponse::InternalServerError().body(err);
    }

    if let Err(err) = import_shortcut_groups(
        "visible",
        &incoming.visible_groups,
        &incoming.visible_group_names,
    )
    .await
    {
        return HttpResponse::InternalServerError().body(err);
    }

    if let Err(err) =
        import_shortcut_groups("work", &incoming.work_groups, &incoming.work_group_names).await
    {
        return HttpResponse::InternalServerError().body(err);
    }

    HttpResponse::Ok().body("ok")
}

// Handler for the new shortcut form
#[post("/add_shortcut")]
pub async fn add_shortcut(
    form: Form<AddShortcutForm>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    let scope = form.scope.as_deref().unwrap_or_else(|| {
        if form.hidden.is_some() {
            "hidden_global"
        } else {
            "global"
        }
    });
    let shortcut = form.shortcut.trim();
    let url = form.url.trim();

    // Basic validation
    if shortcut.is_empty() || url.is_empty() {
        return HttpResponse::BadRequest().body("Shortcut and URL cannot be empty.");
    }

    match scope {
        "hidden_global" => {
            let hidden_shortcuts = {
                let mut hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
                hidden_shortcuts.insert(shortcut.to_string(), url.to_string());
                hidden_shortcuts.clone()
            };

            if let Err(e) = app_db::put_json("shortcuts", "hidden", &hidden_shortcuts).await {
                eprintln!("Failed to save hidden shortcuts to app database: {}", e);
            }
            if let Err(e) = save_shortcuts(HIDDEN_SHORTCUTS_FILE, &hidden_shortcuts) {
                eprintln!("Failed to save hidden shortcuts: {}", e);
                return HttpResponse::InternalServerError().body("Failed to save hidden shortcut.");
            }
        }
        "global" => {
            let shortcuts = {
                let mut shortcuts = state.shortcuts.lock().unwrap();
                shortcuts.insert(shortcut.to_string(), url.to_string());
                shortcuts.clone()
            };

            if let Err(e) = app_db::put_json("shortcuts", "visible", &shortcuts).await {
                eprintln!("Failed to save shortcuts to app database: {}", e);
            }
            if let Err(e) = save_shortcuts(SHORTCUTS_FILE, &shortcuts) {
                eprintln!("Failed to save shortcuts: {}", e);
                return HttpResponse::InternalServerError().body("Failed to save shortcut.");
            }
        }
        "local" | "hidden_local" => {
            // Local shortcuts are stored in the browser and should not hit the server.
        }
        _ => {
            return HttpResponse::BadRequest().body("Invalid shortcut scope.");
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
    if let Some(work_shortcuts) = {
        let mut work_shortcuts = state.work_shortcuts.lock().unwrap();
        if work_shortcuts.remove(key).is_some() {
            Some(work_shortcuts.clone())
        } else {
            None
        }
    } {
        if let Err(e) = app_db::put_json("shortcuts", "work", &work_shortcuts).await {
            eprintln!("Failed to save work shortcuts to app database: {}", e);
        }
        if let Err(e) = save_shortcuts(WORK_SHORTCUTS_FILE, &work_shortcuts) {
            eprintln!("Failed to save work shortcuts after deletion: {}", e);
        }
    }

    // 2. Check and delete from hidden shortcuts
    if let Some(hidden_shortcuts) = {
        let mut hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
        if hidden_shortcuts.remove(key).is_some() {
            Some(hidden_shortcuts.clone())
        } else {
            None
        }
    } {
        if let Err(e) = app_db::put_json("shortcuts", "hidden", &hidden_shortcuts).await {
            eprintln!("Failed to save hidden shortcuts to app database: {}", e);
        }
        if let Err(e) = save_shortcuts(HIDDEN_SHORTCUTS_FILE, &hidden_shortcuts) {
            eprintln!("Failed to save hidden shortcuts after deletion: {}", e);
        }
    }

    // 3. Check and delete from visible shortcuts
    if let Some(shortcuts) = {
        let mut shortcuts = state.shortcuts.lock().unwrap();
        if shortcuts.remove(key).is_some() {
            Some(shortcuts.clone())
        } else {
            None
        }
    } {
        if let Err(e) = app_db::put_json("shortcuts", "visible", &shortcuts).await {
            eprintln!("Failed to save visible shortcuts to app database: {}", e);
        }
        if let Err(e) = save_shortcuts(SHORTCUTS_FILE, &shortcuts) {
            eprintln!("Failed to save visible shortcuts after deletion: {}", e);
        }
    }

    // Redirect back to the home page
    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}

#[post("/shortcut_group/create")]
pub async fn create_shortcut_group(form: Form<CreateShortcutGroupForm>) -> impl Responder {
    let scope = form.scope.as_deref().unwrap_or("visible");
    let group_name = form.group_name.trim();
    if group_name.is_empty() {
        return HttpResponse::BadRequest().body("Group name cannot be empty.");
    }

    if let Err(e) = app_db::create_shortcut_group(scope, group_name).await {
        eprintln!("Failed to create shortcut group: {}", e);
        return HttpResponse::InternalServerError().body("Failed to create shortcut group.");
    }

    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}

#[post("/shortcut_group/move")]
pub async fn move_shortcut_to_group(form: Form<MoveShortcutGroupForm>) -> impl Responder {
    let scope = form.scope.as_deref().unwrap_or("visible");
    let key = form.key.trim();
    if key.is_empty() {
        return HttpResponse::BadRequest().body("Shortcut key cannot be empty.");
    }

    if let Err(e) =
        app_db::set_shortcut_group(scope, key, form.group_name.as_deref().unwrap_or("")).await
    {
        eprintln!("Failed to move shortcut to group: {}", e);
        return HttpResponse::InternalServerError().body("Failed to move shortcut to group.");
    }

    HttpResponse::Ok().body("ok")
}
