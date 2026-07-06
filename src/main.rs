mod app_state;
mod base_page;
mod elements;
mod pages;

use actix_files::Files;
use actix_web::{
    get,
    web::Data,
    App, HttpResponse, HttpServer, Responder,
};
use std::{
    collections::HashMap,
    path::Path,
    fs,
    sync::{Arc, Mutex},
};

use app_state::AppState;

use pages::not_found::{go, render_shortcuts_table};
use elements::theme::save_theme;
use elements::shortcut::{add_shortcut, delete_shortcut};
use base_page::{render_base_page, render_add_shortcut_button, render_add_shortcut_modal, nav_bar_html};

static SHORTCUTS_FILE: &str = "shortcuts.json";
static HIDDEN_SHORTCUTS_FILE: &str = "hidden-shortcuts.json";
static WORK_SHORTCUTS_FILE: &str = "work-shortcuts.json";

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

    let mut combined_shortcuts = shortcuts.clone();
    combined_shortcuts.extend(work_shortcuts.clone());

    let table_html = render_shortcuts_table(&combined_shortcuts);
    let saved_themes = state.saved_themes.lock().unwrap();

    let nav_with_button = nav_bar_html()
        .replace(r#"<div id="optional-button-placeholder"></div>"#, &render_add_shortcut_button());

    let content = format!(
        r#"
        <p>Type a shortcut key into the URL bar (e.g., <code>/gh</code>) to go directly to the destination.</p>
        {}
        "#,
        table_html
    );

    let html_output = render_base_page("Home - Shortcuts List", &content, &current_theme, &saved_themes);

    let final_html = html_output
        .replace(&nav_bar_html(), &nav_with_button)
        .replace("</body>", &format!("{}</body>", render_add_shortcut_modal()));

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(final_html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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

    let state = Arc::new(AppState {
        shortcuts: Mutex::new(shortcuts),
        hidden_shortcuts: Mutex::new(hidden_shortcuts),
        work_shortcuts: Mutex::new(work_shortcuts),
        current_theme: Mutex::new(current_theme),
        saved_themes: Mutex::new(saved_themes),
    });

    let _ = Path::new(".");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(index)
            .service(Files::new("/static", "./static").prefer_utf8(true))
            .service(add_shortcut)
            .service(delete_shortcut)
            .service(save_theme)
            .service(go)
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
