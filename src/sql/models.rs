use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_db_type() -> String {
    "postgres".to_string()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DbConnection {
    #[serde(default = "default_db_type")]
    pub db_type: String, // "postgres" or "sqlite"
    pub host: String,    // For sqlite, this is the filename/path
    pub db_name: String, // Postgres only
    pub user: String,    // Postgres only
    pub password: String, // Postgres only
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct AddConnForm {
    pub db_type: Option<String>,
    pub host: String,
    pub db_name: String,
    pub user: String,
    pub password: String,
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct SqlForm {
    pub sql: String,
    pub connection: String,
    pub variables: Option<HashMap<String, String>>,
}