// --- Updated Module Declarations ---
mod app_db;
mod app_state;
mod base_page;
mod elements;

// Grouping all page-related modules under the new pages module.
mod pages;

use actix_files::Files;
use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware::DefaultHeaders, web::Data,
};
use std::{
    collections::HashMap,
    fs,
    sync::{Arc, Mutex},
};

use app_state::AppState;

use pages::inspector::inspector_get;
use pages::not_found::{go, load_visible_shortcut_groups, render_home_shortcuts_content};
use pages::request::{
    request_cancel, request_create_folder, request_delete, request_delete_folder, request_get,
    request_history_get, request_history_save, request_import_postman, request_move,
    request_move_folder, request_rename, request_run, request_save, request_save_variables,
    scratchpads_get, scratchpads_save,
};

// Re-exporting SQL routes from the nested pages module
use pages::sql;

use base_page::render_base_page_with_options;
use elements::calculator::calculator_get;
use elements::shortcut::{
    add_shortcut, create_shortcut_group, delete_shortcut, move_shortcut_to_group,
};
use elements::theme::save_theme;

static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static WORK_SHORTCUTS_FILE: &str = "work-shortcuts.json";

// Only shortcut loading remains here
fn load_shortcuts(path: &str) -> std::io::Result<HashMap<String, String>> {
    let data = fs::read_to_string(path)?;
    let map: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(map)
}

async fn load_shortcuts_doc(key: &str, path: &str) -> HashMap<String, String> {
    app_db::migrate_json_file::<HashMap<String, String>>("shortcuts", key, path).await;
    app_db::get_json("shortcuts", key)
        .await
        .or_else(|| {
            load_shortcuts(path)
                .map_err(|e| {
                    eprintln!("Failed to load {path}: {e}");
                    e
                })
                .ok()
        })
        .unwrap_or_default()
}

#[get("/")]
async fn index(state: Data<Arc<AppState>>) -> impl Responder {
    let shortcuts = state
        .shortcuts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let work_shortcuts = state
        .work_shortcuts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let current_theme = state
        .current_theme
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    let saved_themes = state
        .saved_themes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();

    // Combine all *visible* shortcuts for display on the home page
    let mut combined_shortcuts = shortcuts;
    combined_shortcuts.extend(work_shortcuts);

    let (shortcut_groups, group_names) = load_visible_shortcut_groups().await;
    let full_page_content =
        render_home_shortcuts_content(&combined_shortcuts, &shortcut_groups, &group_names);
    let final_html = render_base_page_with_options(
        "Aliases",
        &full_page_content,
        &current_theme,
        &saved_themes,
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

    if let Err(err) = app_db::init().await {
        eprintln!("Failed to initialize app database. Falling back where possible: {err}");
    }
    app_db::migrate_json_file::<serde_json::Value>("sql", "queries", "saved_queries.json").await;
    app_db::migrate_json_file::<serde_json::Value>(
        "sql",
        "query_folders",
        "saved_query_folders.json",
    )
    .await;
    app_db::migrate_json_file::<serde_json::Value>("requests", "saved", "saved_requests.json")
        .await;
    app_db::migrate_json_file::<serde_json::Value>(
        "requests",
        "folders",
        "saved_request_folders.json",
    )
    .await;
    app_db::migrate_json_file::<serde_json::Value>(
        "requests",
        "variables",
        "request_variables.json",
    )
    .await;

    // --- Shortcut Loading ---
    let shortcuts = load_shortcuts_doc("visible", SHORTCUTS_FILE).await;
    let hidden_shortcuts = load_shortcuts_doc("hidden", HIDDEN_SHORTCUTS_FILE).await;
    let work_shortcuts = load_shortcuts_doc("work", WORK_SHORTCUTS_FILE).await;

    // --- Theme Loading ---
    let saved_themes = elements::theme::load_themes("themes.json").unwrap_or_else(|e| {
        eprintln!("Failed to load themes.json: {e}. Creating default map.");
        let mut map = HashMap::new();
        let default = elements::theme::default_dark_theme();
        map.insert(default.name.clone(), default);
        map
    });

    let current_theme =
        elements::theme::load_current_theme("current_theme.json").unwrap_or_else(|e| {
            eprintln!("Failed to load current_theme.json: {e}. Using default theme.");
            saved_themes
                .get("Dark Default")
                .cloned()
                .unwrap_or_else(elements::theme::default_dark_theme)
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
        last_results: Mutex::new(HashMap::new()),
        sql_jobs: Mutex::new(HashMap::new()),

        // SQL Connection Pools
        sqlite_pools: Mutex::new(HashMap::new()),
        pg_pools: Mutex::new(HashMap::new()),
    });

    // Build server
    HttpServer::new(move || {
        App::new()
            .wrap(DefaultHeaders::new().add(("Cache-Control", "no-store")))
            .app_data(Data::new(state.clone()))
            .service(index)
            // Register Request Builder handlers
            .service(request_get)
            .service(request_save)
            .service(request_delete)
            .service(request_rename)
            .service(request_create_folder)
            .service(request_delete_folder)
            .service(request_move)
            .service(request_move_folder)
            .service(request_save_variables)
            .service(request_history_get)
            .service(request_history_save)
            .service(request_import_postman)
            .service(request_run)
            .service(request_cancel)
            .service(scratchpads_get)
            .service(scratchpads_save)
            .service(inspector_get)
            .service(calculator_get)
            .service(sql::sql_get)
            .service(sql::sql_add)
            .service(sql::sql_run)
            .service(sql::sql_table_data)
            .service(sql::sql_table_update)
            .service(sql::sql_run_background)
            .service(sql::sql_run_history_get)
            .service(sql::sql_run_history_save)
            .service(sql::sql_run_history_delete)
            .service(sql::sql_run_history_clear)
            .service(sql::sql_jobs)
            .service(sql::sql_job)
            .service(sql::sql_job_activate)
            .service(sql::sql_export)
            .service(sql::sql_export_queries)
            .service(sql::sql_import_queries)
            .service(sql::sql_view)
            .service(sql::sql_save)
            .service(sql::sql_delete)
            .service(sql::sql_rename)
            .service(sql::sql_create_folder)
            .service(sql::sql_delete_folder)
            .service(sql::sql_move_query)
            .service(sql::sql_move_folder)
            .service(sql::sql_disconnect)
            .service(sql::sql_disconnect_connection)
            .service(sql::sql_delete_connection)
            .service(sql::sql_update_connection)
            .service(sql::sql_schema_json)
            .service(sql::sql_functions_json)
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .service(add_shortcut)
            .service(delete_shortcut)
            .service(create_shortcut_group)
            .service(move_shortcut_to_group)
            .service(save_theme)
            .service(go)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
