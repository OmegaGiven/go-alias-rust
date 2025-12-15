use crate::app_state::Theme; 
use std::collections::HashMap; 

fn render_theme_variables(theme: &Theme) -> String {
    format!(
        r#"
<style id="current-theme-vars">
:root {{
    --primary-bg: {};
    --secondary-bg: {};
    --tertiary-bg: {};
    --text-color: {};
    --link-color: {};
    --link-visited: {};
    --link-hover: {};
    --border-color: {};
}}
</style>
"#,
        theme.primary_bg, theme.secondary_bg, theme.tertiary_bg,
        theme.text_color, theme.link_color, theme.link_visited,
        theme.link_hover, theme.border_color,
    )
}

pub fn nav_bar_html() -> String {
    r#"
    <div class="tools">
      <div class="tool-buttons">
        <a href="/"><button class="nav-button">Home (Shortcuts)</button></a>
        <a href="/sql"><button class="nav-button">SQL Manager</button></a>
        <a href="/note"><button class="nav-button">Notes</button></a>
        <a href="/board"><button class="nav-button">Task Board</button></a>
        <a href="/calculator"><button class="nav-button">Calculator</button></a>
        <a href="/paint"><button class="nav-button">Paint</button></a>
        <a href="/requests"><button class="nav-button">Requests</button></a>
        <a href="/formatter"><button class="nav-button">Formatter</button></a> <!-- NEW BUTTON -->
      </div>
      <div class="right-buttons">
        <div id="optional-button-placeholder"></div>
        <a href="/settings"><button class="nav-button">Settings</button></a> 
      </div>
    </div>
    "#.to_string()
}

pub fn render_base_page(title: &str, body_content: &str, current_theme: &Theme) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>{}</title>
    {} 
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    {}
    {}
  </body>
</html>"#,
        title,
        render_theme_variables(current_theme),
        nav_bar_html(),
        body_content
    )
}

pub fn render_add_shortcut_button() -> String {
    r#"
    <div class="add-shortcut-container">
        <button class="nav-button add-button" id="addShortcutBtn">+ Add Shortcut</button>
    </div>
    "#.to_string()
}

pub fn render_add_shortcut_modal() -> String {
    let modal_html = r#"
<dialog id="addShortcutModal">
  <div class="modal-content">
    <span class="close-btn" id="closeModalBtn">&times;</span>
    <h2>Add New Shortcut</h2>
    <form action="/add_shortcut" method="POST" class="modal-form">
      <label for="shortcut">Shortcut:</label>
      <input type="text" id="shortcut" name="shortcut" placeholder="e.g., gh" required>

      <label for="url">URL:</label>
      <input type="url" id="url" name="url" placeholder="e.g., https://github.com" required>

      <div style="margin-top: 15px;">
        <input type="checkbox" id="hidden" name="hidden" value="true">
        <label for="hidden" style="display: inline; font-weight: normal;">Hidden?</label>
      </div>

      <div class="form-actions">
        <button type="submit" class="form-submit-btn">Save Shortcut</button>
      </div>
    </form>
  </div>
</dialog>
"#;

    let modal_js = r#"
<script>
  document.addEventListener('DOMContentLoaded', (event) => {{
    var modal = document.getElementById("addShortcutModal");
    var btn = document.getElementById("addShortcutBtn");
    var span = document.getElementById("closeModalBtn");

    if (btn && modal) {{
      btn.onclick = function() {{
        modal.showModal(); 
      }}
    }}

    if (span && modal) {{
      span.onclick = function() {{
        modal.close();
      }}
    }}

    if (modal) {{
        modal.addEventListener('click', (e) => {{
            if (e.target.nodeName === 'DIALOG') {{
                const rect = e.target.getBoundingClientRect();
                if (e.clientY < rect.top || e.clientY > rect.bottom ||
                    e.clientX < rect.left || e.clientX > rect.right) {{
                    modal.close();
                }}
            }}
        }});
    }}
  }});
</script>
"#;

    format!("{}{}", modal_html, modal_js)
}


