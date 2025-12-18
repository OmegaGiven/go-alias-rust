pub fn get_css() -> &'static str {
    r#"
<style>
    /* Shared Sidebar CSS */
    #sidebar { 
        width: 300px; 
        min-width: 0;
        background: var(--secondary-bg); 
        border-right: 1px solid var(--border-color); 
        display: flex; 
        flex-direction: column; 
        padding: 0; 
        flex-shrink: 0;
        height: 100%; 
        box-sizing: border-box;
    }
    #sidebar.collapsed { width: 0 !important; padding: 0; overflow: hidden; border-right: none; }

    #sidebar h2 { margin-top: 0; font-size: 1.1em; border-bottom: 1px solid var(--border-color); padding: 10px; margin-bottom: 0px; flex-shrink: 0; white-space: nowrap; overflow: hidden; }

    #sidebar-resizer {
        width: 8px;
        background: var(--tertiary-bg);
        cursor: col-resize;
        flex-shrink: 0;
        height: 100%;
        z-index: 10;
        position: relative;
        transition: background-color 0.2s;
    }
    #sidebar-resizer:hover, #sidebar-resizer.resizing {
        background: var(--link-color);
        opacity: 0.8;
    }
</style>
    "#
}

pub fn get_js() -> &'static str {
    r#"
    (function() {
        const sidebar = document.getElementById('sidebar');
        const sbResizer = document.getElementById('sidebar-resizer');
        if (!sidebar || !sbResizer) return;

        let isSbResizing = false;
        let lastSbDownX = 0;
        let savedSbWidth = 300;

        sbResizer.addEventListener('mousedown', (e) => {
            isSbResizing = true;
            lastSbDownX = e.clientX;
            sbResizer.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
        });

        document.addEventListener('mousemove', (e) => {
            if (isSbResizing) {
                let newWidth = e.clientX;
                if (newWidth < 0) newWidth = 0;
                if (newWidth > 600) newWidth = 600;
                sidebar.style.width = newWidth + 'px';
                if (newWidth === 0) sidebar.classList.add('collapsed');
                else sidebar.classList.remove('collapsed');
            }
        });

        document.addEventListener('mouseup', (e) => {
            if (isSbResizing) {
                isSbResizing = false;
                sbResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                // Increased threshold for better click detection (especially when collapsed)
                if (Math.abs(e.clientX - lastSbDownX) < 10) {
                    if (sidebar.offsetWidth === 0 || sidebar.classList.contains('collapsed')) {
                        sidebar.classList.remove('collapsed');
                        sidebar.style.width = (savedSbWidth || 300) + 'px';
                    } else {
                        savedSbWidth = sidebar.offsetWidth;
                        sidebar.classList.add('collapsed');
                        sidebar.style.width = '0px';
                    }
                } else {
                    if (sidebar.offsetWidth > 0) savedSbWidth = sidebar.offsetWidth;
                }
            }
        });
    })();
    "#
}

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
