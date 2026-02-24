use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::base_page::render_base_page;

#[get("/connection")]
pub async fn connection_page(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    let body = r#"
    <div class="container connection-container">
        <h1>Connection Tool Moved</h1>
        <p class="subtitle">Use <b>Tools -> Connection</b> in the top bar to open the floating connection manager from any page.</p>
    </div>

    <style>
        .connection-container { max-width: 900px; margin: 40px auto; padding: 20px; }
        .connection-card { 
            background: var(--secondary-bg); 
            border: 1px solid var(--border-color); 
            padding: 25px; 
            border-radius: 12px;
            display: flex;
            flex-direction: column;
            gap: 15px;
        }
    </style>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_base_page("Manage Connections", body, &current_theme, &saved_themes))
}
