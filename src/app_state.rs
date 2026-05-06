use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

use crate::sql::DbConnection;

pub fn default_font_family() -> String {
    "sans-serif".to_string()
}

pub fn default_theme_mode() -> String {
    "custom".to_string()
}

pub fn default_accent_color() -> String {
    "#4da6ff".to_string()
}

pub fn default_element_margin() -> u32 {
    10
}

pub fn default_nav_height() -> u32 {
    30
}

// Define the structure for a theme, which consists of CSS color variables
#[derive(Serialize, Deserialize, Clone)]
pub struct Theme {
    pub name: String,
    #[serde(default = "default_theme_mode")]
    pub mode: String,
    pub primary_bg: String,    // e.g., #2e2e2e (Main page background)
    pub secondary_bg: String,  // e.g., #222 (Navigation/Modal background)
    pub tertiary_bg: String,   // e.g., #3a3a3a (Table header/List item background)
    pub text_color: String,    // e.g., #eee (Main text color)
    #[serde(default = "default_accent_color")]
    pub accent_color: String,  // e.g., #4da6ff (Primary UI accent color)
    pub link_color: String,    // e.g., #4da6ff (Default link color)
    pub link_visited: String,  // e.g., #b366ff (Visited link color)
    pub link_hover: String,    // e.g., #66ccff (Hover link color)
    pub border_color: String,  // e.g., #444 (Borders/Dividers)
    pub font_size_small: u32,
    pub font_size_medium: u32,
    pub font_size_large: u32,
    #[serde(default = "default_element_margin")]
    pub element_margin: u32,
    #[serde(default = "default_nav_height")]
    pub nav_height: u32,
    #[serde(default = "default_font_family")]
    pub font_family: String,
}

pub struct AppState {
    pub shortcuts: Mutex<HashMap<String, String>>,
    pub hidden_shortcuts: Mutex<HashMap<String, String>>,
    pub work_shortcuts: Mutex<HashMap<String, String>>,

    // THEME STATE
    pub current_theme: Mutex<Theme>, // The theme currently applied
    pub saved_themes: Mutex<HashMap<String, Theme>>, // All available themes

    // SQL service state
    pub connections: Mutex<Option<Vec<DbConnection>>>,
    pub last_results: Mutex<HashMap<String, Vec<HashMap<String, String>>>>,
    pub sql_jobs: Mutex<HashMap<String, SqlJob>>,

    // SQL Connection Pooling
    pub sqlite_pools: Mutex<HashMap<String, sqlx::SqlitePool>>,
    pub pg_pools: Mutex<HashMap<String, sqlx::PgPool>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SqlJob {
    pub id: String,
    pub connection: String,
    pub sql: String,
    pub query_name: String,
    pub query_folder: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub html: Option<String>,
    pub row_count_text: Option<String>,
    pub error: Option<String>,
    #[serde(skip)]
    pub results: Vec<HashMap<String, String>>,
}
