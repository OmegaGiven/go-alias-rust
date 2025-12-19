use actix_web::{get, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::app_state::Theme;
use crate::base_page::{render_base_page, render_add_shortcut_button, render_add_shortcut_modal, nav_bar_html};

/// Builds HTML table rows of shortcuts, grouped by URL, with inline delete buttons.
fn grouped_shortcuts_table_with_delete(shortcuts: &HashMap<String, String>) -> String {
    let mut grouped: HashMap<&str, Vec<&str>> = HashMap::new();
    for (key, url) in shortcuts.iter() {
        grouped.entry(url.as_str()).or_default().push(key.as_str());
    }

    let mut rows = String::new();
    let mut grouped_vec: Vec<_> = grouped.into_iter().collect();
    // Sort by URL first
    grouped_vec.sort_by_key(|(url, _)| url.to_owned());

    for (url, mut keys) in grouped_vec {
        keys.sort();
        let key_links = keys
            .iter()
            .map(|k| {
                let delete_form = format!(
                    r#"
                    <form action="/delete_shortcut" method="POST" style="display:inline; margin-left: 5px;" onsubmit="return confirm('Are you sure you want to delete shortcut: {}?');">
                        <input type="hidden" name="key" value="{}">
                        <button type="submit" class="delete-button" title="Delete {}" style="background: none; border: none; color: #ff6347; padding: 0; cursor: pointer; margin: 0; font-size: 10px; line-height: 1;">X</button>
                    </form>
                    "#,
                    encode_minimal(k),
                    encode_minimal(k),
                    encode_minimal(k)
                );
                
                // Group the link and the delete button, keeping them on one line
                format!("<span style='white-space: nowrap;'><a href=\"/{0}\">{0}</a>{1}</span>", encode_minimal(k), delete_form)
            })
            .collect::<Vec<_>>()
            .join(" , ");

        rows.push_str(&format!(
            "<tr><td class=\"keys\">{}</td><td class=\"url\">{}</td></tr>",
            key_links,
            encode_minimal(url)
        ));
    }
    rows
}

pub fn render_shortcuts_table(shortcuts: &HashMap<String, String>) -> String {
    let rows = grouped_shortcuts_table_with_delete(shortcuts); 
    format!(
        r#"
    <table class="grid">
      <thead>
        <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>
    "#,
        rows = rows
    )
}

pub fn not_found_page(
    shortcuts: &HashMap<String, String>, 
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>
) -> String {
    let table = render_shortcuts_table(shortcuts);
    
    let nav_with_button = nav_bar_html()
        .replace(r#"<div id="optional-button-placeholder"></div>"#, &render_add_shortcut_button());

    let content = format!(
        r#"
    <h1>404 – Shortcut Not Found</h1>
    <p>The requested shortcut was not found. Here are your available shortcuts:</p>
    {}
    "#,
        table
    );
    
    // Use the imported render_base_page and replace the navigation content
    render_base_page("Shortcut Not Found", &content, current_theme, saved_themes)
            // 1. Swap the navigation placeholder with the nav + button
            .replace(&nav_bar_html(), &nav_with_button)
            // 2. Append the modal just before </body>
            .replace("</body>", &format!("{}</body>", render_add_shortcut_modal()))
}

/// Catch‑all route for shortcuts
/// Updated to capture the full path (including slashes) using {tail:.*}
#[get("/{tail:.*}")]
// FIX: Made function public for external use (E0603)
pub async fn go(path: web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    // The path here captures everything after the domain, e.g. "youtube/omegagiven"
    let req_path = path.into_inner();
    
    // Lock mutexes to read
    let shortcuts = state.shortcuts.lock().unwrap();
    let hidden_shortcuts = state.hidden_shortcuts.lock().unwrap();
    let work_shortcuts = state.work_shortcuts.lock().unwrap(); 
    let current_theme = state.current_theme.lock().unwrap(); // Get current theme
    let saved_themes = state.saved_themes.lock().unwrap();

    // Helper to find a URL in any of the maps
    let find_url = |key: &str| -> Option<String> {
        shortcuts.get(key)
            .or_else(|| hidden_shortcuts.get(key))
            .or_else(|| work_shortcuts.get(key))
            .cloned()
    };

    // 1. Exact Match: Check if the full path is a defined shortcut
    if let Some(url) = find_url(&req_path) {
        return HttpResponse::Found()
            .append_header(("Location", url))
            .finish();
    }

    // 2. Smart Append: Check if the first segment is a shortcut (e.g. "youtube/omegagiven")
    // This splits "youtube/omegagiven" into "youtube" and "omegagiven"
    if let Some((alias, remainder)) = req_path.split_once('/') {
        if let Some(base_url) = find_url(alias) {
            // If the base URL ends with '/', just append. Otherwise add '/' then append.
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

    // 3. Not Found
    // Combine all *visible* shortcuts for display on the 404 page
    let mut combined_shortcuts = shortcuts.clone();
    combined_shortcuts.extend(work_shortcuts.clone());

    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(not_found_page(&combined_shortcuts, &current_theme, &saved_themes)) 
}