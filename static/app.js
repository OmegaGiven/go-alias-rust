document.addEventListener('DOMContentLoaded', () => {
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-link-item');
    const SQL_CONNECTION_TABS_KEY = 'go_service_sql_connection_tabs';
    const sqlConnectionColors = [
        '#3b82f6',
        '#22c55e',
        '#f59e0b',
        '#ef4444',
        '#a855f7',
        '#14b8a6',
        '#f97316',
        '#ec4899',
        '#84cc16',
        '#06b6d4',
    ];

    links.forEach((link) => {
        const href = link.getAttribute('href');
        if (!href) return;

        if (href === '/' && path === '/') {
            link.classList.add('active');
        } else if (href === '/sql' && path !== '/sql') {
            link.classList.remove('active');
        } else if (href !== '/' && path.startsWith(href)) {
            link.classList.add('active');
        }
    });

    function readSqlConnectionTabs() {
        try {
            const tabs = JSON.parse(localStorage.getItem(SQL_CONNECTION_TABS_KEY) || '[]');
            return Array.isArray(tabs)
                ? tabs.filter((tab) => tab && typeof tab.nickname === 'string' && tab.nickname.trim() !== '')
                : [];
        } catch (_) {
            return [];
        }
    }

    function writeSqlConnectionTabs(tabs) {
        localStorage.setItem(SQL_CONNECTION_TABS_KEY, JSON.stringify(tabs));
    }

    function colorForSqlConnection(tabs, nickname) {
        const usedColors = new Set(tabs.map((tab) => tab.color).filter(Boolean));
        const availableColor = sqlConnectionColors.find((color) => !usedColors.has(color));
        if (availableColor) return availableColor;

        let hash = 0;
        for (let i = 0; i < nickname.length; i += 1) {
            hash = ((hash << 5) - hash + nickname.charCodeAt(i)) | 0;
        }
        return sqlConnectionColors[Math.abs(hash) % sqlConnectionColors.length];
    }

    function upsertSqlConnectionTab(nickname) {
        const cleanNickname = String(nickname || '').trim();
        if (!cleanNickname) return readSqlConnectionTabs();

        const tabs = readSqlConnectionTabs();
        const existing = tabs.find((tab) => tab.nickname === cleanNickname);
        if (existing) {
            existing.lastOpenedAt = Date.now();
            writeSqlConnectionTabs(tabs);
            return tabs;
        }

        tabs.push({
            nickname: cleanNickname,
            color: colorForSqlConnection(tabs, cleanNickname),
            lastOpenedAt: Date.now(),
        });
        writeSqlConnectionTabs(tabs);
        return tabs;
    }

    async function closeSqlConnectionTab(nickname) {
        const cleanNickname = String(nickname || '').trim();
        if (!cleanNickname) return;

        const tabs = readSqlConnectionTabs().filter((tab) => tab.nickname !== cleanNickname);
        writeSqlConnectionTabs(tabs);

        try {
            await fetch(`/sql/disconnect/${encodeURIComponent(cleanNickname)}`, { method: 'POST' });
        } catch (error) {
            console.error('Failed to disconnect SQL connection', error);
        }

        renderSqlConnectionTabs();
        if (decodeURIComponent(window.location.pathname.replace(/^\/sql\/?/, '')) === cleanNickname) {
            window.location.href = '/sql';
        }
    }

    function renderSqlConnectionTabs() {
        const sqlLink = document.querySelector('.nav-left a[href="/sql"]');
        if (!sqlLink) return;

        let tabList = document.getElementById('sql-connection-tabs');
        if (!tabList) {
            tabList = document.createElement('div');
            tabList.id = 'sql-connection-tabs';
            tabList.className = 'sql-connection-tabs';
            sqlLink.insertAdjacentElement('afterend', tabList);
        }

        const tabs = readSqlConnectionTabs();
        const activeConnection = document.getElementById('sql-active-connection')?.dataset.connection || '';
        tabList.innerHTML = '';

        tabs.forEach((tab) => {
            const tabEl = document.createElement('a');
            tabEl.href = `/sql/${encodeURIComponent(tab.nickname)}`;
            tabEl.className = 'nav-link-item sql-connection-tab';
            tabEl.style.setProperty('--sql-tab-accent', tab.color || 'var(--accent-color)');
            tabEl.title = `SQL connection: ${tab.nickname}`;
            if (tab.nickname === activeConnection) {
                tabEl.classList.add('active');
            }

            const label = document.createElement('span');
            label.className = 'sql-connection-tab-label';
            label.textContent = tab.nickname;

            const openButton = document.createElement('button');
            openButton.type = 'button';
            openButton.className = 'sql-connection-tab-action sql-connection-tab-open';
            openButton.setAttribute('aria-label', `Open another ${tab.nickname} SQL tab`);
            openButton.title = `Open another ${tab.nickname}`;
            openButton.innerHTML = `
                <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                    <path d="M8 3.5v9M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                </svg>
            `;
            openButton.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                window.open(`/sql/${encodeURIComponent(tab.nickname)}`, '_blank', 'noopener');
            });

            const closeButton = document.createElement('button');
            closeButton.type = 'button';
            closeButton.className = 'sql-connection-tab-action sql-connection-tab-close';
            closeButton.setAttribute('aria-label', `Close ${tab.nickname} SQL tab`);
            closeButton.title = `Close ${tab.nickname}`;
            closeButton.innerHTML = `
                <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                    <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
                </svg>
            `;
            closeButton.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                closeSqlConnectionTab(tab.nickname);
            });

            tabEl.appendChild(label);
            tabEl.appendChild(openButton);
            tabEl.appendChild(closeButton);
            tabList.appendChild(tabEl);
        });
    }

    const activeSqlConnectionEl = document.getElementById('sql-active-connection');
    if (activeSqlConnectionEl?.dataset.connection) {
        upsertSqlConnectionTab(activeSqlConnectionEl.dataset.connection);
    }
    renderSqlConnectionTabs();

    window.closeSqlConnectionTab = closeSqlConnectionTab;

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
