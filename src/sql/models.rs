use serde::{Deserialize, Serialize};

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
}