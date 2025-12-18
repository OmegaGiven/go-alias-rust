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
    use std::sync::OnceLock;
    static NAV_BAR: OnceLock<String> = OnceLock::new();
    NAV_BAR.get_or_init(|| {
        r#"
    <style>
        .modern-nav {
            display: flex;
            justify-content: space-between;
            align-items: center;
            background-color: var(--secondary-bg);
            border-bottom: 1px solid var(--border-color);
            padding: 0 20px;
            height: 30px; 
            min-height: 30px; /* Force fixed height */
            max-height: 30px; /* Prevent expansion */
            user-select: none;
            box-sizing: border-box;
            overflow: hidden; /* Ensure nothing pushes the box open */
        }
        
        .nav-left, .nav-right {
            display: flex;
            align-items: center;
            height: 100%;
            gap: 5px; /* Reduced gap slightly to fit more items */
        }
        
        /* High specificity selector to override global styles */
        .modern-nav .nav-link-item {
            display: inline-flex;
            align-items: center;
            justify-content: center;
            padding: 0 15px;
            height: 100%;
            color: var(--text-color);
            text-decoration: none !important;
            font-size: 1rem;
            opacity: 0.8;
            transition: background-color 0.2s ease, opacity 0.2s ease;
            border-bottom: 3px solid transparent;
            box-sizing: border-box;
            background: transparent;
            border-top: 3px solid transparent;
            cursor: pointer;
            white-space: nowrap;
            margin: 0 !important; /* Critical: Remove global button margins */
            line-height: normal;
        }
        
        /* Specific reset for the button version to match <a> tags exactly */
        .modern-nav button.nav-link-item {
            border: none;
            font-family: inherit;
            font-size: 1rem;
            appearance: none;
            background: transparent;
            outline: none;
        }

        .nav-link-item:hover {
            background-color: var(--tertiary-bg);
            opacity: 1;
            color: var(--text-color);
        }
        
        .nav-link-item.active {
            background-color: var(--tertiary-bg);
            opacity: 1;
            font-weight: bold;
            border-bottom-color: var(--link-color);
        }
        
        /* Connection indicator */
        .conn-link { position: relative; }
        .conn-link::after {
            content: '';
            position: absolute;
            top: 15px; /* Fixed position relative to 60px height */
            right: 5px;
            width: 8px;
            height: 8px;
            background-color: #49cc90;
            border-radius: 50%;
            opacity: 0.6;
            box-shadow: 0 0 4px rgba(73, 204, 144, 0.4);
        }
        
        #optional-button-placeholder {
            display: flex;
            align-items: center;
            height: 100%;
        }
    </style>

    <div class="modern-nav">
      <div class="nav-left">
        <a href="/" class="nav-link-item">Home</a>
        <a href="/sql" class="nav-link-item">SQL</a>
        <a href="/note" class="nav-link-item">Notes</a>
        <a href="/board" class="nav-link-item">Board</a>
        <a href="/paint" class="nav-link-item">Paint</a>
        <a href="/calculator" class="nav-link-item">Calculator</a>
        <a href="/requests" class="nav-link-item">Requests</a>
        <a href="/inspector" class="nav-link-item">Inspector</a>
        <a href="/connection" class="nav-link-item conn-link">Connection</a>
      </div>
      <div class="nav-right">
        <div id="optional-button-placeholder"></div>
        <a href="/settings" class="nav-link-item">Settings</a> 
      </div>
    </div>
    
    <script>
        document.addEventListener('DOMContentLoaded', () => {
            const path = window.location.pathname;
            const links = document.querySelectorAll('.nav-link-item');
            
            links.forEach(link => {
                const href = link.getAttribute('href');
                if (!href) return;
                
                // Active state logic: Exact match or sub-path match
                if (href === '/' && path === '/') {
                    link.classList.add('active');
                } else if (href !== '/' && path.startsWith(href)) {
                    link.classList.add('active');
                }
            });
        });
    </script>
    "#.to_string()
    }).clone()
}

pub fn render_base_page(title: &str, body_content: &str, current_theme: &Theme) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>{}</title>
    <!-- Favicon link pointing to the static folder -->
    <link rel="icon" type="image/x-icon" href="/static/favicon.ico">
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
    <button class="nav-link-item" id="addShortcutBtn">+ Add Shortcut</button>
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