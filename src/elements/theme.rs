use actix_web::{
    get, post,
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

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

// File constants
pub static THEMES_FILE: &str = "themes.json";
pub static CURRENT_THEME_FILE: &str = "current_theme.json";

// Helper to define a default dark theme
pub fn default_dark_theme() -> Theme {
    Theme {
        name: "Dark Default".to_string(),
        primary_bg: "#2e2e2e".to_string(),
        secondary_bg: "#222222".to_string(),
        tertiary_bg: "#3a3a3a".to_string(),
        text_color: "#eeeeee".to_string(),
        link_color: "#4da6ff".to_string(),
        link_visited: "#b366ff".to_string(),
        link_hover: "#66ccff".to_string(),
        border_color: "#444444".to_string(),
        font_size_small: 14,
        font_size_medium: 16,
        font_size_large: 18,
    }
}

// Load saved themes
pub fn load_themes(path: &str) -> io::Result<HashMap<String, Theme>> {
    let data = fs::read_to_string(path)?;
    let map: HashMap<String, Theme> = serde_json::from_str(&data)?;
    Ok(map)
}

// Save saved themes
fn save_themes(path: &str, themes: &HashMap<String, Theme>) -> io::Result<()> {
    let data = serde_json::to_string_pretty(themes)?;
    fs::write(path, data)
}

// Load current theme
pub fn load_current_theme(path: &str) -> io::Result<Theme> {
    let data = fs::read_to_string(path)?;
    let theme: Theme = serde_json::from_str(&data)?;
    Ok(theme)
}

// Save current theme
fn save_current_theme(path: &str, theme: &Theme) -> io::Result<()> {
    let data = serde_json::to_string_pretty(theme)?;
    fs::write(path, data)
}

// Struct to capture the theme form data
#[derive(Deserialize)]
pub struct ThemeForm {
    // FIX: Changed to Option to prevent "missing field" errors if the frontend
    // doesn't send this hidden input correctly.
    #[serde(rename = "original_name")]
    pub _original_name: Option<String>, 
    
    pub theme_name: String,
    pub primary_bg: String,
    pub secondary_bg: String,
    pub tertiary_bg: String,
    pub text_color: String,
    pub link_color: String,
    pub link_visited: String,
    pub link_hover: String,
    pub border_color: String,
    pub font_size_small: u32,
    pub font_size_medium: u32,
    pub font_size_large: u32,
    
    pub load_theme_name: Option<String>,
    
    // Action is optional because JS submissions might not include button values
    pub action: Option<String>,                  
}

// Handler for POST /save_theme
#[post("/save_theme")]
pub async fn save_theme(
    form: Form<ThemeForm>,
    state: Data<Arc<AppState>>,
) -> impl Responder {
    // 1. Handle loading a theme first (if requested via dropdown)
    if let Some(load_name) = form.load_theme_name.clone().filter(|n| !n.is_empty()) {
        let mut current_theme = state.current_theme.lock().unwrap();
        let saved_themes = state.saved_themes.lock().unwrap();

        if let Some(loaded_theme) = saved_themes.get(&load_name) {
            *current_theme = loaded_theme.clone();
            // Persist the newly loaded theme as the current theme
            if let Err(e) = save_current_theme(CURRENT_THEME_FILE, &current_theme) {
                eprintln!("Failed to save current theme after loading: {}", e);
            }
        }
        
        // Redirect back to settings page to show the loaded theme
        return HttpResponse::Found()
            .append_header(("Location", "/settings"))
            .finish();
    }


    // 2. Create the new theme from form data
    let new_theme = Theme {
        name: form.theme_name.clone(),
        primary_bg: form.primary_bg.clone(),
        secondary_bg: form.secondary_bg.clone(),
        tertiary_bg: form.tertiary_bg.clone(),
        text_color: form.text_color.clone(),
        link_color: form.link_color.clone(),
        link_visited: form.link_visited.clone(),
        link_hover: form.link_hover.clone(),
        border_color: form.border_color.clone(),
        font_size_small: form.font_size_small,
        font_size_medium: form.font_size_medium,
        font_size_large: form.font_size_large,
    };

    // 3. Update the current theme
    {
        let mut current_theme = state.current_theme.lock().unwrap();
        *current_theme = new_theme.clone();
        
        // Persist the current theme regardless of save action
        if let Err(e) = save_current_theme(CURRENT_THEME_FILE, &current_theme) {
            eprintln!("Failed to save current theme: {}", e);
            return HttpResponse::InternalServerError().body("Failed to save current theme state.");
        }
    }

    // 4. Handle saving to saved_themes if action is "save"
    if let Some(action) = &form.action {
        if action == "save" {
            let mut saved_themes = state.saved_themes.lock().unwrap();
            saved_themes.insert(new_theme.name.clone(), new_theme);

            // Persist all saved themes
            if let Err(e) = save_themes(THEMES_FILE, &saved_themes) {
                eprintln!("Failed to save themes list: {}", e);
                return HttpResponse::InternalServerError().body("Failed to save themes list.");
            }
        }
    }

    // Redirect back to settings page
    HttpResponse::Found()
        .append_header(("Location", "/settings"))
        .finish()
}