(function() {
    const sidebar = document.getElementById('sidebar');
    const sbResizer = document.getElementById('sidebar-resizer');
    if (!sidebar || !sbResizer) return;

    let isSbResizing = false;
    let lastSbDownX = 0;
    let savedSbWidth = 300;

    sbResizer.addEventListener('mousedown', (event) => {
        isSbResizing = true;
        lastSbDownX = event.clientX;
        sbResizer.classList.add('resizing');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
    });

    document.addEventListener('mousemove', (event) => {
        if (!isSbResizing) return;

        let newWidth = event.clientX;
        if (newWidth < 0) newWidth = 0;
        if (newWidth > 600) newWidth = 600;

        sidebar.style.width = `${newWidth}px`;
        if (newWidth === 0) {
            sidebar.classList.add('collapsed');
        } else {
            sidebar.classList.remove('collapsed');
        }
    });

    document.addEventListener('mouseup', (event) => {
        if (!isSbResizing) return;

        isSbResizing = false;
        sbResizer.classList.remove('resizing');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';

        if (Math.abs(event.clientX - lastSbDownX) < 10) {
            if (sidebar.offsetWidth === 0 || sidebar.classList.contains('collapsed')) {
                sidebar.classList.remove('collapsed');
                sidebar.style.width = `${savedSbWidth || 300}px`;
            } else {
                savedSbWidth = sidebar.offsetWidth;
                sidebar.classList.add('collapsed');
                sidebar.style.width = '0px';
            }
        } else if (sidebar.offsetWidth > 0) {
            savedSbWidth = sidebar.offsetWidth;
        }
    });
})();
