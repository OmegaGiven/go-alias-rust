use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;

use crate::app_state::AppState;
use crate::base_page::render_base_page;

#[get("/calculator")]
pub async fn calculator_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    let content = r#"
        <div style="padding: 50px; text-align: center; max-width: 600px; margin: calc(var(--element-margin) / 2) var(--element-margin);">
            <h1>Calculator</h1>
            <p style="font-size: 1.1em; opacity: 0.8; margin: calc(var(--element-margin) / 2) var(--element-margin);">
                The calculator is a floating app that stays with you.
                You can toggle it from anywhere using the <b>Calculator</b> item in the tools menu.
            </p>
            <button class="form-submit-btn" onclick="toggleCalculator()" style="width: auto; padding: 12px 30px; font-size: 1.1em; cursor: pointer;">
                Open Calculator
            </button>
            <p style="margin: calc(var(--element-margin) / 2) var(--element-margin); font-size: 0.9em; opacity: 0.6;">
                It remembers its position and visibility state when you switch pages or refresh.
            </p>
        </div>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_base_page("Calculator", content, &current_theme, &saved_themes))
}
