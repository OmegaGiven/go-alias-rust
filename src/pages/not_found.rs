use actix_web::{
    HttpResponse, Responder, get,
    web::{self, Data},
};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_db;
use crate::app_state::AppState;
use crate::app_state::Theme;
use crate::base_page::{
    render_base_page_with_options, render_inline_add_shortcut_button, static_asset,
};

fn normalize_group_name(group: Option<&String>) -> String {
    group
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Ungrouped".to_string())
}

fn shortcut_group_board(
    shortcuts: &HashMap<String, String>,
    shortcut_groups: &HashMap<String, String>,
    group_names: &[String],
    empty_message: &str,
) -> String {
    if shortcuts.is_empty() {
        return format!(
            r#"<p class="shortcut-empty">{}</p>"#,
            encode_minimal(empty_message)
        );
    }

    let mut groups: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
    for (key, url) in shortcuts {
        let group = normalize_group_name(shortcut_groups.get(key));
        groups
            .entry(group)
            .or_default()
            .push((key.as_str(), url.as_str()));
    }

    for group_name in group_names {
        let group_name = group_name.trim();
        if !group_name.is_empty() {
            groups.entry(group_name.to_string()).or_default();
        }
    }
    groups.entry("Ungrouped".to_string()).or_default();

    let mut group_order = groups.keys().cloned().collect::<Vec<_>>();
    group_order.sort_by(|a, b| {
        if a == "Ungrouped" {
            std::cmp::Ordering::Less
        } else if b == "Ungrouped" {
            std::cmp::Ordering::Greater
        } else {
            a.to_lowercase().cmp(&b.to_lowercase())
        }
    });

    let mut sections = String::new();
    for group in group_order {
        let mut entries = groups.remove(&group).unwrap_or_default();
        entries.sort_by_key(|(key, _)| key.to_lowercase());
        let group_attr = if group == "Ungrouped" {
            String::new()
        } else {
            htmlescape::encode_attribute(&group)
        };
        let group_title = encode_minimal(&group);

        let rows = if entries.is_empty() {
            r#"<tr><td colspan="3" class="shortcut-empty">Drop aliases here.</td></tr>"#.to_string()
        } else {
            entries
                .iter()
                .map(|(key, url)| {
                    let key_display = encode_minimal(key);
                    let key_attr = htmlescape::encode_attribute(key);
                    let url_display = encode_minimal(url);
                    let url_attr = htmlescape::encode_attribute(url);
                    let group_display = if group == "Ungrouped" {
                        String::new()
                    } else {
                        encode_minimal(&group)
                    };
                    format!(
                        r#"
                        <tr class="shortcut-alias-row" draggable="true" data-shortcut-key="{key_attr}" data-shortcut-scope="visible">
                          <td class="keys"><a href="/{key_attr}">{key_display}</a></td>
                          <td class="url"><a href="{url_attr}" title="{url_attr}">{url_display}</a></td>
                          <td class="shortcut-group-name">{group_display}</td>
                        </tr>
                        "#
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        };

        sections.push_str(&format!(
            r#"
            <section class="shortcut-group-card" data-shortcut-scope="visible" data-shortcut-group="{group_attr}">
              <h3>{group_title}</h3>
              <table class="grid shortcut-grid">
                <thead>
                  <tr><th>Alias</th><th>Destination URL</th><th>Group</th></tr>
                </thead>
                <tbody>{rows}</tbody>
              </table>
            </section>
            "#
        ));
    }

    format!(r#"<div class="shortcut-group-board">{sections}</div>"#)
}

pub fn render_shortcuts_table(
    global_shortcuts: &HashMap<String, String>,
    shortcut_groups: &HashMap<String, String>,
    group_names: &[String],
) -> String {
    let global_table = shortcut_group_board(
        global_shortcuts,
        shortcut_groups,
        group_names,
        "No global shortcuts configured.",
    );

    format!(
        r#"
        <div class="shortcut-sections">
            <section class="shortcut-section">
                <div class="shortcut-section-heading">
                    <div>
                        <h2>Global Shortcuts</h2>
                        <p>Saved on the server. These are shared with everyone using this instance.</p>
                    </div>
                    <form method="POST" action="/shortcut_group/create" class="shortcut-create-group-form">
                        <input type="hidden" name="scope" value="visible">
                        <input type="text" name="group_name" placeholder="New group name" required>
                        <button type="submit" class="form-submit-btn">Create Group</button>
                    </form>
                </div>
                {global_table}
            </section>

            <section class="shortcut-section">
                <div class="shortcut-section-heading">
                    <div>
                        <h2>Local Shortcuts</h2>
                        <p>Saved in browser Cookies. These can be removed from this browser without affecting the server.</p>
                    </div>
                    <form class="shortcut-create-group-form" data-local-shortcut-group-form>
                        <input type="text" name="group_name" placeholder="New group name" required>
                        <button type="submit" class="form-submit-btn">Create Group</button>
                    </form>
                </div>
                <div id="local-shortcuts-table"></div>
            </section>
        </div>        "#,
        global_table = global_table
    )
}

pub fn render_home_shortcuts_content(
    global_shortcuts: &HashMap<String, String>,
    shortcut_groups: &HashMap<String, String>,
    group_names: &[String],
) -> String {
    format!(
        r#"
        <link rel="stylesheet" href="{}">
        <section class="shortcut-home-intro">
            <p>Type a shortcut key into the URL bar (e.g., <code>/gh</code>) to go directly to the destination.</p>
            {}
        </section>
        {}
        <script src="{}" defer></script>
        "#,
        static_asset("shortcuts.css"),
        render_inline_add_shortcut_button(),
        render_shortcuts_table(global_shortcuts, shortcut_groups, group_names),
        static_asset("shortcuts.js")
    )
}

pub fn not_found_page(
    global_shortcuts: &HashMap<String, String>,
    shortcut_groups: &HashMap<String, String>,
    group_names: &[String],
    requested_path: &str,
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    let requested_path_display = encode_minimal(requested_path);
    let home_content =
        render_home_shortcuts_content(global_shortcuts, shortcut_groups, group_names);

    let content = format!(
        r#"
        <div id="shortcut-not-found-popup" class="shortcut-not-found-popup" role="status" data-requested-path="{}">
            <button type="button" class="shortcut-not-found-popup-close" aria-label="Dismiss shortcut not found message">&times;</button>
            <div class="shortcut-not-found-popup-title">Shortcut not found</div>
            <p class="shortcut-not-found-popup-message">No shortcut exists for <code>/{}</code>.</p>
        </div>

        {}
        "#,
        requested_path_display, requested_path_display, home_content
    );

    render_base_page_with_options("Aliases", &content, current_theme, saved_themes, true)
}

pub async fn load_visible_shortcut_groups() -> (HashMap<String, String>, Vec<String>) {
    let mut shortcut_groups = app_db::get_shortcut_group_map("visible").await;
    shortcut_groups.extend(app_db::get_shortcut_group_map("work").await);

    let mut group_names = app_db::get_shortcut_groups("visible").await;
    group_names.extend(app_db::get_shortcut_groups("work").await);
    group_names.sort_by_key(|name| name.to_lowercase());
    group_names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    (shortcut_groups, group_names)
}

#[get("/{tail:.*}")]
pub async fn go(path: web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let req_path = path.into_inner();

    let shortcuts = state.shortcuts.lock().unwrap().clone();
    let hidden_shortcuts = state.hidden_shortcuts.lock().unwrap().clone();
    let work_shortcuts = state.work_shortcuts.lock().unwrap().clone();
    let current_theme = state.current_theme.lock().unwrap().clone();
    let saved_themes = state.saved_themes.lock().unwrap().clone();

    let find_url = |key: &str| -> Option<String> {
        shortcuts
            .get(key)
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

    let mut combined_shortcuts = shortcuts;
    combined_shortcuts.extend(work_shortcuts);
    let (shortcut_groups, group_names) = load_visible_shortcut_groups().await;

    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(not_found_page(
            &combined_shortcuts,
            &shortcut_groups,
            &group_names,
            &req_path,
            &current_theme,
            &saved_themes,
        ))
}
