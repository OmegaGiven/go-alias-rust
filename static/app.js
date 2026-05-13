document.addEventListener('DOMContentLoaded', () => {
    const path = window.location.pathname;
    const links = document.querySelectorAll('.nav-link-item');
    const SQL_CONNECTION_TABS_KEY = 'ogdevdesk_sql_connection_tabs';
    const SQL_CONNECTION_GROUPS_COLLAPSED_KEY = 'ogdevdesk_sql_connection_groups_collapsed';
    const SQL_CONNECTION_GROUP_ORDER_KEY = 'ogdevdesk_sql_connection_group_order';
    const REQUEST_WORKSPACE_TABS_KEY = 'ogdevdesk_request_workspace_tabs';
    const TOP_NAV_ORDER_KEY = 'ogdevdesk_top_nav_order';
    const TOOL_WINDOW_MODE_KEY = 'ogdevdesk_tool_window_mode';
    let draggedSqlConnectionGroupName = '';
    let draggedTopNavItemName = '';
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
    const FLOATING_WINDOW_SELECTOR = [
        '#floating-settings',
        '#floating-documentation',
        '#floating-calculator',
        '#floating-jwt-decoder',
        '#floating-scratch-pad',
        '#floating-ai-assistant',
        '#floating-ai-settings',
    ].join(',');
    let floatingWindowZIndex = 10050;
    const desktopToolIds = {
        appearance: 'floating-settings',
        calculator: 'floating-calculator',
        jwt: 'floating-jwt-decoder',
        scratchpad: 'floating-scratch-pad',
        ai: 'floating-ai-assistant',
        documentation: 'floating-documentation',
    };
    let tauriBridgeRequestId = 0;
    const tauriBridgeRequests = new Map();

    function getToolWindowMode() {
        return localStorage.getItem(TOOL_WINDOW_MODE_KEY) === 'detached' ? 'detached' : 'floating';
    }

    function setToolWindowMode(mode) {
        const nextMode = mode === 'detached' ? 'detached' : 'floating';
        localStorage.setItem(TOOL_WINDOW_MODE_KEY, nextMode);
        document.querySelectorAll('[data-tool-window-mode-control]').forEach((control) => {
            control.value = nextMode;
        });
    }

    function setupToolWindowModeControls() {
        document.querySelectorAll('[data-tool-window-mode-control]').forEach((control) => {
            control.value = getToolWindowMode();
            if (control.dataset.toolWindowModeReady === 'true') return;
            control.dataset.toolWindowModeReady = 'true';
            control.addEventListener('change', () => setToolWindowMode(control.value));
        });
    }

    function tauriInvoke(command, payload = {}) {
        if (window.__TAURI__?.core?.invoke) {
            return window.__TAURI__.core.invoke(command, payload);
        }
        if (window.OGDEVDESK_DESKTOP_MODE === true && window.parent !== window) {
            const id = `app-tauri-${Date.now()}-${tauriBridgeRequestId++}`;
            return new Promise((resolve, reject) => {
                tauriBridgeRequests.set(id, { resolve, reject });
                window.parent.postMessage({
                    type: 'ogdevdesk-tauri-invoke',
                    id,
                    command,
                    payload,
                }, '*');
                window.setTimeout(() => {
                    const pending = tauriBridgeRequests.get(id);
                    if (!pending) return;
                    tauriBridgeRequests.delete(id);
                    pending.reject(new Error('Timed out waiting for the desktop app.'));
                }, 30000);
            });
        }
        return null;
    }

    window.isOgdevdeskDesktop = function () {
        return Boolean(window.__TAURI__?.core?.invoke) ||
            (window.OGDEVDESK_DESKTOP_MODE === true && window.parent !== window);
    };

    function handleDesktopToolOpenError(tool, error, fallback) {
        console.error(`Failed to open desktop ${tool} window`, error);
        if (typeof fallback === 'function') fallback();
    }

    function postDesktopShellMessage(message) {
        if (window.OGDEVDESK_DESKTOP_MODE !== true) return false;
        if (window.parent === window) return false;
        window.parent.postMessage(message, '*');
        return true;
    }

    window.addEventListener('message', (event) => {
        const data = event.data || {};
        if (data.type !== 'ogdevdesk-tauri-result' || typeof data.id !== 'string') return;
        const pending = tauriBridgeRequests.get(data.id);
        if (!pending) return;
        tauriBridgeRequests.delete(data.id);
        if (data.ok) {
            pending.resolve(data.result);
        } else {
            pending.reject(new Error(data.error || 'Desktop command failed.'));
        }
    });

    window.openDesktopToolWindow = function (tool, fallback) {
        if (!window.isOgdevdeskDesktop() || window.OGDEVDESK_DESKTOP_TOOL) return false;
        if (getToolWindowMode() !== 'detached') return false;
        const openPromise = tauriInvoke('open_tool_window', { tool });
        if (!openPromise && postDesktopShellMessage({ type: 'ogdevdesk-open-tool', tool })) {
            return true;
        }
        if (!openPromise) {
            fetch(`/desktop-open-tool/${encodeURIComponent(tool)}`, { method: 'POST' })
                .then((response) => {
                    if (!response.ok) {
                        return response.text().then((message) => {
                            throw new Error(message || `Desktop tool open failed with ${response.status}`);
                        });
                    }
                    return response;
                })
                .catch((error) => {
                    handleDesktopToolOpenError(tool, error, fallback);
                });
            return true;
        }

        openPromise.catch((error) => {
            handleDesktopToolOpenError(tool, error, fallback);
        });
        return true;
    };

    window.closeDesktopToolWindow = function () {
        if (!window.OGDEVDESK_DESKTOP_TOOL || !window.isOgdevdeskDesktop()) return false;
        const closePromise = tauriInvoke('close_current_window');
        if (!closePromise && postDesktopShellMessage({ type: 'ogdevdesk-close-tool' })) {
            return true;
        }
        if (!closePromise) {
            fetch(`/desktop-close-tool/${encodeURIComponent(window.OGDEVDESK_DESKTOP_TOOL)}`, { method: 'POST' })
                .then((response) => {
                    if (!response.ok) {
                        return response.text().then((message) => {
                            throw new Error(message || `Desktop tool close failed with ${response.status}`);
                        });
                    }
                    return response;
                })
                .catch((error) => {
                    console.error('Failed to close desktop tool window', error);
                    window.close();
                });
            return true;
        }

        closePromise.catch((error) => {
            console.error('Failed to close desktop tool window', error);
        });
        return true;
    };

    function setupDesktopUpdater() {
        const updateButton = document.getElementById('desktop-update-check-btn');
        if (!updateButton) return;
        updateButton.hidden = !window.isOgdevdeskDesktop();
        if (!window.isOgdevdeskDesktop() || updateButton.dataset.updateReady === 'true') return;
        updateButton.dataset.updateReady = 'true';

        updateButton.addEventListener('click', async () => {
            const originalText = updateButton.textContent;
            updateButton.disabled = true;
            updateButton.textContent = 'Checking...';
            try {
                const info = await tauriInvoke('check_for_update');
                if (!info?.available) {
                    window.alert(`OGdevDesk is up to date${info?.current_version ? ` (${info.current_version})` : ''}.`);
                    return;
                }

                const lines = [
                    `Version ${info.version} is available.`,
                    info.body ? `\n${info.body}` : '',
                    '\nInstall it now? OGdevDesk will restart after installation.',
                ];
                if (!window.confirm(lines.join('\n'))) return;

                updateButton.textContent = 'Installing...';
                await tauriInvoke('install_update');
            } catch (error) {
                window.alert(error?.message || String(error));
            } finally {
                updateButton.disabled = false;
                updateButton.textContent = originalText;
            }
        });
    }

    function openUrlInSystemBrowser(url) {
        if (!window.isOgdevdeskDesktop()) return false;
        const openPromise = tauriInvoke('open_url_in_browser', { url });
        if (!openPromise) return false;
        openPromise.catch((error) => {
            window.alert(error?.message || String(error));
        });
        return true;
    }

    function setupDesktopAliasLinks() {
        if (!window.isOgdevdeskDesktop()) return;
        document.addEventListener('click', (event) => {
            const link = event.target.closest('a');
            if (!link || !link.closest('.shortcut-sections')) return;
            const href = link.getAttribute('href');
            if (!href || href.startsWith('#') || href.startsWith('javascript:')) return;

            let targetUrl = href;
            if (href.startsWith('/')) {
                targetUrl = new URL(href, window.location.origin).toString();
            }

            if (!/^https?:\/\//i.test(targetUrl)) return;

            event.preventDefault();
            openUrlInSystemBrowser(targetUrl);
        });
    }

    function bringFloatingWindowToFront(windowEl) {
        if (!windowEl) return;
        floatingWindowZIndex += 1;
        windowEl.style.zIndex = String(floatingWindowZIndex);
    }

    function setupFloatingWindowLayering() {
        document.querySelectorAll(FLOATING_WINDOW_SELECTOR).forEach((windowEl) => {
            if (windowEl.dataset.floatLayerReady === 'true') return;
            windowEl.dataset.floatLayerReady = 'true';
            windowEl.addEventListener('pointerdown', () => bringFloatingWindowToFront(windowEl), true);
            windowEl.addEventListener('focusin', () => bringFloatingWindowToFront(windowEl), true);
        });
    }

    function activateDesktopToolPage() {
        const tool = window.OGDEVDESK_DESKTOP_TOOL;
        const toolId = desktopToolIds[tool];
        const toolEl = toolId ? document.getElementById(toolId) : null;
        if (!toolEl) return;

        document.body.classList.add('desktop-tool-page');
        document.body.dataset.desktopTool = tool;
        toolEl.classList.add('desktop-tool-active');
        toolEl.style.display = 'flex';
        toolEl.style.transform = 'none';
        toolEl.querySelectorAll('.floating-window-close').forEach((button) => {
            button.addEventListener('click', (event) => {
                if (!window.closeDesktopToolWindow()) return;
                event.preventDefault();
                event.stopImmediatePropagation();
            }, true);
        });

        if (tool === 'documentation') {
            const documentationSearch = document.getElementById('documentation-search');
            if (documentationSearch) {
                setTimeout(() => documentationSearch.focus(), 0);
            }
        }
    }

    window.bringFloatingWindowToFront = bringFloatingWindowToFront;

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
                ? tabs
                    .filter((tab) => tab && typeof tab.nickname === 'string' && tab.nickname.trim() !== '')
                    .map((tab) => ({
                        ...tab,
                        id: tab.id || makeSqlConnectionTabId(tab.nickname),
                    }))
                : [];
        } catch (_) {
            return [];
        }
    }

    function writeSqlConnectionTabs(tabs) {
        localStorage.setItem(SQL_CONNECTION_TABS_KEY, JSON.stringify(tabs));
    }

    function readCollapsedSqlConnectionGroups() {
        try {
            const values = JSON.parse(localStorage.getItem(SQL_CONNECTION_GROUPS_COLLAPSED_KEY) || '[]');
            return new Set(Array.isArray(values) ? values : []);
        } catch (_) {
            return new Set();
        }
    }

    function writeCollapsedSqlConnectionGroups(groups) {
        localStorage.setItem(SQL_CONNECTION_GROUPS_COLLAPSED_KEY, JSON.stringify(Array.from(groups)));
    }

    function readSqlConnectionGroupOrder() {
        try {
            const values = JSON.parse(localStorage.getItem(SQL_CONNECTION_GROUP_ORDER_KEY) || '[]');
            return Array.isArray(values) ? values.filter((value) => typeof value === 'string') : [];
        } catch (_) {
            return [];
        }
    }

    function writeSqlConnectionGroupOrder(order) {
        localStorage.setItem(SQL_CONNECTION_GROUP_ORDER_KEY, JSON.stringify(order));
    }

    function orderedSqlConnectionGroups(groups) {
        const order = readSqlConnectionGroupOrder();
        const byName = new Map(groups.map((group) => [group.nickname, group]));
        const ordered = order.map((name) => byName.get(name)).filter(Boolean);
        const remaining = groups.filter((group) => !order.includes(group.nickname));
        return [...ordered, ...remaining];
    }

    function clearSqlConnectionGroupDropClasses(container) {
        container?.querySelectorAll('.sql-connection-group.drop-before, .sql-connection-group.drop-after, .sql-connection-group.dragging, .sql-connection-group.dragging-target').forEach((group) => {
            group.classList.remove('drop-before', 'drop-after', 'dragging', 'dragging-target');
        });
    }

    function moveSqlConnectionGroup(draggedName, targetName, placeAfter, groups) {
        if (!draggedName || !targetName || draggedName === targetName) return false;
        const visibleNames = orderedSqlConnectionGroups(groups).map((item) => item.nickname);
        const nextOrder = visibleNames.filter((name) => name !== draggedName);
        const targetIndex = nextOrder.indexOf(targetName);
        if (targetIndex < 0) return false;

        nextOrder.splice(targetIndex + (placeAfter ? 1 : 0), 0, draggedName);
        writeSqlConnectionGroupOrder(nextOrder);
        return true;
    }

    function readTopNavOrder() {
        try {
            const values = JSON.parse(localStorage.getItem(TOP_NAV_ORDER_KEY) || '[]');
            return Array.isArray(values) ? values.filter((value) => typeof value === 'string') : [];
        } catch (_) {
            return [];
        }
    }

    function writeTopNavOrder(order) {
        localStorage.setItem(TOP_NAV_ORDER_KEY, JSON.stringify(order));
    }

    function topNavItems() {
        const navLeft = document.querySelector('.nav-left');
        if (!navLeft) return [];
        return Array.from(navLeft.querySelectorAll(':scope > [data-top-nav-item]'));
    }

    function clearTopNavDropClasses() {
        topNavItems().forEach((item) => {
            item.classList.remove('top-nav-dragging', 'top-nav-drop-before', 'top-nav-drop-after');
        });
    }

    function orderedTopNavItems(items) {
        const order = readTopNavOrder();
        const byName = new Map(items.map((item) => [item.dataset.topNavItem, item]));
        const ordered = order.map((name) => byName.get(name)).filter(Boolean);
        const remaining = items.filter((item) => !order.includes(item.dataset.topNavItem));
        return [...ordered, ...remaining];
    }

    function applyTopNavOrder() {
        const navLeft = document.querySelector('.nav-left');
        if (!navLeft) return;
        orderedTopNavItems(topNavItems()).forEach((item) => navLeft.appendChild(item));
    }

    function moveTopNavItem(draggedName, targetName, placeAfter) {
        if (!draggedName || !targetName || draggedName === targetName) return false;
        const names = orderedTopNavItems(topNavItems()).map((item) => item.dataset.topNavItem);
        const nextOrder = names.filter((name) => name !== draggedName);
        const targetIndex = nextOrder.indexOf(targetName);
        if (targetIndex < 0) return false;
        nextOrder.splice(targetIndex + (placeAfter ? 1 : 0), 0, draggedName);
        writeTopNavOrder(nextOrder);
        return true;
    }

    function setupTopNavDraggableItem(item) {
        if (!item || item.dataset.topNavDragReady === 'true') return;
        item.dataset.topNavDragReady = 'true';
        item.draggable = true;

        item.addEventListener('dragstart', (event) => {
            if (event.target.closest('.sql-connection-group')) return;
            if (event.target.closest('.request-workspace-tab')) return;
            draggedTopNavItemName = item.dataset.topNavItem || '';
            if (!draggedTopNavItemName) return;
            event.dataTransfer.effectAllowed = 'move';
            event.dataTransfer.setData('text/plain', draggedTopNavItemName);
            item.classList.add('top-nav-dragging');
        });

        item.addEventListener('dragend', () => {
            clearTopNavDropClasses();
            draggedTopNavItemName = '';
        });

        item.addEventListener('dragover', (event) => {
            if (!draggedTopNavItemName || draggedTopNavItemName === item.dataset.topNavItem) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = 'move';
            const rect = item.getBoundingClientRect();
            const placeAfter = event.clientX > rect.left + rect.width / 2;
            clearTopNavDropClasses();
            item.classList.toggle('top-nav-drop-before', !placeAfter);
            item.classList.toggle('top-nav-drop-after', placeAfter);
        });

        item.addEventListener('dragleave', (event) => {
            if (item.contains(event.relatedTarget)) return;
            item.classList.remove('top-nav-drop-before', 'top-nav-drop-after');
        });

        item.addEventListener('drop', (event) => {
            if (!draggedTopNavItemName || draggedTopNavItemName === item.dataset.topNavItem) return;
            event.preventDefault();
            event.stopPropagation();
            const rect = item.getBoundingClientRect();
            const placeAfter = event.clientX > rect.left + rect.width / 2;
            if (moveTopNavItem(draggedTopNavItemName, item.dataset.topNavItem, placeAfter)) {
                clearTopNavDropClasses();
                draggedTopNavItemName = '';
                applyTopNavOrder();
            }
        });
    }

    function ensureSqlNavCluster() {
        const navLeft = document.querySelector('.nav-left');
        const sqlLink = document.querySelector('.nav-left a[href="/sql"]');
        if (!navLeft || !sqlLink) return null;

        let cluster = document.getElementById('sql-nav-cluster');
        if (!cluster) {
            cluster = document.createElement('div');
            cluster.id = 'sql-nav-cluster';
            cluster.className = 'sql-nav-cluster';
            cluster.dataset.topNavItem = 'sql';
            navLeft.insertBefore(cluster, sqlLink);
            cluster.appendChild(sqlLink);
        }

        sqlLink.classList.add('sql-nav-main-link');
        return { navLeft, sqlLink, cluster };
    }

    function ensureRequestNavCluster() {
        const navLeft = document.querySelector('.nav-left');
        const requestsLink = document.querySelector('.nav-left a[href="/requests"]');
        if (!navLeft || !requestsLink) return null;

        let cluster = document.getElementById('request-nav-cluster');
        if (!cluster) {
            cluster = document.createElement('div');
            cluster.id = 'request-nav-cluster';
            cluster.className = 'request-nav-cluster';
            cluster.dataset.topNavItem = 'requests';
            navLeft.insertBefore(cluster, requestsLink);
            cluster.appendChild(requestsLink);
        }

        requestsLink.classList.add('request-nav-main-link');
        return { navLeft, requestsLink, cluster };
    }

    function setupTopNavDragging() {
        const navLeft = document.querySelector('.nav-left');
        const sqlCluster = ensureSqlNavCluster()?.cluster;
        const requestCluster = ensureRequestNavCluster()?.cluster;
        const inspectorLink = document.querySelector('.nav-left a[href="/inspector"]');
        if (inspectorLink) inspectorLink.dataset.topNavItem = 'inspector';

        [sqlCluster, requestCluster, inspectorLink].forEach(setupTopNavDraggableItem);
        applyTopNavOrder();

        if (navLeft && navLeft.dataset.topNavDropReady !== 'true') {
            navLeft.dataset.topNavDropReady = 'true';
            navLeft.addEventListener('dragover', (event) => {
                if (!draggedTopNavItemName) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = 'move';
            });
            navLeft.addEventListener('drop', (event) => {
                if (!draggedTopNavItemName) return;
                const target = event.target.closest('[data-top-nav-item]');
                if (target) return;
                const items = topNavItems();
                const last = items[items.length - 1];
                if (last && moveTopNavItem(draggedTopNavItemName, last.dataset.topNavItem, true)) {
                    event.preventDefault();
                    clearTopNavDropClasses();
                    draggedTopNavItemName = '';
                    applyTopNavOrder();
                }
            });
        }
    }

    function makeRequestWorkspaceTabId() {
        return `request-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    }

    function requestWorkspaceStorageKey(tabId) {
        return `request_workspace_${tabId || 'default'}`;
    }

    function readRequestWorkspaceTabs() {
        try {
            const tabs = JSON.parse(localStorage.getItem(REQUEST_WORKSPACE_TABS_KEY) || '[]');
            return Array.isArray(tabs)
                ? tabs
                    .filter((tab) => tab && typeof tab.id === 'string')
                    .map((tab) => ({
                        id: tab.id,
                        title: String(tab.title || 'Request').trim() || 'Request',
                        lastOpenedAt: Number(tab.lastOpenedAt || Date.now()),
                    }))
                : [];
        } catch (_) {
            return [];
        }
    }

    function writeRequestWorkspaceTabs(tabs) {
        localStorage.setItem(REQUEST_WORKSPACE_TABS_KEY, JSON.stringify(tabs));
    }

    function readRequestWorkspace(tabId) {
        try {
            return JSON.parse(localStorage.getItem(requestWorkspaceStorageKey(tabId)) || '{}');
        } catch (_) {
            return {};
        }
    }

    function writeRequestWorkspace(tabId, workspace) {
        if (!tabId) return;
        localStorage.setItem(requestWorkspaceStorageKey(tabId), JSON.stringify(workspace || {}));
    }

    function createRequestWorkspaceTab(workspace = {}) {
        const id = makeRequestWorkspaceTabId();
        const title = String(workspace.title || workspace.name || 'New Request').trim() || 'New Request';
        const tabs = readRequestWorkspaceTabs();
        const tab = { id, title, lastOpenedAt: Date.now() };
        tabs.push(tab);
        writeRequestWorkspaceTabs(tabs);
        writeRequestWorkspace(id, { ...workspace, title });
        renderRequestWorkspaceTabs();
        return tab;
    }

    function updateRequestWorkspaceTab(tabId, workspace = {}) {
        if (!tabId) return;
        const tabs = readRequestWorkspaceTabs();
        const tab = tabs.find((candidate) => candidate.id === tabId);
        if (tab) {
            const title = String(workspace.title || workspace.name || tab.title || 'Request').trim() || 'Request';
            tab.title = title;
            tab.lastOpenedAt = Date.now();
            writeRequestWorkspaceTabs(tabs);
        }
        writeRequestWorkspace(tabId, workspace);
        renderRequestWorkspaceTabs();
    }

    function closeRequestWorkspaceTab(tabId) {
        const tabs = readRequestWorkspaceTabs();
        const nextTabs = tabs.filter((tab) => tab.id !== tabId);
        writeRequestWorkspaceTabs(nextTabs);
        localStorage.removeItem(requestWorkspaceStorageKey(tabId));
        renderRequestWorkspaceTabs();

        const activeTabId = new URLSearchParams(window.location.search).get('tab') || '';
        if (path.startsWith('/requests') && activeTabId === tabId) {
            const nextTab = nextTabs[0];
            window.location.href = nextTab ? `/requests?tab=${encodeURIComponent(nextTab.id)}` : '/requests';
        }
    }

    function renderRequestWorkspaceTabs() {
        const requestNav = ensureRequestNavCluster();
        if (!requestNav) return;
        const { cluster } = requestNav;

        let tabList = document.getElementById('request-workspace-tabs');
        if (!tabList) {
            tabList = document.createElement('div');
            tabList.id = 'request-workspace-tabs';
            tabList.className = 'request-workspace-tabs';
            cluster.appendChild(tabList);
        }

        const activeTabId = new URLSearchParams(window.location.search).get('tab') || '';
        const tabs = readRequestWorkspaceTabs();
        tabList.innerHTML = '';
        tabList.hidden = tabs.length === 0;
        cluster.classList.toggle('has-tabs', tabs.length > 0);
        cluster.classList.toggle('active', tabs.length > 0 && path.startsWith('/requests'));

        tabs.forEach((tab) => {
            const workspace = readRequestWorkspace(tab.id);
            const labelText = String(workspace.title || workspace.name || tab.title || 'Request').trim() || 'Request';
            const tabEl = document.createElement('a');
            tabEl.href = `/requests?tab=${encodeURIComponent(tab.id)}`;
            tabEl.className = 'nav-link-item request-workspace-tab';
            tabEl.title = `Request tab: ${labelText}`;
            tabEl.addEventListener('click', (event) => {
                if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
                    return;
                }
                if (!path.startsWith('/requests') || typeof window.switchRequestWorkspaceTab !== 'function') {
                    return;
                }
                event.preventDefault();
                window.switchRequestWorkspaceTab(tab.id);
            });
            tabEl.classList.toggle('active', path.startsWith('/requests') && activeTabId === tab.id);

            const label = document.createElement('span');
            label.className = 'request-workspace-tab-label';
            label.textContent = labelText;

            const closeButton = document.createElement('button');
            closeButton.type = 'button';
            closeButton.className = 'request-workspace-tab-action request-workspace-tab-close';
            closeButton.setAttribute('aria-label', `Close ${labelText} request tab`);
            closeButton.title = `Close ${labelText}`;
            closeButton.innerHTML = `
                <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                    <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
                </svg>
            `;
            closeButton.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                closeRequestWorkspaceTab(tab.id);
            });

            tabEl.appendChild(label);
            tabEl.appendChild(closeButton);
            tabList.appendChild(tabEl);
        });
    }

    function sqlWorkspaceStorageKey(nickname, tabId) {
        return `sql_workspace_${nickname}_${tabId || 'default'}`;
    }

    function readSqlTabWorkspace(nickname, tabId) {
        try {
            return JSON.parse(localStorage.getItem(sqlWorkspaceStorageKey(nickname, tabId)) || '{}');
        } catch (_) {
            return {};
        }
    }

    function writeSqlTabWorkspace(nickname, tabId, workspace) {
        const cleanNickname = String(nickname || '').trim();
        if (!cleanNickname || !tabId) return;
        localStorage.setItem(sqlWorkspaceStorageKey(cleanNickname, tabId), JSON.stringify(workspace || {}));
    }

    function makeSqlConnectionTabId(nickname) {
        return `${String(nickname || '').trim()}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    }

    function colorForSqlConnection(tabs, nickname) {
        const existingConnectionColor = tabs.find((tab) => tab.nickname === nickname && tab.color)?.color;
        if (existingConnectionColor) return existingConnectionColor;

        const usedColors = new Set(tabs.map((tab) => tab.color).filter(Boolean));
        const availableColor = sqlConnectionColors.find((color) => !usedColors.has(color));
        if (availableColor) return availableColor;

        let hash = 0;
        for (let i = 0; i < nickname.length; i += 1) {
            hash = ((hash << 5) - hash + nickname.charCodeAt(i)) | 0;
        }
        return sqlConnectionColors[Math.abs(hash) % sqlConnectionColors.length];
    }

    function upsertSqlConnectionTab(nickname, preferredId = '') {
        const cleanNickname = String(nickname || '').trim();
        if (!cleanNickname) return readSqlConnectionTabs();

        const tabs = readSqlConnectionTabs();
        const existing = preferredId
            ? tabs.find((tab) => tab.id === preferredId)
            : tabs.find((tab) => tab.nickname === cleanNickname);
        if (existing) {
            existing.lastOpenedAt = Date.now();
            writeSqlConnectionTabs(tabs);
            return tabs;
        }

        tabs.push({
            id: preferredId || makeSqlConnectionTabId(cleanNickname),
            nickname: cleanNickname,
            color: colorForSqlConnection(tabs, cleanNickname),
            lastOpenedAt: Date.now(),
        });
        writeSqlConnectionTabs(tabs);
        return tabs;
    }

    function createSqlConnectionTab(nickname) {
        const cleanNickname = String(nickname || '').trim();
        if (!cleanNickname) return null;

        const id = makeSqlConnectionTabId(cleanNickname);
        const tabs = readSqlConnectionTabs();
        tabs.push({
            id,
            nickname: cleanNickname,
            color: colorForSqlConnection(tabs, cleanNickname),
            lastOpenedAt: Date.now(),
        });
        writeSqlConnectionTabs(tabs);
        return tabs[tabs.length - 1];
    }

    async function closeSqlConnectionTab(tabIdOrNickname) {
        const tabsBeforeClose = readSqlConnectionTabs();
        const closingTab = tabsBeforeClose.find((tab) => tab.id === tabIdOrNickname)
            || tabsBeforeClose.find((tab) => tab.nickname === tabIdOrNickname);
        const cleanNickname = String(closingTab?.nickname || tabIdOrNickname || '').trim();
        if (!cleanNickname) return;

        const tabs = tabsBeforeClose.filter((tab) => {
            if (closingTab) return tab.id !== closingTab.id;
            return tab.nickname !== cleanNickname;
        });
        writeSqlConnectionTabs(tabs);

        const remainingSameConnection = tabs.filter((tab) => tab.nickname === cleanNickname);
        if (remainingSameConnection.length === 0) {
            try {
                await fetch(`/sql/disconnect/${encodeURIComponent(cleanNickname)}`, { method: 'POST' });
            } catch (error) {
                console.error('Failed to disconnect SQL connection', error);
            }
        }

        renderSqlConnectionTabs();
        const activeConnection = decodeURIComponent(window.location.pathname.replace(/^\/sql\/?/, ''));
        const activeTabId = new URLSearchParams(window.location.search).get('tab') || '';
        const closedActiveTab = closingTab
            ? activeTabId === closingTab.id || (!activeTabId && activeConnection === closingTab.nickname)
            : activeConnection === cleanNickname;

        if (closedActiveTab) {
            const nextTab = remainingSameConnection[0];
            window.location.href = nextTab
                ? `/sql/${encodeURIComponent(nextTab.nickname)}?tab=${encodeURIComponent(nextTab.id)}`
                : '/sql';
        }
    }

    function renderSqlConnectionTabs() {
        const sqlNav = ensureSqlNavCluster();
        if (!sqlNav) return;
        const { cluster } = sqlNav;

        let tabList = document.getElementById('sql-connection-tabs');
        if (!tabList) {
            tabList = document.createElement('div');
            tabList.id = 'sql-connection-tabs';
            tabList.className = 'sql-connection-tabs';
            cluster.appendChild(tabList);
        }

        const tabs = readSqlConnectionTabs();
        const activeConnection = document.getElementById('sql-active-connection')?.dataset.connection || '';
        const activeTabId = new URLSearchParams(window.location.search).get('tab') || '';
        const collapsedGroups = readCollapsedSqlConnectionGroups();
        let activatedFallbackTab = false;
        tabList.innerHTML = '';

        const groups = [];
        tabs.forEach((tab) => {
            let group = groups.find((candidate) => candidate.nickname === tab.nickname);
            if (!group) {
                group = {
                    nickname: tab.nickname,
                    color: tab.color || colorForSqlConnection(tabs, tab.nickname),
                    tabs: [],
                };
                groups.push(group);
            }
            group.tabs.push(tab);
        });

        const hasOpenSqlTabs = groups.some((group) => group.tabs.length > 0);
        cluster.classList.toggle('has-tabs', hasOpenSqlTabs);
        cluster.classList.toggle('active', hasOpenSqlTabs && path.startsWith('/sql'));

        tabList.ondragover = (event) => {
            if (!draggedSqlConnectionGroupName) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = 'move';
        };
        tabList.ondrop = (event) => {
            if (!draggedSqlConnectionGroupName) return;
            event.preventDefault();
            const groupsInDom = Array.from(tabList.querySelectorAll('.sql-connection-group'));
            const targetGroup = event.target.closest('.sql-connection-group');
            if (!targetGroup) {
                const lastGroup = groupsInDom[groupsInDom.length - 1];
                const targetName = lastGroup?.dataset.connectionGroup || '';
                if (moveSqlConnectionGroup(draggedSqlConnectionGroupName, targetName, true, groups)) {
                    clearSqlConnectionGroupDropClasses(tabList);
                    draggedSqlConnectionGroupName = '';
                    renderSqlConnectionTabs();
                }
            }
        };

        orderedSqlConnectionGroups(groups).forEach((group) => {
            const isCollapsed = collapsedGroups.has(group.nickname);
            const groupEl = document.createElement('div');
            groupEl.className = 'sql-connection-group';
            groupEl.draggable = true;
            groupEl.dataset.connectionGroup = group.nickname;
            groupEl.classList.toggle('active', group.nickname === activeConnection);
            groupEl.classList.toggle('collapsed', isCollapsed);
            groupEl.style.setProperty('--sql-tab-accent', group.color || 'var(--accent-color)');

            groupEl.addEventListener('dragstart', (event) => {
                draggedSqlConnectionGroupName = group.nickname;
                event.dataTransfer.effectAllowed = 'move';
                event.dataTransfer.setData('text/plain', group.nickname);
                event.dataTransfer.setData('application/x-go-sql-connection-group', group.nickname);
                groupEl.classList.add('dragging');
            });
            groupEl.addEventListener('dragend', () => {
                clearSqlConnectionGroupDropClasses(tabList);
                draggedSqlConnectionGroupName = '';
            });
            groupEl.addEventListener('dragover', (event) => {
                const draggedName = draggedSqlConnectionGroupName;
                if (!draggedName || draggedName === group.nickname) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = 'move';
                const rect = groupEl.getBoundingClientRect();
                const placeAfter = event.clientX > rect.left + rect.width / 2;
                clearSqlConnectionGroupDropClasses(tabList);
                groupEl.classList.add('dragging-target');
                groupEl.classList.toggle('drop-before', !placeAfter);
                groupEl.classList.toggle('drop-after', placeAfter);
            });
            groupEl.addEventListener('dragleave', (event) => {
                if (groupEl.contains(event.relatedTarget)) return;
                groupEl.classList.remove('drop-before', 'drop-after', 'dragging-target');
            });
            groupEl.addEventListener('drop', (event) => {
                const draggedName = draggedSqlConnectionGroupName;
                if (!draggedName || draggedName === group.nickname) return;
                event.preventDefault();
                event.stopPropagation();
                const rect = groupEl.getBoundingClientRect();
                const placeAfter = event.clientX > rect.left + rect.width / 2;
                if (moveSqlConnectionGroup(draggedName, group.nickname, placeAfter, groups)) {
                    clearSqlConnectionGroupDropClasses(tabList);
                    draggedSqlConnectionGroupName = '';
                    renderSqlConnectionTabs();
                }
            });

            const groupButton = document.createElement('button');
            groupButton.type = 'button';
            groupButton.className = 'sql-connection-group-label';
            groupButton.title = `${isCollapsed ? 'Expand' : 'Collapse'} ${group.nickname} SQL tabs`;
            groupButton.setAttribute('aria-expanded', String(!isCollapsed));
            groupButton.innerHTML = `
                <span class="sql-connection-group-caret">${isCollapsed ? '▸' : '▾'}</span>
                <span class="sql-connection-group-name"></span>
                <span class="sql-connection-group-count">${group.tabs.length}</span>
            `;
            groupButton.querySelector('.sql-connection-group-name').textContent = group.nickname;
            groupButton.addEventListener('click', () => {
                const nextCollapsed = readCollapsedSqlConnectionGroups();
                if (nextCollapsed.has(group.nickname)) {
                    nextCollapsed.delete(group.nickname);
                } else {
                    nextCollapsed.add(group.nickname);
                }
                writeCollapsedSqlConnectionGroups(nextCollapsed);
                renderSqlConnectionTabs();
            });
            groupEl.appendChild(groupButton);

            const groupTabsEl = document.createElement('div');
            groupTabsEl.className = 'sql-connection-group-tabs';
            groupTabsEl.hidden = isCollapsed;

            group.tabs.forEach((tab) => {
            const sameConnectionTabCount = group.tabs.length;
            const workspace = readSqlTabWorkspace(tab.nickname, tab.id);
            const tabNumber = group.tabs.findIndex((candidate) => candidate.id === tab.id) + 1;
            const tabLabel = String(workspace.queryName || '').trim() || `Unnamed ${tabNumber}`;
            const tabEl = document.createElement('a');
            tabEl.href = `/sql/${encodeURIComponent(tab.nickname)}?tab=${encodeURIComponent(tab.id)}`;
            tabEl.className = 'nav-link-item sql-connection-tab';
            tabEl.style.setProperty('--sql-tab-accent', tab.color || 'var(--accent-color)');
            tabEl.title = `SQL connection: ${tab.nickname}`;
            tabEl.addEventListener('click', (event) => {
                if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
                    return;
                }
                const activeConnectionName = decodeURIComponent(window.location.pathname.replace(/^\/sql\/?/, ''));
                const sameConnection = path.startsWith('/sql/') && activeConnectionName === tab.nickname;
                if (!sameConnection || typeof window.switchSqlWorkspaceTab !== 'function') return;
                event.preventDefault();
                window.switchSqlWorkspaceTab(tab.id);
            });
            if (tab.id === activeTabId || (!activeTabId && tab.nickname === activeConnection && !activatedFallbackTab)) {
                tabEl.classList.add('active');
                activatedFallbackTab = true;
            }

            const label = document.createElement('span');
            label.className = 'sql-connection-tab-label';
            label.textContent = tabLabel;

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
                const newTab = createSqlConnectionTab(tab.nickname);
                if (newTab) {
                    renderSqlConnectionTabs();
                    if (tab.nickname === activeConnection && typeof window.switchSqlWorkspaceTab === 'function') {
                        window.switchSqlWorkspaceTab(newTab.id);
                    } else {
                        window.location.href = `/sql/${encodeURIComponent(newTab.nickname)}?tab=${encodeURIComponent(newTab.id)}`;
                    }
                }
            });

            const closeButton = document.createElement('button');
            closeButton.type = 'button';
            closeButton.className = 'sql-connection-tab-action sql-connection-tab-close';
            const closeLabel = sameConnectionTabCount > 1
                ? `Close ${tab.nickname} SQL tab`
                : `Disconnect ${tab.nickname}`;
            closeButton.setAttribute('aria-label', closeLabel);
            closeButton.title = closeLabel;
            closeButton.innerHTML = `
                <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                    <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
                </svg>
            `;
            closeButton.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                closeSqlConnectionTab(tab.id);
            });

            tabEl.appendChild(label);
            tabEl.appendChild(openButton);
            tabEl.appendChild(closeButton);
            groupTabsEl.appendChild(tabEl);
            });

            groupEl.appendChild(groupTabsEl);
            tabList.appendChild(groupEl);
        });
    }

    const activeSqlConnectionEl = document.getElementById('sql-active-connection');
    if (activeSqlConnectionEl?.dataset.connection) {
        upsertSqlConnectionTab(
            activeSqlConnectionEl.dataset.connection,
            new URLSearchParams(window.location.search).get('tab') || ''
        );
    }
    setupTopNavDragging();
    renderSqlConnectionTabs();
    renderRequestWorkspaceTabs();

    window.closeSqlConnectionTab = closeSqlConnectionTab;
    window.renderSqlConnectionTabs = renderSqlConnectionTabs;
    window.createSqlConnectionTab = createSqlConnectionTab;
    window.writeSqlTabWorkspace = writeSqlTabWorkspace;
    window.createRequestWorkspaceTab = createRequestWorkspaceTab;
    window.updateRequestWorkspaceTab = updateRequestWorkspaceTab;
    window.readRequestWorkspace = readRequestWorkspace;
    window.renderRequestWorkspaceTabs = renderRequestWorkspaceTabs;

    const documentationWindow = document.getElementById('floating-documentation');
    const documentationHandle = document.getElementById('documentation-drag-handle');
    const documentationSearch = document.getElementById('documentation-search');
    const documentationEmpty = document.getElementById('documentation-empty');
    const documentationEntries = Array.from(document.querySelectorAll('.documentation-entry'));

    function documentationSectionForPath() {
        if (path.startsWith('/sql')) return 'sql';
        if (path.startsWith('/requests')) return 'requests';
        if (path.startsWith('/inspector')) return 'inspector';
        return 'aliases';
    }

    function filterDocumentation() {
        const query = String(documentationSearch?.value || '').trim().toLowerCase();
        let visibleCount = 0;

        documentationEntries.forEach((entry) => {
            const haystack = entry.textContent.toLowerCase();
            const visible = !query || haystack.includes(query);
            entry.hidden = !visible;
            if (visible) visibleCount += 1;
        });

        if (documentationEmpty) {
            documentationEmpty.hidden = visibleCount > 0;
        }
    }

    function focusDocumentationSection(sectionName) {
        documentationEntries.forEach((entry) => entry.classList.remove('documentation-entry-focused'));
        const section = document.querySelector(`.documentation-entry[data-doc-section="${sectionName}"]`);
        if (!section || section.hidden) return;

        section.classList.add('documentation-entry-focused');
        section.scrollIntoView({ block: 'start', behavior: 'smooth' });
    }

    function centerDocumentationWindow() {
        if (!documentationWindow) return;
        const rect = documentationWindow.getBoundingClientRect();
        documentationWindow.style.left = `${Math.max(10, (window.innerWidth - rect.width) / 2)}px`;
        documentationWindow.style.top = `${Math.max(10, (window.innerHeight - rect.height) / 2)}px`;
        documentationWindow.style.right = 'auto';
    }

    function bringDocumentationToFront() {
        if (!documentationWindow) return;
        bringFloatingWindowToFront(documentationWindow);
    }

    function toggleDocumentationInPage() {
        if (!documentationWindow) return;
        const isOpen = documentationWindow.style.display === 'flex';

        if (isOpen) {
            documentationWindow.style.display = 'none';
            return;
        }

        documentationWindow.style.display = 'flex';
        bringDocumentationToFront();
        if (documentationSearch) {
            documentationSearch.value = '';
            filterDocumentation();
            documentationSearch.focus();
        }
        requestAnimationFrame(() => focusDocumentationSection(documentationSectionForPath()));
    }

    window.toggleDocumentation = function () {
        if (window.openDesktopToolWindow?.('documentation', toggleDocumentationInPage)) return;
        toggleDocumentationInPage();
    };

    if (documentationSearch) {
        documentationSearch.addEventListener('input', filterDocumentation);
    }

    if (documentationWindow && documentationHandle) {
        let isDraggingDocumentation = false;
        let dragOffsetX = 0;
        let dragOffsetY = 0;

        documentationHandle.addEventListener('mousedown', (event) => {
            if (event.button !== 0) return;
            const rect = documentationWindow.getBoundingClientRect();
            isDraggingDocumentation = true;
            dragOffsetX = event.clientX - rect.left;
            dragOffsetY = event.clientY - rect.top;
            documentationWindow.style.left = `${rect.left}px`;
            documentationWindow.style.top = `${rect.top}px`;
            documentationWindow.style.right = 'auto';
            bringDocumentationToFront();
            document.body.style.userSelect = 'none';
        });

        document.addEventListener('mousemove', (event) => {
            if (!isDraggingDocumentation) return;
            const width = documentationWindow.offsetWidth;
            const height = documentationWindow.offsetHeight;
            const nextLeft = Math.min(Math.max(0, event.clientX - dragOffsetX), window.innerWidth - width);
            const nextTop = Math.min(Math.max(0, event.clientY - dragOffsetY), window.innerHeight - height);
            documentationWindow.style.left = `${nextLeft}px`;
            documentationWindow.style.top = `${nextTop}px`;
        });

        document.addEventListener('mouseup', () => {
            if (!isDraggingDocumentation) return;
            isDraggingDocumentation = false;
            document.body.style.userSelect = '';
        });

        documentationWindow.addEventListener('dblclick', (event) => {
            const rect = documentationWindow.getBoundingClientRect();
            const edgeSize = 12;
            const onEdge = event.clientX - rect.left <= edgeSize
                || rect.right - event.clientX <= edgeSize
                || event.clientY - rect.top <= edgeSize
                || rect.bottom - event.clientY <= edgeSize;
            if (onEdge) centerDocumentationWindow();
        });
    }

    const LOCAL_SHORTCUTS_KEY = 'ogdevdesk_local_shortcuts';
    const LOCAL_HIDDEN_SHORTCUTS_KEY = 'ogdevdesk_local_hidden_shortcuts';
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

    setupFloatingWindowLayering();
    setupToolWindowModeControls();
    setupDesktopUpdater();
    setupDesktopAliasLinks();
    window.addEventListener('storage', (event) => {
        if (event.key === TOOL_WINDOW_MODE_KEY) setupToolWindowModeControls();
    });
    activateDesktopToolPage();
});
