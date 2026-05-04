document.addEventListener('DOMContentLoaded', () => {
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-link-item');

    links.forEach((link) => {
        const href = link.getAttribute('href');
        if (!href) return;

        if (href === '/' && path === '/') {
            link.classList.add('active');
        } else if (href !== '/' && path.startsWith(href)) {
            link.classList.add('active');
        }
    });

    const LOCAL_SHORTCUTS_KEY = 'go_service_local_shortcuts';
    const LOCAL_HIDDEN_SHORTCUTS_KEY = 'go_service_local_hidden_shortcuts';
    const modal = document.getElementById('addShortcutModal');
    const addShortcutTriggers = document.querySelectorAll('[data-add-shortcut-trigger]');
    const closeButton = document.getElementById('closeModalBtn');
    const form = modal ? modal.querySelector('form') : null;
    const scopeSelect = document.getElementById('shortcutScope');
    const scopeNote = document.getElementById('shortcutScopeNote');

    function readShortcutBucket(key) {
        try {
            return JSON.parse(localStorage.getItem(key) || '{}');
        } catch (_) {
            return {};
        }
    }

    function writeShortcutBucket(key, value) {
        localStorage.setItem(key, JSON.stringify(value));
    }

    function updateScopeNote() {
        if (!scopeSelect || !scopeNote) return;
        scopeNote.textContent = scopeSelect.value === 'global' || scopeSelect.value === 'hidden_global'
            ? 'Global shortcuts are saved on the server and visible to everyone using this instance.'
            : 'Local shortcuts stay in this browser only and are not sent to the server.';
    }

    if (addShortcutTriggers.length && modal) {
        addShortcutTriggers.forEach((trigger) => {
            trigger.addEventListener('click', () => modal.showModal());
        });
    }

    if (closeButton && modal) {
        closeButton.onclick = () => modal.close();
    }

    if (modal) {
        modal.addEventListener('click', (event) => {
            if (event.target.nodeName !== 'DIALOG') return;
            const rect = event.target.getBoundingClientRect();
            if (
                event.clientY < rect.top ||
                event.clientY > rect.bottom ||
                event.clientX < rect.left ||
                event.clientX > rect.right
            ) {
                modal.close();
            }
        });
    }

    if (scopeSelect) {
        scopeSelect.addEventListener('change', updateScopeNote);
        updateScopeNote();
    }

    if (form) {
        form.addEventListener('submit', (event) => {
            const scope = scopeSelect ? scopeSelect.value : 'global';
            if (scope !== 'local' && scope !== 'hidden_local') {
                return;
            }

            event.preventDefault();
            const shortcutInput = document.getElementById('shortcut');
            const urlInput = document.getElementById('url');
            const shortcut = shortcutInput ? shortcutInput.value.trim() : '';
            const url = urlInput ? urlInput.value.trim() : '';

            if (!shortcut || !url) return;

            const storageKey = scope === 'hidden_local' ? LOCAL_HIDDEN_SHORTCUTS_KEY : LOCAL_SHORTCUTS_KEY;
            const bucket = readShortcutBucket(storageKey);
            bucket[shortcut] = url;
            writeShortcutBucket(storageKey, bucket);

            modal.close();
            window.location.reload();
        });
    }
});
