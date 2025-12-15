use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

use crate::sql::DbConnection;

// NEW STRUCT: Note
// This struct stores both the subject and the content of a saved note.
#[derive(Serialize, Deserialize, Clone)]
pub struct Note {
    pub subject: String,
    pub content: String,
}

// Define the structure for a theme, which consists of CSS color variables
#[derive(Serialize, Deserialize, Clone)]
pub struct Theme {
    pub name: String,
    pub primary_bg: String,    // e.g., #2e2e2e (Main page background)
    pub secondary_bg: String,  // e.g., #222 (Navigation/Modal background)
    pub tertiary_bg: String,   // e.g., #3a3a3a (Table header/List item background)
    pub text_color: String,    // e.g., #eee (Main text color)
    pub link_color: String,    // e.g., #4da6ff (Default link color)
    pub link_visited: String,  // e.g., #b366ff (Visited link color)
    pub link_hover: String,    // e.g., #66ccff (Hover link color)
    pub border_color: String,  // e.g., #444 (Borders/Dividers)
}

pub struct AppState {
    pub shortcuts: Mutex<HashMap<String, String>>,
    pub hidden_shortcuts: Mutex<HashMap<String, String>>,
    pub work_shortcuts: Mutex<HashMap<String, String>>,
    // UPDATED: Use Vec<Note> instead of Vec<String>
    pub notes: Mutex<Vec<Note>>,

    // THEME STATE
    pub current_theme: Mutex<Theme>, // The theme currently applied
    pub saved_themes: Mutex<HashMap<String, Theme>>, // All available themes

    // SQL service state
    pub connections: Mutex<Vec<DbConnection>>,
    pub last_results: Mutex<Vec<HashMap<String, String>>>,
}