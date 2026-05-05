use actix_web::{get, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::app_state::Theme;
use crate::base_page::{render_base_page_with_options, render_inline_add_shortcut_button};

fn grouped_shortcuts_table(shortcuts: &HashMap<String, String>, empty_message: &str) -> String {
    if shortcuts.is_empty() {
        return format!(r#"<p class="shortcut-empty">{}</p>"#, encode_minimal(empty_message));
    }

    let mut grouped: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, url) in shortcuts {
        grouped.entry(url.as_str()).or_default().push(key.as_str());
    }

    let mut grouped_vec: Vec<_> = grouped.into_iter().collect();
    grouped_vec.sort_by_key(|(url, _)| url.to_owned());

    let mut rows = String::new();
    for (url, mut keys) in grouped_vec {
        keys.sort();
        let key_links = keys
            .iter()
            .map(|key| {
                let key_escaped = encode_minimal(key);
                format!(
                    r#"<span class="shortcut-key-chip"><a href="/{key}">{key}</a></span>"#,
                    key = key_escaped
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        rows.push_str(&format!(
            r#"<tr><td class="keys">{}</td><td class="url">{}</td></tr>"#,
            key_links,
            encode_minimal(url)
        ));
    }

    format!(
        r#"
        <table class="grid shortcut-grid">
          <thead>
            <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
          </thead>
          <tbody>
            {}
          </tbody>
        </table>
        "#,
        rows
    )
}

pub fn render_shortcuts_table(global_shortcuts: &HashMap<String, String>) -> String {
    let global_table = grouped_shortcuts_table(global_shortcuts, "No global shortcuts configured.");

    format!(
        r#"
        <div class="shortcut-sections">
            <section class="shortcut-section">
                <h2>Global Shortcuts</h2>
                <p>Saved on the server. These are shared with everyone using this instance.</p>
                {global_table}
            </section>

            <section class="shortcut-section">
                <h2>Local Shortcuts</h2>
                <p>Saved in browser Cookies. These can be removed from this browser without affecting the server.</p>
                <div id="local-shortcuts-table"></div>
            </section>
        </div>        "#,
        global_table = global_table
    )
}

pub fn render_home_shortcuts_content(global_shortcuts: &HashMap<String, String>) -> String {
    format!(
        r#"
        <link rel="stylesheet" href="/static/shortcuts.css">
        <section class="shortcut-home-intro">
            <p>Type a shortcut key into the URL bar (e.g., <code>/gh</code>) to go directly to the destination.</p>
            {}
        </section>
        {}
        <script src="/static/shortcuts.js" defer></script>
        "#,
        render_inline_add_shortcut_button(),
        render_shortcuts_table(global_shortcuts)
    )
}

pub fn not_found_page(
    global_shortcuts: &HashMap<String, String>,
    requested_path: &str,
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    let requested_path_display = encode_minimal(requested_path);
    let home_content = render_home_shortcuts_content(global_shortcuts);

    let content = format!(
        r#"
        <div id="shortcut-not-found-popup" class="shortcut-not-found-popup" role="status" data-requested-path="{}">
            <button type="button" class="shortcut-not-found-popup-close" aria-label="Dismiss shortcut not found message">&times;</button>
            <div class="shortcut-not-found-popup-title">Shortcut not found</div>
            <p class="shortcut-not-found-popup-message">No shortcut exists for <code>/{}</code>.</p>
        </div>

        {}
        "#,
        requested_path_display,
        requested_path_display,
        home_content
    );

    render_base_page_with_options(
        "Aliases",
        &content,
        current_theme,
        saved_themes,
        true,
    )
}

#[get("/{tail:.*}")]
pub async fn go(path: web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let req_path = path.into_inner();

    let shortcuts = state.shortcuts.lock().unwrap();
    let hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
    let work_shortcuts = state.work_shortcuts.lock().unwrap();
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    let find_url = |key: &str| -> Option<String> {
        shortcuts.get(key)
            .or_else(|| hidden_shortcuts.get(key))
            .or_else(|| work_shortcuts.get(key))
            .cloned()
    };

    if let Some(url) = find_url(&req_path) {
        return HttpResponse::Found()
            .append_header(("Location", url))
            .finish();
    }

    if let Some((alias, remainder)) = req_path.split_once('/') {
        if let Some(base_url) = find_url(alias) {
            let new_url = if base_url.ends_with('/') {
                format!("{}{}", base_url, remainder)
            } else {
                format!("{}/{}", base_url, remainder)
            };

            return HttpResponse::Found()
                .append_header(("Location", new_url))
                .finish();
        }
    }

    let mut combined_shortcuts = shortcuts.clone();
    combined_shortcuts.extend(work_shortcuts.clone());

    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(not_found_page(
            &combined_shortcuts,
            &req_path,
            &current_theme,
            &saved_themes,
        ))
}
