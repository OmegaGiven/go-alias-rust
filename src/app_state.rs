use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

use crate::sql::DbConnection;

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

// NEW: Structure for P2P Room Signaling
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomState {
    pub id: String,
    pub host_offer: Option<String>,
    pub guest_answer: Option<String>,
    pub host_ice: Vec<String>,
    pub guest_ice: Vec<String>,
    // Key: Tool Name (e.g., "paint", "board"), Value: "rw" (Read/Write), "r" (Read Only), "none"
    pub permissions: HashMap<String, String>, 
}

impl RoomState {
    pub fn new(id: String) -> Self {
        let mut permissions = HashMap::new();
        // Default permissions - Host can toggle these in Manage Connection tab
        permissions.insert("paint".to_string(), "rw".to_string());
        permissions.insert("board".to_string(), "rw".to_string());
        permissions.insert("sql".to_string(), "none".to_string()); // Default sensitive tools to none
        
        Self {
            id,
            host_offer: None,
            guest_answer: None,
            host_ice: Vec::new(),
            guest_ice: Vec::new(),
            permissions,
        }
    }
}

pub struct AppState {
    pub shortcuts: Mutex<HashMap<String, String>>,
    pub hidden_shortcuts: Mutex<HashMap<String, String>>,
    pub work_shortcuts: Mutex<HashMap<String, String>>,
    pub notes: Mutex<Vec<Note>>,

    // THEME STATE
    pub current_theme: Mutex<Theme>, // The theme currently applied
    pub saved_themes: Mutex<HashMap<String, Theme>>, // All available themes

    // SQL service state
    pub connections: Mutex<Option<Vec<DbConnection>>>,
    pub last_results: Mutex<Vec<HashMap<String, String>>>,
    
    // NEW: P2P Signaling State
    pub rooms: Mutex<HashMap<String, RoomState>>,

    // SQL Connection Pooling
    pub sqlite_pools: Mutex<HashMap<String, sqlx::SqlitePool>>,
    pub pg_pools: Mutex<HashMap<String, sqlx::PgPool>>,
}