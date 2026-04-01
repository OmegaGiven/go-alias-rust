use actix_web::{get, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::app_state::Theme;
use crate::base_page::{render_add_shortcut_button, render_add_shortcut_modal, render_base_page, nav_bar_html};

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
        <style>
            .shortcut-sections {{
                display: grid;
                gap: 18px;
                margin-top: 16px;
            }}

            .shortcut-section {{
                background: var(--secondary-bg);
                border: 1px solid var(--border-color);
                border-radius: 8px;
                padding: 16px;
            }}

            .shortcut-section h2 {{
                margin-top: 0;
                margin-bottom: 8px;
            }}

            .shortcut-section p {{
                margin-top: 0;
            }}

            .shortcut-grid {{
                margin-bottom: 0;
            }}

            .shortcut-key-chip {{
                display: inline-flex;
                align-items: center;
                gap: 6px;
                white-space: nowrap;
                margin-right: 6px;
                vertical-align: middle;
            }}

            .shortcut-empty {{
                opacity: 0.8;
                margin-bottom: 0;
            }}

            .local-shortcut-delete {{
                display: inline-flex;
                align-items: center;
                justify-content: center;
                background: transparent;
                border: 1px solid var(--link-hover);
                color: var(--link-hover);
                border-radius: 4px;
                cursor: pointer;
                padding: 0 6px;
                font-size: var(--font-size-small);
                line-height: 1.2;
                height: 22px;
                margin: 0;
                vertical-align: middle;
                box-sizing: border-box;
            }}

            .local-shortcut-delete:hover {{
                background: var(--link-hover);
                color: var(--primary-bg);
            }}
        </style>

        <div class="shortcut-sections">
            <section class="shortcut-section">
                <h2>Global Shortcuts</h2>
                <p>Saved on the server. These are shared with everyone using this instance.</p>
                {global_table}
            </section>

            <section class="shortcut-section">
                <h2>Local Shortcuts</h2>
                <p>Saved only in this browser. These can be removed from this browser without affecting the server.</p>
                <div id="local-shortcuts-table"></div>
            </section>
        </div>

        <script>
            (() => {{
                const LOCAL_SHORTCUTS_KEY = 'go_service_local_shortcuts';
                const LOCAL_HIDDEN_SHORTCUTS_KEY = 'go_service_local_hidden_shortcuts';

                function readShortcutBucket(key) {{
                    try {{
                        return JSON.parse(localStorage.getItem(key) || '{{}}');
                    }} catch (_) {{
                        return {{}};
                    }}
                }}

                function writeShortcutBucket(key, value) {{
                    localStorage.setItem(key, JSON.stringify(value));
                }}

                function buildShortcutPath(key) {{
                    return '/' + encodeURIComponent(key).replace(/%2F/g, '/');
                }}

                function groupByUrl(shortcuts) {{
                    const grouped = new Map();
                    Object.entries(shortcuts)
                        .sort((a, b) => a[0].localeCompare(b[0]))
                        .forEach(([key, url]) => {{
                            if (!grouped.has(url)) {{
                                grouped.set(url, []);
                            }}
                            grouped.get(url).push(key);
                        }});
                    return Array.from(grouped.entries()).sort((a, b) => a[0].localeCompare(b[0]));
                }}

                function escapeHtml(value) {{
                    return value
                        .replaceAll('&', '&amp;')
                        .replaceAll('<', '&lt;')
                        .replaceAll('>', '&gt;')
                        .replaceAll('"', '&quot;')
                        .replaceAll("'", '&#39;');
                }}

                function removeLocalShortcut(storageKey, shortcutKey) {{
                    const bucket = readShortcutBucket(storageKey);
                    delete bucket[shortcutKey];
                    writeShortcutBucket(storageKey, bucket);
                    renderLocalShortcutSections();
                }}

                function renderLocalShortcutTable(containerId, storageKey, emptyMessage) {{
                    const container = document.getElementById(containerId);
                    if (!container) return;

                    const shortcuts = readShortcutBucket(storageKey);
                    const grouped = groupByUrl(shortcuts);

                    if (grouped.length === 0) {{
                        container.innerHTML = `<p class="shortcut-empty">${{escapeHtml(emptyMessage)}}</p>`;
                        return;
                    }}

                    const rows = grouped.map(([url, keys]) => {{
                        const keyLinks = keys.map((key) => {{
                            const escapedKey = escapeHtml(key);
                            const escapedUrl = escapeHtml(url);
                            const storageAttr = escapeHtml(storageKey);
                            return `
                                <span class="shortcut-key-chip">
                                    <a href="${{escapedUrl}}" title="Open ${{escapedKey}}">${{escapedKey}}</a>
                                    <button
                                        type="button"
                                        class="local-shortcut-delete"
                                        data-storage-key="${{storageAttr}}"
                                        data-shortcut-key="${{escapedKey}}"
                                        title="Delete local shortcut ${{escapedKey}}"
                                    >Delete</button>
                                </span>
                            `;
                        }}).join(' ');

                        return `<tr><td class="keys">${{keyLinks}}</td><td class="url">${{escapeHtml(url)}}</td></tr>`;
                    }}).join('');

                    container.innerHTML = `
                        <table class="grid shortcut-grid">
                            <thead>
                                <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
                            </thead>
                            <tbody>${{rows}}</tbody>
                        </table>
                    `;
                }}

                function renderLocalShortcutSections() {{
                    renderLocalShortcutTable('local-shortcuts-table', LOCAL_SHORTCUTS_KEY, 'No local shortcuts saved in this browser.');

                    document.querySelectorAll('.local-shortcut-delete').forEach((button) => {{
                        button.onclick = () => removeLocalShortcut(button.dataset.storageKey, button.dataset.shortcutKey);
                    }});
                }}

                window.resolveLocalShortcutPath = function(reqPath) {{
                    const localShortcuts = readShortcutBucket(LOCAL_SHORTCUTS_KEY);
                    const hiddenLocalShortcuts = readShortcutBucket(LOCAL_HIDDEN_SHORTCUTS_KEY);
                    const direct = localShortcuts[reqPath] || hiddenLocalShortcuts[reqPath];
                    if (direct) {{
                        return direct;
                    }}

                    const slashIndex = reqPath.indexOf('/');
                    if (slashIndex === -1) {{
                        return null;
                    }}

                    const alias = reqPath.slice(0, slashIndex);
                    const remainder = reqPath.slice(slashIndex + 1);
                    const baseUrl = localShortcuts[alias] || hiddenLocalShortcuts[alias];
                    if (!baseUrl) {{
                        return null;
                    }}

                    return baseUrl.endsWith('/') ? `${{baseUrl}}${{remainder}}` : `${{baseUrl}}/${{remainder}}`;
                }};

                document.addEventListener('DOMContentLoaded', renderLocalShortcutSections);
            }})();
        </script>
        "#,
        global_table = global_table
    )
}

