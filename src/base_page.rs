use crate::app_state::Theme; 
use crate::elements::calculator;
use crate::elements::jwt_decoder;
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
    --font-size-small: {}px;
    --font-size-medium: {}px;
    --font-size-large: {}px;
    --base-font-size: {}px;
}}
</style>
"#,
        theme.primary_bg, theme.secondary_bg, theme.tertiary_bg,
        theme.text_color, theme.link_color, theme.link_visited,
        theme.link_hover, theme.border_color,
        theme.font_size_small, theme.font_size_medium, theme.font_size_large,
        theme.font_size_medium
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
            overflow: visible; /* Allow tool dropdown to float over page content */
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

        .tools-dropdown {
            position: relative;
            height: 100%;
            display: flex;
            align-items: stretch;
        }

        .tools-dropdown-menu {
            display: none;
            position: absolute;
            top: 100%;
            right: 0;
            min-width: 170px;
            background: var(--secondary-bg);
            border: 1px solid var(--border-color);
            border-radius: 0 0 6px 6px;
            box-shadow: 0 10px 24px rgba(0, 0, 0, 0.35);
            z-index: 10020;
            overflow: hidden;
        }

        .tools-dropdown:hover .tools-dropdown-menu {
            display: block;
        }

        .tools-dropdown-item {
            width: 100%;
            text-align: left;
            padding: 8px 12px;
            margin: 0 !important;
            border: none;
            border-bottom: 1px solid var(--border-color);
            background: transparent;
            color: var(--text-color);
            font-size: 0.9rem;
            cursor: pointer;
        }

        .tools-dropdown-item:last-child {
            border-bottom: none;
        }

        .tools-dropdown-item:hover {
            background: var(--tertiary-bg);
        }
    </style>

    <div class="modern-nav">
      <div class="nav-left">
        <a href="/" class="nav-link-item">Home</a>
        <a href="/sql" class="nav-link-item">SQL</a>
        <a href="/note" class="nav-link-item">Notes</a>
        <a href="/board" class="nav-link-item">Board</a>
        <a href="/paint" class="nav-link-item">Paint</a>
        <a href="/requests" class="nav-link-item">Requests</a>
        <a href="/inspector" class="nav-link-item">Inspector</a>
        <a href="/connection" class="nav-link-item conn-link">Connection</a>
      </div>
      <div class="nav-right">
        <div id="optional-button-placeholder"></div>
        <div class="tools-dropdown">
          <button class="nav-link-item">Tools</button>
          <div class="tools-dropdown-menu">
            <button class="tools-dropdown-item" onclick="toggleCalculator()">Calculator</button>
            <button class="tools-dropdown-item" onclick="toggleJwtDecoder()">JWT Decoder</button>
          </div>
        </div>
        <button class="nav-link-item" onclick="toggleSettings()">Settings</button> 
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

pub fn render_base_page(
    title: &str, 
    body_content: &str, 
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>{}</title>
    <!-- Favicon link pointing to the static folder -->
    <link rel="icon" type="image/x-icon" href="/static/favicon.ico">
    {} 
    {}
    {}
    {}
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    {}
    {}
    {}
    {}
    {}
    <script>{}</script>
    <script>{}</script>
    <script>{}</script>
  </body>
</html>"#,
        title,
        render_theme_variables(current_theme),
        calculator::get_css(),
        jwt_decoder::get_css(),
        get_settings_css(),
        nav_bar_html(),
        body_content,
        calculator::get_html(),
        jwt_decoder::get_html(),
        get_settings_html(current_theme, saved_themes),
        calculator::get_js(),
        jwt_decoder::get_js(),
        get_settings_js()
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


pub fn get_settings_css() -> String {
    r#"
    <style>
    #floating-settings {
        position: fixed;
        top: 60px;
        right: 20px;
        width: 450px;
        max-height: 80vh;
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow: 0 10px 30px rgba(0,0,0,0.5);
        z-index: 10001;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .settings-header {
        background-color: var(--tertiary-bg);
        padding: 10px 15px;
        cursor: move;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid var(--border-color);
        user-select: none;
    }

    .settings-header h3 {
        margin: 0;
        font-size: 1rem;
    }

    .settings-close-btn {
        background: none;
        border: none;
        color: var(--text-color);
        font-size: 1.5rem;
        cursor: pointer;
        opacity: 0.7;
    }

    .settings-close-btn:hover {
        opacity: 1;
    }

    .settings-content-wrapper {
        padding: 20px;
        overflow-y: auto;
    }

    .settings-form h2 {
        display: none; /* Hide redundant header in floating view */
    }

    .settings-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 15px;
    }

    .settings-grid div {
        display: flex;
        flex-direction: column;
        gap: 5px;
    }

    .settings-grid label {
        font-size: 0.9rem;
        font-weight: bold;
        opacity: 0.9;
    }

    .theme-action-buttons {
        margin-top: 25px;
        display: flex;
        gap: 10px;
    }
    </style>
    "#.to_string()
}

