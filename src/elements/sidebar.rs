pub fn render(content: &str) -> String {
    format!(
        r#"
        <div id="sidebar">
            {content}
        </div>
        <div id="sidebar-resizer" title="Drag to resize, Click to toggle"></div>
        "#,
        content = content
    )
}
