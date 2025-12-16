use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct DbConnection {
    pub host: String,
    pub db_name: String,
    pub user: String,
    pub password: String,
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct AddConnForm {
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