pub fn render_settings_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let theme_options: String = saved_themes.keys()
        .map(|name| {
            let selected = if name == &current_theme.name { "selected" } else { "" };
            format!("<option value=\"{0}\" {1}>{0}</option>", name, selected)
        })
        .collect();

    format!(
        r#"
    <h1>Theme Settings</h1>
    <p>Customize the look and feel of your alias service.</p>

    <form action="/save_theme" method="POST" class="settings-form">
        <h2>Active Theme: {current_theme_name}</h2>
        <input type="hidden" id="original_name" name="original_name" value="{current_theme_name}">

        <div class="settings-grid">
            <!-- Theme Name and Selector -->
            <div>
                <label for="theme_name">Theme Name:</label>
                <input type="text" id="theme_name" name="theme_name" value="{current_theme_name}" required>
            </div>
            <div>
                <label for="load_theme">Load Saved Theme:</label>
                <select id="load_theme" onchange="document.getElementById('theme_name_input').value = this.value; document.querySelector('.settings-form').submit();">
                    <option value="" disabled selected>--- Select to Load ---</option>
                    {theme_options}
                </select>
                <input type="hidden" id="theme_name_input" name="load_theme_name" value="">
            </div>

            <!-- Color Pickers -->
            <div>
                <label for="primary_bg">Primary Background:</label>
                <input type="color" id="primary_bg" name="primary_bg" value="{primary_bg}">
            </div>
            <div>
                <label for="secondary_bg">Secondary Background:</label>
                <input type="color" id="secondary_bg" name="secondary_bg" value="{secondary_bg}">
            </div>
            <div>
                <label for="text_color">Text Color:</label>
                <input type="color" id="text_color" name="text_color" value="{text_color}">
            </div>
            <div>
                <label for="link_color">Link Color:</label>
                <input type="color" id="link_color" name="link_color" value="{link_color}">
            </div>
            <div>
                <label for="border_color">Border/Separator:</label>
                <input type="color" id="border_color" name="border_color" value="{border_color}">
            </div>
            <div>
                <label for="tertiary_bg">Tertiary/Row Background:</label>
                <input type="color" id="tertiary_bg" name="tertiary_bg" value="{tertiary_bg}">
            </div>
            <div>
                <label for="link_visited">Visited Link Color:</label>
                <input type="color" id="link_visited" name="link_visited" value="{link_visited}">
            </div>
            <div>
                <label for="link_hover">Link Hover Color:</label>
                <input type="color" id="link_hover" name="link_hover" value="{link_hover}">
            </div>
        </div>

        <div class="theme-action-buttons">
            <button type="button" id="applyChangesBtn">Apply Changes (Preview)</button>
            <button type="submit" name="action" value="save" class="form-submit-btn">Save / Update Theme</button>
            <button type="submit" name="action" value="apply_only" class="form-submit-btn">Apply Theme Only</button>
        </div>
    </form>
    
    <script>
        document.addEventListener('DOMContentLoaded', () => {{
            const form = document.querySelector('.settings-form');
            const applyBtn = document.getElementById('applyChangesBtn');
            const styleElement = document.getElementById('current-theme-vars');
            const themeInputs = form.querySelectorAll('input[type="color"]');

            const applyTheme = () => {{
                let cssVars = ':root {{';
                themeInputs.forEach(input => {{
                    cssVars += `--${{input.id}}: ${{input.value}};`; 
                }});
                cssVars += '}}';
                styleElement.innerHTML = cssVars;
            }};

            themeInputs.forEach(input => {{
                input.addEventListener('input', applyTheme);
            }});
            applyBtn.addEventListener('click', (e) => {{
                e.preventDefault(); 
                applyTheme();
            }});
        }});
    </script>
"#,
        current_theme_name = current_theme.name,
        primary_bg = current_theme.primary_bg,
        secondary_bg = current_theme.secondary_bg,
        tertiary_bg = current_theme.tertiary_bg,
        text_color = current_theme.text_color,
        link_color = current_theme.link_color,
        link_visited = current_theme.link_visited,
        link_hover = current_theme.link_hover,
        border_color = current_theme.border_color,
        theme_options = theme_options
    )
}