pub fn not_found_page(
    global_shortcuts: &HashMap<String, String>,
    requested_path: &str,
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    let table = render_shortcuts_table(global_shortcuts);
    let requested_path_json = serde_json::to_string(requested_path).unwrap_or_else(|_| "\"\"".to_string());

    let nav_with_button = nav_bar_html()
        .replace(r#"<div id="optional-button-placeholder"></div>"#, &render_add_shortcut_button());

    let content = format!(
        r#"
        <h1>404 - Shortcut Not Found</h1>
        <p>The requested shortcut was not found on the server. Checking this browser's local shortcuts first, then showing the available lists below.</p>
        {}
        <script>
            document.addEventListener('DOMContentLoaded', () => {{
                const requestedPath = {};
                if (!requestedPath || typeof window.resolveLocalShortcutPath !== 'function') {{
                    return;
                }}

                const localUrl = window.resolveLocalShortcutPath(requestedPath);
                if (localUrl) {{
                    window.location.replace(localUrl);
                }}
            }});
        </script>
        "#,
        table,
        requested_path_json
    );

    render_base_page("Shortcut Not Found", &content, current_theme, saved_themes)
        .replace(&nav_bar_html(), &nav_with_button)
        .replace("</body>", &format!("{}</body>", render_add_shortcut_modal()))
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