pub fn get_settings_html(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let theme_options: String = saved_themes.keys()
        .map(|name| {
            let selected = if name == &current_theme.name { "selected" } else { "" };
            format!("<option value=\"{0}\" {1}>{0}</option>", name, selected)
        })
        .collect();

    format!(
        r#"
    <div id="floating-settings" style="display: none;">
        <div class="settings-header" id="settings-drag-handle">
            <h3>Theme Settings</h3>
            <button class="settings-close-btn" onclick="toggleSettings()">&times;</button>
        </div>
        <div class="settings-content-wrapper">
            <form action="/save_theme" method="POST" class="settings-form">
                <input type="hidden" id="original_name" name="original_name" value="{current_theme_name}">

                <div class="settings-grid">
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

                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px;">
                        <div>
                            <label for="primary_bg">Primary BG:</label>
                            <input type="color" id="primary_bg" name="primary_bg" value="{primary_bg}">
                        </div>
                        <div>
                            <label for="secondary_bg">Secondary BG:</label>
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
                            <label for="border_color">Border Color:</label>
                            <input type="color" id="border_color" name="border_color" value="{border_color}">
                        </div>
                        <div>
                            <label for="tertiary_bg">Tertiary BG:</label>
                            <input type="color" id="tertiary_bg" name="tertiary_bg" value="{tertiary_bg}">
                        </div>
                        <div>
                            <label for="link_visited">Visited Color:</label>
                            <input type="color" id="link_visited" name="link_visited" value="{link_visited}">
                        </div>
                        <div>
                            <label for="link_hover">Hover Color:</label>
                            <input type="color" id="link_hover" name="link_hover" value="{link_hover}">
                        </div>
                    </div>

                    <div style="margin-top: 10px;">
                        <label>Font Sizes (px):</label>
                        <div style="display: flex; gap: 20px; margin-top: 5px;">
                            <label style="font-weight: normal; cursor: pointer; display: flex; align-items: center; gap: 5px;">
                                Small:
                                <input type="number" id="font_size_small" name="font_size_small" value="{font_size_small}" style="width: 50px; margin-left: 5px;">
                            </label>
                            <label style="font-weight: normal; cursor: pointer; display: flex; align-items: center; gap: 5px;">
                                Medium:
                                <input type="number" id="font_size_medium" name="font_size_medium" value="{font_size_medium}" style="width: 50px; margin-left: 5px;">
                            </label>
                            <label style="font-weight: normal; cursor: pointer; display: flex; align-items: center; gap: 5px;">
                                Large:
                                <input type="number" id="font_size_large" name="font_size_large" value="{font_size_large}" style="width: 50px; margin-left: 5px;">
                            </label>
                        </div>
                    </div>
                </div>

                <div class="theme-action-buttons">
                    <button type="submit" name="action" value="save" class="form-submit-btn" style="flex:1;">Save</button>
                    <button type="submit" name="action" value="apply_only" class="form-submit-btn" style="flex:1;">Apply</button>
                </div>
            </form>
        </div>
    </div>
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
        font_size_small = current_theme.font_size_small,
        font_size_medium = current_theme.font_size_medium,
        font_size_large = current_theme.font_size_large,
        theme_options = theme_options
    )
}

pub fn get_settings_js() -> String {
    r#"
    (function() {
        const settings = document.getElementById('floating-settings');
        const handle = document.getElementById('settings-drag-handle');
        const form = settings.querySelector('.settings-form');
        const styleElement = document.getElementById('current-theme-vars');
        const themeInputs = form.querySelectorAll('input[type="color"]');
        const fontSizeNumericInputs = form.querySelectorAll('input[type="number"]');

        window.toggleSettings = function() {
            if (settings.style.display === 'none') {
                settings.style.display = 'flex';
                localStorage.setItem('settings-visible', 'true');
            } else {
                settings.style.display = 'none';
                localStorage.setItem('settings-visible', 'false');
            }
        };

        // Live Preview logic
        const applyTheme = () => {
            let cssVars = ':root {';
            themeInputs.forEach(input => {
                cssVars += `--${input.id}: ${input.value};`; 
            });
            
            const smallVal = document.getElementById('font_size_small').value;
            const mediumVal = document.getElementById('font_size_medium').value;
            const largeVal = document.getElementById('font_size_large').value;
            
            cssVars += `--font-size-small: ${smallVal}px;`;
            cssVars += `--font-size-medium: ${mediumVal}px;`;
            cssVars += `--font-size-large: ${largeVal}px;`;
            cssVars += `--base-font-size: ${mediumVal}px;`;
            
            cssVars += '}';
            styleElement.innerHTML = cssVars;
        };

        themeInputs.forEach(input => {
            input.addEventListener('input', applyTheme);
        });

        fontSizeNumericInputs.forEach(input => {
            input.addEventListener('input', applyTheme);
        });

        // Drag logic
        let isDragging = false;
        let currentX;
        let currentY;
        let initialX;
        let initialY;
        let xOffset = 0;
        let yOffset = 0;

        // Restore position / visibility
        const savedPos = localStorage.getItem('settings-pos');
        if (savedPos) {
            const pos = JSON.parse(savedPos);
            xOffset = pos.x;
            yOffset = pos.y;
            settings.style.transform = `translate3d(${xOffset}px, ${yOffset}px, 0)`;
        }

        if (localStorage.getItem('settings-visible') === 'true') {
            settings.style.display = 'flex';
        }

        handle.addEventListener('mousedown', dragStart);
        document.addEventListener('mousemove', drag);
        document.addEventListener('mouseup', dragEnd);

        function dragStart(e) {
            initialX = e.clientX - xOffset;
            initialY = e.clientY - yOffset;
            if (e.target === handle || handle.contains(e.target)) {
                isDragging = true;
            }
        }

        function drag(e) {
            if (isDragging) {
                e.preventDefault();
                currentX = e.clientX - initialX;
                currentY = e.clientY - initialY;
                xOffset = currentX;
                yOffset = currentY;
                settings.style.transform = `translate3d(${currentX}px, ${currentY}px, 0)`;
            }
        }

        function dragEnd(e) {
            initialX = currentX;
            initialY = currentY;
            isDragging = false;
            if (currentX !== undefined && currentY !== undefined) {
                localStorage.setItem('settings-pos', JSON.stringify({x: currentX, y: currentY}));
            }
        }
    })();
    "#.to_string()
}
