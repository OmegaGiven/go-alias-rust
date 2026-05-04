// --- Updated Module Declarations ---
mod app_state;
mod base_page;
mod elements;

// Grouping all page-related modules under the new pages module.
mod pages; 

use actix_files::Files;
use actix_web::{
    get, 
    web::{Data}, 
    App, HttpResponse, HttpServer, Responder,
};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};

use app_state::AppState;

use pages::request::{request_get, request_save, request_delete, request_run, request_cancel};
use pages::inspector::inspector_get; 
use pages::not_found::{go, render_home_shortcuts_content}; 

// Re-exporting SQL routes from the nested pages module
use pages::sql;

use elements::theme::{save_theme};
use elements::shortcut::{add_shortcut, delete_shortcut}; 
use elements::calculator::calculator_get;
use base_page::render_base_page_with_options;

static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static WORK_SHORTCUTS_FILE: &str = "work-shortcuts.json"; 

// Only shortcut loading remains here
fn load_shortcuts(path: &str) -> std::io::Result<HashMap<String, String>> {
    let data = fs::read_to_string(path)?;
    let map: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

#[get("/")]
async fn index(state: Data<Arc<AppState>>) -> impl Responder {
    let shortcuts = state.shortcuts.lock().unwrap();
    let work_shortcuts = state.work_shortcuts.lock().unwrap(); 
    let current_theme = state.current_theme.lock().unwrap(); 

    // Combine all *visible* shortcuts for display on the home page
    let mut combined_shortcuts = shortcuts.clone();
    combined_shortcuts.extend(work_shortcuts.clone());

    let saved_themes = state.saved_themes.lock().unwrap();
    
    let full_page_content = render_home_shortcuts_content(&combined_shortcuts);
    let final_html = render_base_page_with_options(
        "Aliases",
        &full_page_content,
        &current_theme,
        &saved_themes,
        true,
        true,
    );

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(final_html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(80);

    // --- Shortcut Loading ---
    let shortcuts = load_shortcuts(SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {SHORTCUTS_FILE}: {e}"); 
        HashMap::new()
    });

    let hidden_shortcuts = load_shortcuts(HIDDEN_SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {HIDDEN_SHORTCUTS_FILE}: {e}");
        HashMap::new()
    });

    let work_shortcuts = load_shortcuts(WORK_SHORTCUTS_FILE).unwrap_or_else(|e| {
        eprintln!("Failed to load {WORK_SHORTCUTS_FILE}: {e}");
        HashMap::new()
    });

    // --- Theme Loading ---
    let saved_themes = elements::theme::load_themes("themes.json").unwrap_or_else(|e| {
        eprintln!("Failed to load themes.json: {e}. Creating default map.");
        let mut map = HashMap::new();
        let default = elements::theme::default_dark_theme();
        map.insert(default.name.clone(), default);
        map
    });

    let current_theme = elements::theme::load_current_theme("current_theme.json").unwrap_or_else(|e| {
        eprintln!("Failed to load current_theme.json: {e}. Using default theme.");
        saved_themes.get("Dark Default").cloned().unwrap_or_else(elements::theme::default_dark_theme)
    });


    // Shared application state
    let state = Arc::new(AppState {
        shortcuts: Mutex::new(shortcuts),
        hidden_shortcuts: Mutex::new(hidden_shortcuts),
        work_shortcuts: Mutex::new(work_shortcuts),

        // THEME STATE
        current_theme: Mutex::new(current_theme),
        saved_themes: Mutex::new(saved_themes),

        // SQL service state
        connections: Mutex::new(None),
        last_results: Mutex::new(Vec::new()),

        // SQL Connection Pools
        sqlite_pools: Mutex::new(HashMap::new()),
        pg_pools: Mutex::new(HashMap::new()),
    });

    // Build server
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(index)
            // Register Request Builder handlers
            .service(request_get)
            .service(request_save)
            .service(request_delete)
            .service(request_run) 
            .service(request_cancel)
            .service(inspector_get)
            .service(calculator_get)
            .service(sql::sql_get)
            .service(sql::sql_add)
            .service(sql::sql_run)
            .service(sql::sql_export)
            .service(sql::sql_view)
            .service(sql::sql_save) 
            .service(sql::sql_delete) 
            .service(sql::sql_delete_connection)
            .service(sql::sql_schema_json) 
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .service(add_shortcut)      
            .service(delete_shortcut)       
            .service(save_theme)        
            .service(go) 
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
