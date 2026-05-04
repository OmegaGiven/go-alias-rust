use crate::app_state::Theme;
use askama::Template;
use std::collections::HashMap;

#[derive(Clone)]
struct FontOption {
    value: &'static str,
    label: &'static str,
    selected: bool,
}

#[derive(Clone)]
struct ThemeNameOption {
    name: String,
    selected: bool,
}

#[derive(Template)]
#[template(path = "base.html")]
struct BasePageTemplate<'a> {
    title: &'a str,
    theme_vars: String,
    body_content: &'a str,
    current_theme: &'a Theme,
    saved_theme_options: Vec<ThemeNameOption>,
    font_options: Vec<FontOption>,
    show_add_shortcut_button: bool,
    include_add_shortcut_modal: bool,
}

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
    --accent-color: {};
    --border-color: {};
    --font-size-small: {}px;
    --font-size-medium: {}px;
    --font-size-large: {}px;
    --element-margin: {}px;
    --base-font-size: {}px;
    --base-font-family: {};
}}
</style>
"#,
        theme.primary_bg,
        theme.secondary_bg,
        theme.tertiary_bg,
        theme.text_color,
        theme.link_color,
        theme.link_visited,
        theme.link_hover,
        theme.accent_color,
        theme.border_color,
        theme.font_size_small,
        theme.font_size_medium,
        theme.font_size_large,
        theme.element_margin,
        theme.font_size_medium,
        theme.font_family
    )
}

fn font_options(selected_family: &str) -> Vec<FontOption> {
    [
        ("sans-serif", "Sans Serif"),
        ("Arial, sans-serif", "Arial"),
        ("'Segoe UI', Tahoma, Geneva, Verdana, sans-serif", "Segoe UI"),
        ("'Helvetica Neue', Helvetica, Arial, sans-serif", "Helvetica"),
        ("Georgia, 'Times New Roman', serif", "Georgia"),
        ("'Trebuchet MS', sans-serif", "Trebuchet"),
        ("'Courier New', Courier, monospace", "Courier New"),
        ("'Comic Sans MS', 'Comic Sans', 'Chalkboard SE', 'Marker Felt', cursive", "Comic / Chalkboard"),
        ("Impact, Haettenschweiler, 'Arial Narrow Bold', sans-serif", "Impact Display"),
        ("'Brush Script MT', 'Lucida Handwriting', cursive", "Brush Script"),
    ]
    .into_iter()
    .map(|(value, label)| FontOption {
        value,
        label,
        selected: value == selected_family,
    })
    .collect()
}

pub fn render_base_page(
    title: &str,
    body_content: &str,
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
) -> String {
    render_base_page_with_options(
        title,
        body_content,
        current_theme,
        saved_themes,
        false,
        false,
    )
}

pub fn render_base_page_with_options(
    title: &str,
    body_content: &str,
    current_theme: &Theme,
    saved_themes: &HashMap<String, Theme>,
    show_add_shortcut_button: bool,
    include_add_shortcut_modal: bool,
) -> String {
    let mut saved_theme_options = saved_themes
        .keys()
        .map(|name| ThemeNameOption {
            name: name.clone(),
            selected: name == &current_theme.name,
        })
        .collect::<Vec<_>>();
    saved_theme_options.sort_by(|left, right| left.name.cmp(&right.name));

    BasePageTemplate {
        title,
        theme_vars: render_theme_variables(current_theme),
        body_content,
        current_theme,
        saved_theme_options,
        font_options: font_options(&current_theme.font_family),
        show_add_shortcut_button,
        include_add_shortcut_modal,
    }
    .render()
    .unwrap_or_else(|err| {
        format!(
            "<!DOCTYPE html><html><body><h1>Template error</h1><pre>{}</pre></body></html>",
            htmlescape::encode_minimal(&err.to_string())
        )
    })
}

pub fn render_inline_add_shortcut_button() -> String {
    r#"
    <button type="button" class="form-submit-btn shortcut-inline-add-btn" data-add-shortcut-trigger>+ Add Shortcut</button>
    "#.to_string()
}
