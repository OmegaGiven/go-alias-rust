        const methodSelect = document.getElementById('method');
        const urlInput = document.getElementById('url');
        const bodyInput = document.getElementById('body-input');
        const sendBtn = document.getElementById('send-btn');
        const cancelBtn = document.getElementById('cancel-btn');
        const responseBody = document.getElementById('response-body');
        const responseHeaders = document.getElementById('response-headers');
        const headersBodyResizer = document.getElementById('headers-body-resizer');
        const resStatus = document.getElementById('res-status');
        const resTime = document.getElementById('res-time');
        const resSize = document.getElementById('res-size');
        const downloadResBtn = document.getElementById('download-res-btn');
        const openInspectorBtn = document.getElementById('open-inspector-btn');
        const viewCurlBtn = document.getElementById('view-curl-btn');
        const requestHistorySelect = document.getElementById('request-history-select');
        const deleteRequestHistoryBtn = document.getElementById('delete-request-history-btn');
        const clearRequestHistoryBtn = document.getElementById('clear-request-history-btn');
        const curlViewModal = document.getElementById('curl-view-modal');
        const curlViewOutput = document.getElementById('curl-view-output');
        const closeCurlViewBtn = document.getElementById('close-curl-view-btn');
        const requestDebugInfo = document.getElementById('request-debug-info');
        const savedRequestSearch = document.getElementById('saved-request-search');
        const savedList = document.getElementById('saved-list');
        const newRequestBtn = document.getElementById('new-request-btn');
        const importPostmanBtn = document.getElementById('import-postman-btn');
        const postmanImportModal = document.getElementById('postman-import-modal');
        const closePostmanImportBtn = document.getElementById('close-postman-import-btn');
        const postmanImportFile = document.getElementById('postman-import-file');
        const postmanDuplicateMode = document.getElementById('postman-duplicate-mode');
        const postmanImportPreview = document.getElementById('postman-import-preview');
        const confirmPostmanImportBtn = document.getElementById('confirm-postman-import-btn');
        const createRequestFolderBtn = document.getElementById('create-request-folder-btn');
        const createRequestFolderForm = document.getElementById('create-request-folder-form');
        const newRequestFolderName = document.getElementById('new-request-folder-name');
        const requestVariablesBtn = document.getElementById('request-variables-btn');
        const requestVariablesModal = document.getElementById('request-variables-modal');
        const closeRequestVariablesBtn = document.getElementById('close-request-variables-btn');
        const requestVariableSetSelect = document.getElementById('request-variable-set-select');
        const newRequestVariableSetModal = document.getElementById('new-request-variable-set-modal');
        const newRequestVariableSetName = document.getElementById('new-request-variable-set-name');
        const addRequestVariableSetBtn = document.getElementById('add-request-variable-set-btn');
        const renameRequestVariableSetBtn = document.getElementById('rename-request-variable-set-btn');
        const copyRequestVariableSetBtn = document.getElementById('copy-request-variable-set-btn');
        const deleteRequestVariableSetBtn = document.getElementById('delete-request-variable-set-btn');
        const createRequestVariableSetBtn = document.getElementById('create-request-variable-set-btn');
        const cancelRequestVariableSetBtn = document.getElementById('cancel-request-variable-set-btn');
        const requestVariableSetModalTitle = document.getElementById('request-variable-set-modal-title');
        const requestVariableSetModalDescription = document.getElementById('request-variable-set-modal-description');
        const requestVariablesContainer = document.getElementById('request-variables-container');
        const addRequestVariableBtn = document.getElementById('add-request-variable-btn');
        const saveRequestVariablesBtn = document.getElementById('save-request-variables-btn');
        const requestVariablesStatus = document.getElementById('request-variables-status');
        const saveRequestModal = document.getElementById('save-request-modal');
        const saveRequestNameInput = document.getElementById('save-request-name-input');
        const saveRequestFolderSelect = document.getElementById('save-request-folder-select');
        const confirmSaveRequestBtn = document.getElementById('confirm-save-request-btn');
        const cancelSaveRequestBtn = document.getElementById('cancel-save-request-btn');
        const curlImportInput = document.getElementById('curl-import-input');
        const curlImportApplyBtn = document.getElementById('curl-import-apply-btn');
        const curlImportStatus = document.getElementById('curl-import-status');
        const REQUEST_DETAILS_HEIGHT_KEY = 'request-details-height';
        const RESPONSE_HEADERS_HEIGHT_KEY = 'request-response-headers-height';
        const REQUEST_HISTORY_KEY = 'request_run_history';
        const SAVED_REQUEST_FOLDERS_COLLAPSED_KEY = 'saved-request-folders-collapsed';
        const INSPECTOR_PENDING_PAYLOAD_KEY = 'inspector_pending_payload';
        const REQUEST_WORKSPACE_TABS_KEY = 'ogdevdesk_request_workspace_tabs';
        const REQUEST_WORKSPACE_PENDING_KEY = 'ogdevdesk_pending_request_workspace';
        const MAX_REQUEST_HISTORY_ENTRIES = 12;
        const activeRequestTabId = new URLSearchParams(window.location.search).get('tab') || '';
        let pendingPostmanCollection = null;
        let collapsedRequestFolders = readCollapsedRequestFolders();
        let latestCurlCommand = '';
        let latestResponseBody = '';
        let latestResponseHeaders = '';
        let latestResponseMeta = null;
        let variableSetDialogMode = 'create';
        let requestHistoryCache = [];
        let requestHistorySavePromise = Promise.resolve();

        function makeRequestWorkspaceTabId() {
            return `request-${Date.now()}-${Math.random().toString(16).slice(2)}`;
        }

        function requestWorkspaceStorageKey(tabId) {
            return `request_workspace_${tabId || 'default'}`;
        }

        function readRequestWorkspaceTabs() {
            try {
                const tabs = JSON.parse(localStorage.getItem(REQUEST_WORKSPACE_TABS_KEY) || '[]');
                return Array.isArray(tabs) ? tabs.filter((tab) => tab && typeof tab.id === 'string') : [];
            } catch (_) {
                return [];
            }
        }

        function writeRequestWorkspaceTabs(tabs) {
            localStorage.setItem(REQUEST_WORKSPACE_TABS_KEY, JSON.stringify(tabs));
        }

        function readRequestWorkspace(tabId) {
            if (
                typeof window.readRequestWorkspace === 'function'
                && window.readRequestWorkspace !== readRequestWorkspace
            ) {
                return window.readRequestWorkspace(tabId);
            }
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
            if (
                typeof window.createRequestWorkspaceTab === 'function'
                && window.createRequestWorkspaceTab !== createRequestWorkspaceTab
            ) {
                return window.createRequestWorkspaceTab(workspace);
            }

            const id = makeRequestWorkspaceTabId();
            const title = String(workspace.title || workspace.name || 'New Request').trim() || 'New Request';
            const tabs = readRequestWorkspaceTabs();
            const tab = { id, title, lastOpenedAt: Date.now() };
            tabs.push(tab);
            writeRequestWorkspaceTabs(tabs);
            writeRequestWorkspace(id, { ...workspace, title });
            return tab;
        }

        function updateRequestWorkspaceTab(tabId, workspace = {}) {
            if (!tabId) return;
            if (
                typeof window.updateRequestWorkspaceTab === 'function'
                && window.updateRequestWorkspaceTab !== updateRequestWorkspaceTab
            ) {
                window.updateRequestWorkspaceTab(tabId, workspace);
                return;
            }

            const tabs = readRequestWorkspaceTabs();
            const tab = tabs.find((candidate) => candidate.id === tabId);
            if (tab) {
                tab.title = String(workspace.title || workspace.name || tab.title || 'Request').trim() || 'Request';
                tab.lastOpenedAt = Date.now();
                writeRequestWorkspaceTabs(tabs);
            }
            writeRequestWorkspace(tabId, workspace);
        }

        function normalizeFolderPath(folder) {
            return String(folder || '')
                .replace(/\s+\/\s+/g, '/')
                .split('/')
                .map((part) => part.trim())
                .filter(Boolean)
                .join('/');
        }

        function isSameOrChildFolder(folder, parent) {
            return folder === parent || folder.startsWith(parent + '/');
        }

        function pathHasCollapsedFolder(folder, collapsedFolders, includeSelf = true) {
            const normalized = normalizeFolderPath(folder);
            if (!normalized) return false;
            const parts = normalized.split('/');
            const max = includeSelf ? parts.length : parts.length - 1;
            for (let index = 1; index <= max; index += 1) {
                if (collapsedFolders.has(parts.slice(0, index).join('/'))) {
                    return true;
                }
            }
            return false;
        }

        function writeRequestDragPayload(event, payload) {
            const raw = JSON.stringify(payload);
            event.dataTransfer.clearData();
            event.dataTransfer.setData('application/x-go-request-drag', raw);
            event.dataTransfer.setData('text/plain', raw);
            event.dataTransfer.effectAllowed = 'move';
        }

        function readRequestDragPayload(event) {
            const raw = event.dataTransfer.getData('application/x-go-request-drag')
                || event.dataTransfer.getData('text/plain')
                || '{}';
            return JSON.parse(raw);
        }

        function clearRequestDropTargets() {
            savedList.querySelectorAll('.dragging, .drop-target, .drop-target-invalid').forEach((element) => {
                element.classList.remove('dragging', 'drop-target', 'drop-target-invalid');
            });
            savedList.classList.remove('drop-target', 'drop-target-invalid');
        }

        async function postRequestMove(url, fields) {
            const body = new URLSearchParams();
            Object.entries(fields).forEach(([key, value]) => body.append(key, value || ''));
            const response = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' },
                body,
            });
            if (!response.ok) {
                throw new Error(await response.text());
            }
        }
        
        // Save Logic Elements
        const toggleSaveBtn = document.getElementById('toggle-save-btn');
        const saveControls = document.getElementById('save-controls');
        const saveMethod = document.getElementById('save-method');
        const saveUrl = document.getElementById('save-url');
        const saveHeaders = document.getElementById('save-headers');
        const saveBody = document.getElementById('save-body');
        const saveAuthType = document.getElementById('save-auth-type');
        const saveOAuthTokenUrl = document.getElementById('save-oauth-token-url');
        const saveOAuthClientId = document.getElementById('save-oauth-client-id');
        const saveOAuthClientSecret = document.getElementById('save-oauth-client-secret');
        const saveOAuthScope = document.getElementById('save-oauth-scope');
        const reqNameInput = document.getElementById('req-name');
        const reqFolderInput = document.getElementById('req-folder');

        // Auth Elements
        const authTypeSelect = document.getElementById('auth-type');
        const authInputs = document.getElementById('auth-inputs');
        let fetchedOAuthToken = ''; // Store the token here
        let currentRequestId = null;
        let currentAbortController = null;
        let requestVariables = readInitialRequestVariables();

        function readInitialRequestVariables() {
            const data = document.getElementById('request-variables-data');
            if (!data) return normalizeRequestVariables({});
            try {
                const parsed = JSON.parse(data.textContent || '{}');
                return normalizeRequestVariables(parsed);
            } catch (_) {
                return normalizeRequestVariables({});
            }
        }

        function readCollapsedRequestFolders() {
            try {
                const values = JSON.parse(localStorage.getItem(SAVED_REQUEST_FOLDERS_COLLAPSED_KEY) || '[]');
                return new Set(Array.isArray(values) ? values : []);
            } catch (_) {
                return new Set();
            }
        }

        function saveCollapsedRequestFolders() {
            localStorage.setItem(
                SAVED_REQUEST_FOLDERS_COLLAPSED_KEY,
                JSON.stringify(Array.from(collapsedRequestFolders))
            );
        }

        // --- Helper: Create Key-Value Row ---
        function addKvRow(containerId, key = '', val = '', isReadOnlyKey = false) {
            const container = document.getElementById(containerId);
            const row = document.createElement('div');
            row.className = 'kv-row';

            const keyInput = document.createElement('input');
            keyInput.type = 'text';
            keyInput.className = isReadOnlyKey ? 'kv-input key readonly' : 'kv-input key';
            keyInput.placeholder = 'Key';
            keyInput.value = key;
            keyInput.readOnly = isReadOnlyKey;
            keyInput.addEventListener('input', () => onKvChange(containerId));

            const valInput = document.createElement('input');
            valInput.type = 'text';
            valInput.className = 'kv-input val';
            valInput.placeholder = 'Value';
            valInput.value = val;
            valInput.addEventListener('input', () => onKvChange(containerId));

            row.appendChild(keyInput);
            row.appendChild(valInput);

            if (!isReadOnlyKey) {
                const removeBtn = document.createElement('button');
                removeBtn.type = 'button';
                removeBtn.className = 'kv-remove';
                removeBtn.textContent = 'x';
                removeBtn.addEventListener('click', () => {
                    row.remove();
                    onKvChange(containerId);
                });
                row.appendChild(removeBtn);
            }

            container.appendChild(row);
        }

        function getKvMap(containerId) {
            const container = document.getElementById(containerId);
            const map = {};
            container.querySelectorAll('.kv-row').forEach(row => {
                const k = row.querySelector('.key').value.trim();
                const v = row.querySelector('.val').value.trim();
                if(k) map[k] = v;
            });
            return map;
        }

        function getKvPairs(containerId) {
            const container = document.getElementById(containerId);
            const pairs = [];
            container.querySelectorAll('.kv-row').forEach(row => {
                const k = row.querySelector('.key').value.trim();
                const v = row.querySelector('.val').value.trim();
                if (k) pairs.push([k, v]);
            });
            return pairs;
        }

        function substituteRequestVariables(value) {
            const variables = getActiveVariableSet().values || {};
            return String(value || '').replace(/\{\{([^}]+)\}\}/g, (fullMatch, rawKey) => {
                const key = rawKey.trim();
                return Object.prototype.hasOwnProperty.call(variables, key) ? variables[key] : fullMatch;
            });
        }

        function getUnresolvedRequestVariables(value) {
            const names = new Set();
            String(value || '').replace(/\{\{([^}]+)\}\}/g, (_match, rawKey) => {
                const key = rawKey.trim();
                if (key) names.add(key);
                return _match;
            });
            return Array.from(names);
        }

        function normalizeRequestVariables(value) {
            const legacyGlobal = value.global && typeof value.global === 'object' ? value.global : {};
            let sets = Array.isArray(value.sets) ? value.sets : [];
            sets = sets
                .map((set) => ({
                    name: String(set.name || '').trim(),
                    values: set.values && typeof set.values === 'object' ? { ...set.values } : {},
                }))
                .filter((set) => set.name);

            if (sets.length === 0 && Object.keys(legacyGlobal).length > 0) {
                sets = [{ name: 'Default', values: { ...legacyGlobal } }];
            }

            const activeSet = String(value.active_set || '').trim();
            return {
                active_set: sets.some((set) => set.name === activeSet) ? activeSet : (sets[0]?.name || ''),
                sets,
                global: {},
            };
        }

        function getActiveVariableSet() {
            let active = requestVariables.sets.find((set) => set.name === requestVariables.active_set);
            if (!active && requestVariables.sets.length > 0) {
                requestVariables.active_set = requestVariables.sets[0].name;
                active = requestVariables.sets[0];
            }
            return active || { name: '', values: {} };
        }

        function updateRequestVariablesButtonLabel() {
            if (!requestVariablesBtn) return;
            const activeName = getActiveVariableSet().name;
            requestVariablesBtn.textContent = activeName ? `Vars: ${activeName}` : 'Variables';
            requestVariablesBtn.title = activeName ? `Request variables: ${activeName}` : 'Request variables';
        }

        function ensureVariableSet() {
            if (requestVariables.sets.length === 0) {
                requestVariables.sets.push({ name: 'Default', values: {} });
                requestVariables.active_set = 'Default';
            }
        }

        function renderVariableSetSelect() {
            if (!requestVariableSetSelect) return;
            requestVariableSetSelect.innerHTML = '';
            requestVariables.sets.forEach((set) => {
                const option = document.createElement('option');
                option.value = set.name;
                option.textContent = set.name;
                option.selected = set.name === requestVariables.active_set;
                requestVariableSetSelect.appendChild(option);
            });
        }

        function renderRequestVariables() {
            if (!requestVariablesContainer) return;
            ensureVariableSet();
            renderVariableSetSelect();
            updateRequestVariablesButtonLabel();
            requestVariablesContainer.innerHTML = '';
            const entries = Object.entries(getActiveVariableSet().values || {}).sort(([a], [b]) => a.localeCompare(b));
            if (entries.length === 0) {
                addRequestVariableRow('', '');
                return;
            }
            entries.forEach(([key, value]) => addRequestVariableRow(key, value));
        }

        function addRequestVariableRow(key = '', value = '') {
            if (!requestVariablesContainer) return;
            const row = document.createElement('div');
            row.className = 'request-var-row';

            const keyInput = document.createElement('input');
            keyInput.type = 'text';
            keyInput.className = 'request-var-key';
            keyInput.placeholder = 'Variable';
            keyInput.value = key;

            const valueInput = document.createElement('input');
            valueInput.type = 'text';
            valueInput.className = 'request-var-value';
            valueInput.placeholder = 'Value';
            valueInput.value = value;

            const removeBtn = document.createElement('button');
            removeBtn.type = 'button';
            removeBtn.className = 'kv-remove';
            removeBtn.textContent = 'x';
            removeBtn.addEventListener('click', () => row.remove());

            row.appendChild(keyInput);
            row.appendChild(valueInput);
            row.appendChild(removeBtn);
            requestVariablesContainer.appendChild(row);
        }

        function collectRequestVariables() {
            ensureVariableSet();
            const values = {};
            if (!requestVariablesContainer) return requestVariables;
            requestVariablesContainer.querySelectorAll('.request-var-row').forEach((row) => {
                const key = row.querySelector('.request-var-key')?.value.trim();
                const value = row.querySelector('.request-var-value')?.value || '';
                if (key) values[key] = value;
            });
            const active = getActiveVariableSet();
            active.values = values;
            return normalizeRequestVariables(requestVariables);
        }

        function addVariableSet(name) {
            const trimmed = String(name || '').trim();
            if (!trimmed) return;
            collectRequestVariables();
            const existing = requestVariables.sets.find((set) => set.name.toLowerCase() === trimmed.toLowerCase());
            if (existing) {
                requestVariables.active_set = existing.name;
            } else {
                requestVariables.sets.push({ name: trimmed, values: {} });
                requestVariables.active_set = trimmed;
            }
            if (newRequestVariableSetName) newRequestVariableSetName.value = '';
            renderRequestVariables();
        }

        function renameActiveVariableSet(name) {
            const trimmed = String(name || '').trim();
            if (!trimmed) return;
            collectRequestVariables();
            const active = getActiveVariableSet();
            if (!active.name) return;

            const duplicate = requestVariables.sets.find((set) => (
                set.name.toLowerCase() === trimmed.toLowerCase() && set.name !== active.name
            ));
            if (duplicate) {
                requestVariablesStatus.textContent = 'A set with that name already exists.';
                if (requestVariableSetModalDescription) {
                    requestVariableSetModalDescription.textContent = 'A set with that name already exists.';
                }
                return;
            }

            active.name = trimmed;
            requestVariables.active_set = trimmed;
            if (newRequestVariableSetName) newRequestVariableSetName.value = '';
            renderRequestVariables();
        }

        function copyActiveVariableSet(name) {
            const trimmed = String(name || '').trim();
            if (!trimmed) return;
            collectRequestVariables();
            const active = getActiveVariableSet();
            const copiedValues = { ...(active.values || {}) };
            const existing = requestVariables.sets.find((set) => set.name.toLowerCase() === trimmed.toLowerCase());
            if (existing) {
                requestVariablesStatus.textContent = 'A set with that name already exists.';
                if (requestVariableSetModalDescription) {
                    requestVariableSetModalDescription.textContent = 'A set with that name already exists.';
                }
                return;
            }

            requestVariables.sets.push({ name: trimmed, values: copiedValues });
            requestVariables.active_set = trimmed;
            if (newRequestVariableSetName) newRequestVariableSetName.value = '';
            renderRequestVariables();
        }

        function deleteActiveVariableSet() {
            collectRequestVariables();
            ensureVariableSet();
            const active = getActiveVariableSet();
            if (!active.name) return;

            if (requestVariables.sets.length <= 1) {
                if (!window.confirm(`Clear the only variable set "${active.name}"?`)) return;
                active.values = {};
                requestVariablesStatus.textContent = 'Cleared the only variable set.';
                renderRequestVariables();
                return;
            }

            const shouldDelete = window.confirm(`Delete variable set "${active.name}"?`);
            if (!shouldDelete) return;

            requestVariables.sets = requestVariables.sets.filter((set) => set.name !== active.name);
            requestVariables.active_set = requestVariables.sets[0]?.name || '';
            requestVariablesStatus.textContent = 'Variable set deleted. Save to keep this change.';
            renderRequestVariables();
        }

        function openVariableSetNameDialog(mode) {
            if (!newRequestVariableSetModal || !newRequestVariableSetName) return;
            collectRequestVariables();
            variableSetDialogMode = mode;
            requestVariablesStatus.textContent = '';

            const active = getActiveVariableSet();
            const isRename = mode === 'rename';
            const isCopy = mode === 'copy';
            requestVariableSetModalTitle.textContent = isRename
                ? 'Rename Variable Set'
                : (isCopy ? 'Copy Variable Set' : 'New Variable Set');
            requestVariableSetModalDescription.textContent = isRename
                ? 'Rename the active variable set.'
                : (isCopy ? 'Copy the active variable values into a new named set.' : 'Name the set, then add variables for that environment.');
            createRequestVariableSetBtn.textContent = isRename ? 'Rename' : (isCopy ? 'Copy' : 'Create');
            newRequestVariableSetName.value = isRename ? active.name : '';
            newRequestVariableSetModal.showModal();
            newRequestVariableSetName.focus();
            newRequestVariableSetName.select();
        }

        // --- Body Type Toggle ---
        function toggleBodyType() {
            const type = document.querySelector('input[name="body-type"]:checked').value;
            const rawContainer = document.getElementById('body-raw-container');
            const formContainer = document.getElementById('body-form-container');
            const formatJsonBtn = document.getElementById('format-json-btn');
            const isRaw = type === 'raw';
            if (rawContainer) rawContainer.hidden = !isRaw;
            if (formContainer) formContainer.hidden = isRaw;
            if (formatJsonBtn) formatJsonBtn.hidden = !isRaw;
            if (!isRaw) {
                if(document.getElementById('form-body-rows').children.length === 0) {
                     addKvRow('form-body-rows');
                }
            }
        }
        
        // --- Sync URL <-> Params ---
        function onKvChange(containerId) {
            if (containerId === 'params-container') {
                updateUrlFromParams();
            }
            saveActiveRequestWorkspace();
        }

        function updateUrlFromParams() {
            try {
                const parts = urlInput.value.split('?');
                const baseUrl = parts[0];
                const params = getKvPairs('params-container');
                const search = new URLSearchParams();
                params.forEach(([k, v]) => search.append(k, v));
                const queryString = search.toString();
                if (queryString) {
                    urlInput.value = `${baseUrl}?${queryString}`;
                } else {
                    urlInput.value = baseUrl;
                }
                detectPathVariables();
            } catch(e) {}
        }

        function parseUrlToParams() {
            try {
                let urlStr = urlInput.value;
                if (!urlStr.startsWith('http')) urlStr = 'http://placeholder.com' + (urlStr.startsWith('/') ? '' : '/') + urlStr;
                
                const urlObj = new URL(urlStr);
                const container = document.getElementById('params-container');
                container.innerHTML = ''; 
                urlObj.searchParams.forEach((val, key) => {
                    addKvRow('params-container', key, val);
                });
                addKvRow('params-container'); 
            } catch (e) {}
        }

        function detectPathVariables() {
            const url = urlInput.value;
            const regex = /(^|[^{])\{([^{}]+)\}(?!\})/g;
            let match;
            const foundKeys = new Set();
            while ((match = regex.exec(url)) !== null) { foundKeys.add(match[2]); }

            const container = document.getElementById('path-container');
            const currentValues = getKvMap('path-container');
            container.innerHTML = '';

            if (foundKeys.size === 0) {
                container.innerHTML = '<p style="padding:10px; color:#888;">No path variables detected.</p>';
                return;
            }

            foundKeys.forEach(key => {
                const val = currentValues[key] || '';
                addKvRow('path-container', key, val, true);
            });
        }
        
        urlInput.addEventListener('input', () => { parseUrlToParams(); detectPathVariables(); saveActiveRequestWorkspace(); });
        methodSelect.addEventListener('change', saveActiveRequestWorkspace);
        bodyInput.addEventListener('input', saveActiveRequestWorkspace);

        // --- Auth UI ---
        authTypeSelect.addEventListener('change', () => { renderAuthInputs(); saveActiveRequestWorkspace(); });
        authInputs.addEventListener('input', saveActiveRequestWorkspace);

        function renderAuthInputs(savedData = null) {
            const type = authTypeSelect.value;
            let html = '';
            // Helper to get value securely
            const val = (key) => savedData ? (savedData[key] || '') : '';

            // Pre-fill specific OAuth defaults for testing
            const defTokenUrl = val('oauth_token_url') || '';
            const defClientId = val('oauth_client_id') || '';
            const defClientSecret = val('oauth_client_secret') || '';
            const defScope = val('oauth_scope') || 'event_write';

            if (type === 'bearer') {
                html = `<div class="auth-row"><label>Token</label><input type="text" id="auth-bearer-token" placeholder="Bearer Token" value="${fetchedOAuthToken}"></div>`;
            } else if (type === 'basic') {
                html = `
                    <div class="auth-row"><label>Username</label><input type="text" id="auth-basic-user"></div>
                    <div class="auth-row"><label>Password</label><input type="password" id="auth-basic-pass"></div>
                `;
            } else if (type === 'apikey') {
                html = `
                    <div class="auth-row"><label>Key</label><input type="text" id="auth-api-key" placeholder="Key Name (e.g. X-API-Key)"></div>
                    <div class="auth-row"><label>Value</label><input type="text" id="auth-api-val" placeholder="Key Value"></div>
                    <div class="auth-row"><label>Add To</label><select id="auth-api-loc"><option value="header">Header</option></select></div>
                `;
            } else if (type === 'oauth2') {
                html = `
                    <div class="auth-row"><label>Token URL</label><input type="text" id="oauth-token-url" value="${defTokenUrl}"></div>
                    <div class="auth-row"><label>Client ID</label><input type="text" id="oauth-client-id" value="${defClientId}"></div>
                    <div class="auth-row"><label>Client Secret</label><input type="password" id="oauth-client-secret" value="${defClientSecret}"></div>
                    <div class="auth-row"><label>Scope</label><input type="text" id="oauth-scope" value="${defScope}"></div>
                    <div class="auth-row">
                        <button type="button" class="oauth-btn" onclick="fetchOAuthToken()">Get New Access Token</button>
                    </div>
                    <div class="auth-row" id="oauth-status-row" style="display:none; flex-direction:column; align-items:flex-start;">
                         <label>Current Token</label>
                         <div class="token-display" id="oauth-token-display"></div>
                    </div>
                `;
            }
            authInputs.innerHTML = html;
        }
        
        async function fetchOAuthToken() {
            const tokenUrl = document.getElementById('oauth-token-url').value;
            const clientId = document.getElementById('oauth-client-id').value;
            const clientSecret = document.getElementById('oauth-client-secret').value;
            const scope = document.getElementById('oauth-scope').value;
            const display = document.getElementById('oauth-token-display');
            const statusRow = document.getElementById('oauth-status-row');
            
            display.innerText = "Fetching...";
            statusRow.style.display = 'flex';
            
            // Construct JSON payload for this specific API structure
            const payload = {
                clientId: clientId,
                clientSecret: clientSecret,
                scopes: [scope], // Using array format as requested
                grant_type: 'client_credentials'
            };
            
             // Create CURL debug command string
            const debugCurl = `curl -X POST "${tokenUrl}" \\\n  -H "Content-Type: application/json" \\\n  -d '${JSON.stringify(payload)}'`;

            // Display debug info immediately
            display.innerHTML = `<div style="white-space: pre-wrap; margin: calc(var(--element-margin) / 2) var(--element-margin); color: #888; border-bottom: 1px solid #444; padding-bottom: 5px; font-size: var(--font-size-small); overflow-x: auto;">${debugCurl}</div><div id="token-status-msg">Fetching...</div>`;
            
            try {
                const resp = await fetch('/requests/run', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        method: 'POST',
                        url: tokenUrl,
                        headers: [{ key: 'Content-Type', value: 'application/json' }],
                        body: JSON.stringify(payload)
                    })
                });
                const run = await resp.json();
                const cleanJson = (run.body || '').trim();
                
                const msgDiv = document.getElementById('token-status-msg');

                try {
                    const data = JSON.parse(cleanJson);
                    if (data.access_token) {
                        fetchedOAuthToken = data.access_token;
                        // NEW: Show token in an input field with a copy button
                        msgDiv.innerHTML = `
                            <div style="color: #49cc90; margin: calc(var(--element-margin) / 2) var(--element-margin);">Token received!</div>
                            <div style="display:flex; gap:5px; width:100%; margin: calc(var(--element-margin) / 2) var(--element-margin);">
                                <input type="text" id="token-input-field" value="${fetchedOAuthToken}" readonly style="flex-grow:1; padding:5px; background:var(--primary-bg); color:var(--text-color); border:1px solid var(--border-color); border-radius:0;">
                                <button type="button" class="save-btn" onclick="copyToClipboard('token-input-field')" style="padding:5px 10px;">Copy</button>
                            </div>
                        `;
                    } else {
                        msgDiv.innerText = "Error: No access_token in response.";
                        console.error(data);
                    }
                } catch(e) {
                    msgDiv.innerText = "Error parsing response JSON.";
                    console.error("Parse error:", e);
                    console.log("Raw body:", run.body);
                }
                
            } catch (e) {
                 const msgDiv = document.getElementById('token-status-msg');
                 if(msgDiv) msgDiv.innerText = "Error: " + e.message;
            }
        }

        function copyToClipboard(elementId) {
            const copyText = document.getElementById(elementId);
            copyText.select();
            copyText.setSelectionRange(0, 99999);
            document.execCommand("copy");
        }

        function tokenizeCurlCommand(command) {
            const normalized = String(command || '').replace(/\\\r?\n/g, ' ');
            const tokens = [];
            let current = '';
            let quote = null;
            let escaped = false;

            for (const char of normalized) {
                if (escaped) {
                    current += char;
                    escaped = false;
                    continue;
                }
                if (char === '\\') {
                    escaped = true;
                    continue;
                }
                if (quote) {
                    if (char === quote) {
                        quote = null;
                    } else {
                        current += char;
                    }
                    continue;
                }
                if (char === '"' || char === "'") {
                    quote = char;
                    continue;
                }
                if (/\s/.test(char)) {
                    if (current) {
                        tokens.push(current);
                        current = '';
                    }
                    continue;
                }
                current += char;
            }

            if (current) tokens.push(current);
            return tokens;
        }

        function parseCurlHeader(rawHeader) {
            const index = String(rawHeader || '').indexOf(':');
            if (index < 0) return null;
            const key = rawHeader.slice(0, index).trim();
            const value = rawHeader.slice(index + 1).trim();
            return key ? [key, value] : null;
        }

        function parseCurlCommand(command) {
            const tokens = tokenizeCurlCommand(command);
            if (tokens.length === 0 || tokens[0] !== 'curl') {
                throw new Error('Paste a command that starts with curl.');
            }

            const parsed = { method: '', url: '', headers: [], bodyParts: [], basicAuth: null, useGetParams: false };
            const takesValue = new Set([
                '-X', '--request', '-H', '--header', '-d', '--data', '--data-raw', '--data-binary',
                '--data-ascii', '--data-urlencode', '-u', '--user', '--url',
            ]);
            const ignoredValueOptions = new Set(['--connect-timeout', '--max-time', '--proxy', '--resolve']);

            for (let index = 1; index < tokens.length; index += 1) {
                const token = tokens[index];
                const next = () => {
                    index += 1;
                    if (index >= tokens.length) throw new Error(`Missing value after ${token}.`);
                    return tokens[index];
                };

                if (token === '-X' || token === '--request') {
                    parsed.method = next().toUpperCase();
                    continue;
                }
                if (token.startsWith('--request=')) {
                    parsed.method = token.slice('--request='.length).toUpperCase();
                    continue;
                }
                if (token.startsWith('-X') && token.length > 2) {
                    parsed.method = token.slice(2).toUpperCase();
                    continue;
                }
                if (token === '-H' || token === '--header') {
                    const header = parseCurlHeader(next());
                    if (header) parsed.headers.push(header);
                    continue;
                }
                if (token.startsWith('--header=')) {
                    const header = parseCurlHeader(token.slice('--header='.length));
                    if (header) parsed.headers.push(header);
                    continue;
                }
                if (token === '-d' || token === '--data' || token === '--data-raw' || token === '--data-binary' || token === '--data-ascii' || token === '--data-urlencode') {
                    parsed.bodyParts.push(next());
                    continue;
                }
                if (token.startsWith('--data=') || token.startsWith('--data-raw=') || token.startsWith('--data-binary=') || token.startsWith('--data-ascii=') || token.startsWith('--data-urlencode=')) {
                    parsed.bodyParts.push(token.slice(token.indexOf('=') + 1));
                    continue;
                }
                if (token.startsWith('-d') && token.length > 2) {
                    parsed.bodyParts.push(token.slice(2));
                    continue;
                }
                if (token === '-u' || token === '--user') {
                    parsed.basicAuth = next();
                    continue;
                }
                if (token.startsWith('--user=')) {
                    parsed.basicAuth = token.slice('--user='.length);
                    continue;
                }
                if (token === '-G' || token === '--get') {
                    parsed.useGetParams = true;
                    continue;
                }
                if (token === '--url') {
                    parsed.url = next();
                    continue;
                }
                if (token.startsWith('--url=')) {
                    parsed.url = token.slice('--url='.length);
                    continue;
                }
                if (ignoredValueOptions.has(token)) {
                    next();
                    continue;
                }
                if (token.startsWith('http://') || token.startsWith('https://') || token.startsWith('/')) {
                    parsed.url = token;
                    continue;
                }
                if (token.startsWith('-') && takesValue.has(token)) {
                    next();
                }
            }

            if (!parsed.url) throw new Error('Could not find a URL in the curl command.');
            if (!parsed.method) parsed.method = parsed.bodyParts.length > 0 && !parsed.useGetParams ? 'POST' : 'GET';
            return parsed;
        }

        function populateFromCurlCommand(command) {
            const parsed = parseCurlCommand(command);
            const supportedMethods = Array.from(methodSelect.options).map((option) => option.value);
            methodSelect.value = supportedMethods.includes(parsed.method) ? parsed.method : 'GET';

            let nextUrl = parsed.url;
            const body = parsed.bodyParts.join('&');
            if (parsed.useGetParams && body) {
                nextUrl += (nextUrl.includes('?') ? '&' : '?') + body;
            }
            urlInput.value = nextUrl;

            document.querySelectorAll('.saved-req-item').forEach((item) => item.classList.remove('selected'));
            reqNameInput.value = '';
            if (reqFolderInput) reqFolderInput.value = '';

            const headersContainer = document.getElementById('headers-container');
            headersContainer.innerHTML = '';
            parsed.headers.forEach(([key, value]) => addKvRow('headers-container', key, value));
            addKvRow('headers-container');

            const rawBodyOption = document.querySelector('input[name="body-type"][value="raw"]');
            if (rawBodyOption) rawBodyOption.checked = true;
            toggleBodyType();
            bodyInput.value = parsed.useGetParams ? '' : body;

            if (parsed.basicAuth) {
                authTypeSelect.value = 'basic';
                renderAuthInputs();
                const splitAt = parsed.basicAuth.indexOf(':');
                const user = splitAt >= 0 ? parsed.basicAuth.slice(0, splitAt) : parsed.basicAuth;
                const pass = splitAt >= 0 ? parsed.basicAuth.slice(splitAt + 1) : '';
                document.getElementById('auth-basic-user').value = user;
                document.getElementById('auth-basic-pass').value = pass;
            } else {
                let importedAuthType = inferAuthTypeFromHeaders();
                authTypeSelect.value = importedAuthType;
                renderAuthInputs();
                applyAuthDefaultsFromHeaders(importedAuthType);
            }

            parseUrlToParams();
            detectPathVariables();
            return parsed;
        }

        function constructHeaders() {
            const headers = getKvPairs('headers-container');
            const authType = authTypeSelect.value;

            const setHeader = (name, value) => {
                const target = name.toLowerCase();
                for (let i = headers.length - 1; i >= 0; i -= 1) {
                    if ((headers[i][0] || '').toLowerCase() === target) {
                        headers.splice(i, 1);
                    }
                }
                headers.push([name, value]);
            };

            if (authType === 'bearer' || authType === 'oauth2') {
                const token = authType === 'oauth2' ? fetchedOAuthToken : document.getElementById('auth-bearer-token')?.value;
                if(token) setHeader('Authorization', `Bearer ${token}`);
            }
            if (authType === 'basic') {
                const user = document.getElementById('auth-basic-user')?.value || '';
                const pass = document.getElementById('auth-basic-pass')?.value || '';
                setHeader('Authorization', `Basic ${btoa(`${user}:${pass}`)}`);
            }
            if (authType === 'apikey') {
                const key = document.getElementById('auth-api-key')?.value?.trim() || '';
                const val = document.getElementById('auth-api-val')?.value || '';
                if (key) setHeader(key, val);
            }
            return headers;
        }

        function getRequestBodyAndHeaders(method, headerPairs) {
            if (method === 'GET' || method === 'HEAD') return { body: '', headerPairs };
            const bodyType = document.querySelector('input[name="body-type"]:checked')?.value || 'raw';
            if (bodyType === 'form') {
                const formPairs = getKvPairs('form-body-rows');
                const encoded = new URLSearchParams(formPairs).toString();
                const hasContentType = headerPairs.some(([k]) => k.toLowerCase() === 'content-type');
                if (!hasContentType) {
                    headerPairs.push(['Content-Type', 'application/x-www-form-urlencoded']);
                }
                return { body: encoded, headerPairs };
            }
            return { body: bodyInput.value, headerPairs };
        }

        function headerPairsToPayload(headerPairs) {
            return headerPairs.map(([key, value]) => ({ key, value }));
        }

        function getAuthSnapshot() {
            const type = authTypeSelect.value;
            return {
                type,
                bearer_token: document.getElementById('auth-bearer-token')?.value || fetchedOAuthToken || '',
                basic_user: document.getElementById('auth-basic-user')?.value || '',
                basic_pass: document.getElementById('auth-basic-pass')?.value || '',
                api_key: document.getElementById('auth-api-key')?.value || '',
                api_value: document.getElementById('auth-api-val')?.value || '',
                api_location: document.getElementById('auth-api-loc')?.value || 'header',
                oauth_token_url: document.getElementById('oauth-token-url')?.value || '',
                oauth_client_id: document.getElementById('oauth-client-id')?.value || '',
                oauth_client_secret: document.getElementById('oauth-client-secret')?.value || '',
                oauth_scope: document.getElementById('oauth-scope')?.value || '',
                fetched_oauth_token: fetchedOAuthToken || '',
            };
        }

        function applyAuthSnapshot(snapshot = {}) {
            const type = snapshot.type || 'none';
            authTypeSelect.value = type;
            fetchedOAuthToken = snapshot.fetched_oauth_token || snapshot.bearer_token || '';
            renderAuthInputs({
                oauth_token_url: snapshot.oauth_token_url,
                oauth_client_id: snapshot.oauth_client_id,
                oauth_client_secret: snapshot.oauth_client_secret,
                oauth_scope: snapshot.oauth_scope,
            });

            if (type === 'bearer') {
                const tokenInput = document.getElementById('auth-bearer-token');
                if (tokenInput) tokenInput.value = snapshot.bearer_token || '';
            } else if (type === 'basic') {
                const userInput = document.getElementById('auth-basic-user');
                const passInput = document.getElementById('auth-basic-pass');
                if (userInput) userInput.value = snapshot.basic_user || '';
                if (passInput) passInput.value = snapshot.basic_pass || '';
            } else if (type === 'apikey') {
                const keyInput = document.getElementById('auth-api-key');
                const valueInput = document.getElementById('auth-api-val');
                const locationInput = document.getElementById('auth-api-loc');
                if (keyInput) keyInput.value = snapshot.api_key || '';
                if (valueInput) valueInput.value = snapshot.api_value || '';
                if (locationInput) locationInput.value = snapshot.api_location || 'header';
            }
        }

        function getPathValuesSnapshot() {
            return getKvMap('path-container');
        }

        function applyPathValuesSnapshot(values = {}) {
            detectPathVariables();
            document.querySelectorAll('#path-container .kv-row').forEach((row) => {
                const key = row.querySelector('.key')?.value || '';
                const valueInput = row.querySelector('.val');
                if (valueInput && Object.prototype.hasOwnProperty.call(values, key)) {
                    valueInput.value = values[key];
                }
            });
        }

        function headersToString() {
            const h = constructHeaders();
            return h.map(([k, v]) => `${k}: ${v}`).join('\\n');
        }

        function requestWorkspaceFromLink(link) {
            return {
                title: link.dataset.name || 'Request',
                name: link.dataset.name || '',
                folder: link.dataset.folder || '',
                method: link.dataset.method || 'GET',
                url: link.dataset.url || '',
                headers: link.dataset.headers || '',
                body: link.dataset.body || '',
                authType: link.dataset.authType || 'none',
                oauthTokenUrl: link.dataset.oauthTokenUrl || '',
                oauthClientId: link.dataset.oauthClientId || '',
                oauthClientSecret: link.dataset.oauthClientSecret || '',
                oauthScope: link.dataset.oauthScope || '',
            };
        }

        function collectCurrentRequestWorkspace() {
            return {
                title: reqNameInput.value.trim() || 'New Request',
                name: reqNameInput.value.trim(),
                folder: reqFolderInput ? reqFolderInput.value || '' : '',
                method: methodSelect.value || 'GET',
                url: urlInput.value || '',
                headers: getKvPairs('headers-container').map(([key, value]) => `${key}: ${value}`).join('\n'),
                body: bodyInput.value || '',
                authType: authTypeSelect.value || 'none',
                oauthTokenUrl: document.getElementById('oauth-token-url')?.value || '',
                oauthClientId: document.getElementById('oauth-client-id')?.value || '',
                oauthClientSecret: document.getElementById('oauth-client-secret')?.value || '',
                oauthScope: document.getElementById('oauth-scope')?.value || '',
            };
        }

        function saveActiveRequestWorkspace() {
            if (!activeRequestTabId) return;
            updateRequestWorkspaceTab(activeRequestTabId, collectCurrentRequestWorkspace());
        }

        function loadRequestWorkspace(workspace = {}, selectSavedRow = true) {
            methodSelect.value = workspace.method || 'GET';
            urlInput.value = workspace.url || '';
            stringToHeadersTable(workspace.headers || '');
            bodyInput.value = workspace.body || '';
            reqNameInput.value = workspace.name || '';
            if (reqFolderInput) {
                reqFolderInput.value = workspace.folder || '';
            }

            if (selectSavedRow) {
                document.querySelectorAll('.saved-req-item').forEach(el => el.classList.remove('selected'));
                const selectedLink = findSavedRequestLink(workspace.name || '', workspace.folder || '');
                selectedLink?.closest('.saved-req-item')?.classList.add('selected');
            }

            let savedAuthType = workspace.authType || 'none';
            if (savedAuthType === 'none') {
                const inferred = inferAuthTypeFromHeaders();
                if (inferred !== 'none') savedAuthType = inferred;
            }
            authTypeSelect.value = savedAuthType;
            renderAuthInputs({
                oauth_token_url: workspace.oauthTokenUrl,
                oauth_client_id: workspace.oauthClientId,
                oauth_client_secret: workspace.oauthClientSecret,
                oauth_scope: workspace.oauthScope,
            });
            applyAuthDefaultsFromHeaders(savedAuthType);
            parseUrlToParams();
            detectPathVariables();
            saveActiveRequestWorkspace();
        }

        function openSavedRequestInNewTab(link) {
            if (!link) return;
            const workspace = requestWorkspaceFromLink(link);
            const tab = createRequestWorkspaceTab(workspace);
            if (tab) {
                sessionStorage.setItem(REQUEST_WORKSPACE_PENDING_KEY, JSON.stringify({
                    tabId: tab.id,
                    workspace,
                }));
                if (typeof window.renderRequestWorkspaceTabs === 'function') {
                    window.renderRequestWorkspaceTabs();
                }
                window.location.href = `/requests?tab=${encodeURIComponent(tab.id)}`;
            }
        }
        
        function stringToHeadersTable(headerStr) {
            const container = document.getElementById('headers-container');
            container.innerHTML = '';
            if (!headerStr) return;
            const lines = headerStr.split('\\n');
            lines.forEach(line => {
                const parts = line.split(':');
                if (parts.length >= 2) addKvRow('headers-container', parts[0].trim(), parts.slice(1).join(':').trim());
            });
            addKvRow('headers-container'); 
        }

        function getHeaderValueInsensitive(name) {
            const target = (name || '').toLowerCase();
            const pairs = getKvPairs('headers-container');
            for (const [k, v] of pairs) {
                if ((k || '').toLowerCase() === target) return v || '';
            }
            return '';
        }

        function inferAuthTypeFromHeaders() {
            const authHeader = getHeaderValueInsensitive('Authorization');
            if (!authHeader) return 'none';
            const lower = authHeader.toLowerCase();
            if (lower.startsWith('bearer ')) return 'bearer';
            if (lower.startsWith('basic ')) return 'basic';
            return 'apikey';
        }

        function applyAuthDefaultsFromHeaders(authType) {
            const authHeader = getHeaderValueInsensitive('Authorization');
            if (!authHeader) return;

            if (authType === 'bearer') {
                const token = authHeader.replace(/^Bearer\s+/i, '').trim();
                const tokenInput = document.getElementById('auth-bearer-token');
                if (tokenInput && token) tokenInput.value = token;
            }

            if (authType === 'basic') {
                const raw = authHeader.replace(/^Basic\s+/i, '').trim();
                try {
                    const decoded = atob(raw);
                    const idx = decoded.indexOf(':');
                    const user = idx >= 0 ? decoded.slice(0, idx) : decoded;
                    const pass = idx >= 0 ? decoded.slice(idx + 1) : '';
                    const userInput = document.getElementById('auth-basic-user');
                    const passInput = document.getElementById('auth-basic-pass');
                    if (userInput) userInput.value = user;
                    if (passInput) passInput.value = pass;
                } catch (_) {}
            }
        }

        function openTab(id) {
            document.querySelectorAll('.tab-content').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.tab').forEach(el => el.classList.remove('active'));
            document.getElementById(id).classList.add('active');
            const tabs = document.querySelectorAll('.tab');
            for(let t of tabs) {
                if(t.getAttribute('onclick').includes(id)) t.classList.add('active');
            }
        }
        
        // Init
        window.addKvRow = addKvRow; window.openTab = openTab; window.onKvChange = onKvChange; window.fetchOAuthToken = fetchOAuthToken; window.toggleBodyType = toggleBodyType; window.copyToClipboard = copyToClipboard;
        addKvRow('params-container'); addKvRow('headers-container'); parseUrlToParams(); detectPathVariables();
        renderRequestVariables();
        if (activeRequestTabId) {
            let workspace = readRequestWorkspace(activeRequestTabId);
            try {
                const pending = JSON.parse(sessionStorage.getItem(REQUEST_WORKSPACE_PENDING_KEY) || '{}');
                if (pending.tabId === activeRequestTabId && pending.workspace) {
                    workspace = pending.workspace;
                    writeRequestWorkspace(activeRequestTabId, workspace);
                    sessionStorage.removeItem(REQUEST_WORKSPACE_PENDING_KEY);
                }
            } catch (_) {
                sessionStorage.removeItem(REQUEST_WORKSPACE_PENDING_KEY);
            }
            if (workspace && Object.keys(workspace).length > 0) {
                loadRequestWorkspace(workspace);
            }
        }

        function filterSavedRequests() {
            if (!savedRequestSearch || !savedList) return;

            const filter = savedRequestSearch.value.toUpperCase();
            const visibleFolders = new Set();
            const requestItems = Array.from(savedList.querySelectorAll('.saved-req-item'));

            requestItems.forEach((item) => {
                const link = item.querySelector('.req-link');
                if (!link) return;

                const folder = normalizeFolderPath(item.dataset.folder || link.dataset.folder || '');
                const itemText = [
                    link.dataset.name || '',
                    link.dataset.method || '',
                    link.dataset.url || '',
                ].join(' ');
                const matchesFilter = itemText.toUpperCase().includes(filter);
                const isCollapsed = pathHasCollapsedFolder(folder, collapsedRequestFolders, true);
                const isVisible = matchesFilter && !isCollapsed;
                item.style.display = isVisible ? 'flex' : 'none';

                if (matchesFilter && folder) {
                    const parts = folder.split('/');
                    for (let index = 1; index <= parts.length; index += 1) {
                        visibleFolders.add(parts.slice(0, index).join('/'));
                    }
                }
            });

            Array.from(savedList.querySelectorAll('.saved-req-folder')).forEach((folder) => {
                const folderKey = normalizeFolderPath(folder.dataset.folder || '');
                const folderText = folder.textContent || '';
                const matchesFolder = folderKey.toUpperCase().includes(filter) || folderText.toUpperCase().includes(filter);
                const hiddenByAncestor = pathHasCollapsedFolder(folderKey, collapsedRequestFolders, false);
                const shouldShow = !hiddenByAncestor && (folderKey === '' || filter === '' || visibleFolders.has(folderKey) || matchesFolder);
                const isCollapsed = collapsedRequestFolders.has(folderKey);
                folder.style.display = shouldShow ? 'flex' : 'none';
                folder.classList.toggle('collapsed', isCollapsed);
                const toggle = folder.querySelector('.saved-req-folder-toggle');
                if (toggle) toggle.textContent = isCollapsed ? '▸' : '▾';
            });
        }

        function resetRequestBuilder() {
            methodSelect.value = 'GET';
            urlInput.value = '';
            bodyInput.value = '';
            fetchedOAuthToken = '';
            currentRequestId = null;
            currentAbortController = null;
            cancelBtn.disabled = true;
            reqNameInput.value = '';
            if (reqFolderInput) reqFolderInput.value = '';
            authTypeSelect.value = 'none';
            renderAuthInputs();
            const rawBodyOption = document.querySelector('input[name="body-type"][value="raw"]');
            if (rawBodyOption) rawBodyOption.checked = true;
            toggleBodyType();
            document.getElementById('params-container').innerHTML = '';
            document.getElementById('headers-container').innerHTML = '';
            document.getElementById('path-container').innerHTML = '';
            document.getElementById('form-body-rows').innerHTML = '';
            addKvRow('params-container');
            addKvRow('headers-container');
            parseUrlToParams();
            detectPathVariables();
            document.querySelectorAll('.saved-req-item').forEach(el => el.classList.remove('selected'));
            responseBody.innerText = 'Response body will appear here...';
            responseHeaders.innerText = 'Response headers will appear here...';
            resStatus.innerText = 'Status: -';
            resTime.innerText = 'Time: - ms';
            resSize.innerText = 'Size: -';
            requestDebugInfo.innerHTML = '';
            requestDebugInfo.style.display = 'none';
            latestCurlCommand = '';
            latestResponseBody = '';
            latestResponseHeaders = '';
            latestResponseMeta = null;
            if (viewCurlBtn) viewCurlBtn.disabled = true;
            if (openInspectorBtn) openInspectorBtn.disabled = true;
            urlInput.focus();
        }

        function loadRequestHistory() {
            return requestHistoryCache;
        }

        async function loadRequestHistoryFromServer() {
            try {
                const resp = await fetch('/requests/history');
                if (!resp.ok) throw new Error(await resp.text());
                const serverHistory = normalizeRequestHistoryList(await resp.json());
                const localHistory = normalizeRequestHistoryList(JSON.parse(localStorage.getItem(REQUEST_HISTORY_KEY) || '[]'));
                const merged = new Map();
                [...localHistory, ...serverHistory]
                    .sort((left, right) => new Date(right.createdAt || 0) - new Date(left.createdAt || 0))
                    .forEach((entry) => {
                        if (entry.id && !merged.has(entry.id)) merged.set(entry.id, entry);
                    });
                requestHistoryCache = pruneRequestHistory(Array.from(merged.values()));
                if (localHistory.length > serverHistory.length || requestHistoryCache.some((entry) => !serverHistory.some((serverEntry) => serverEntry.id === entry.id))) {
                    saveRequestHistory(requestHistoryCache);
                }
            } catch (err) {
                try {
                    requestHistoryCache = normalizeRequestHistoryList(JSON.parse(localStorage.getItem(REQUEST_HISTORY_KEY) || '[]'));
                } catch (_) {
                    requestHistoryCache = [];
                }
                console.error('Failed to load request history from app database', err);
            }
            renderRequestHistoryOptions();
        }

        function normalizeRequestHistoryList(history) {
            if (!Array.isArray(history)) return [];
            return history.map((entry, index) => ({
                    ...entry,
                    id: entry.id || `${entry.createdAt || 'history'}-${index}`,
            }));
        }

        function saveRequestHistory(history) {
            requestHistoryCache = normalizeRequestHistoryList(history);
            try {
                localStorage.setItem(REQUEST_HISTORY_KEY, JSON.stringify(requestHistoryCache));
            } catch (err) {
                console.error('Failed to save request history fallback', err);
            }

            requestHistorySavePromise = fetch('/requests/history', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(requestHistoryCache),
            }).catch((err) => {
                console.error('Failed to save request history to app database', err);
                try {
                    localStorage.setItem(REQUEST_HISTORY_KEY, JSON.stringify(requestHistoryCache.slice(0, 3)));
                } catch (retryErr) {
                    console.error('Failed to save reduced request history fallback', retryErr);
                }
            });
            return requestHistorySavePromise;
        }

        function requestHistoryLabel(entry) {
            const date = new Date(entry.createdAt);
            const time = Number.isNaN(date.getTime()) ? '' : date.toLocaleString();
            const status = entry.response?.status ? ` - ${entry.response.status}` : '';
            const method = entry.request?.method || 'GET';
            const url = entry.request?.url || entry.request?.finalUrl || 'Request';
            return `${time} - ${method} ${url}${status}`;
        }

        function renderRequestHistoryOptions(selectedId = '') {
            if (!requestHistorySelect) return;

            const nextSelectedId = selectedId || requestHistorySelect.value;
            const history = loadRequestHistory();
            requestHistorySelect.innerHTML = '<option value="">Request history</option>';
            history.forEach((entry) => {
                const option = document.createElement('option');
                option.value = entry.id;
                option.textContent = requestHistoryLabel(entry);
                requestHistorySelect.appendChild(option);
            });
            if (nextSelectedId && history.some((entry) => entry.id === nextSelectedId)) {
                requestHistorySelect.value = nextSelectedId;
            }
        }

        function pruneRequestHistory(history) {
            return history.slice(0, MAX_REQUEST_HISTORY_ENTRIES);
        }

        function getExplicitHeadersString() {
            return getKvPairs('headers-container').map(([k, v]) => `${k}: ${v}`).join('\n');
        }

        function buildRequestHistoryEntry(requestDetails, responseDetails) {
            const response = normalizeRequestHistoryResponse(responseDetails);
            return {
                id: String(Date.now()) + '-' + Math.random().toString(16).slice(2),
                createdAt: new Date().toISOString(),
                request: {
                    name: reqNameInput.value.trim(),
                    folder: reqFolderInput ? reqFolderInput.value : '',
                    method: methodSelect.value,
                    url: urlInput.value,
                    finalUrl: requestDetails.finalUrl,
                    headers: getExplicitHeadersString(),
                    body: bodyInput.value,
                    pathValues: getPathValuesSnapshot(),
                    auth: getAuthSnapshot(),
                    activeVariableSet: requestVariables.active_set || '',
                },
                response,
            };
        }

        function normalizeRequestHistoryResponse(responseDetails) {
            const response = { ...responseDetails };
            response.body = String(response.body || '');
            response.displayBody = String(response.displayBody || response.body || '');
            response.truncated = false;

            if (response.displayBody === response.body) {
                delete response.displayBody;
            }

            return response;
        }

        async function cacheRequestHistory(requestDetails, responseDetails) {
            const entry = buildRequestHistoryEntry(requestDetails, responseDetails);
            const history = loadRequestHistory().filter((existing) => {
                return existing.request?.method !== entry.request.method
                    || existing.request?.url !== entry.request.url
                    || existing.request?.body !== entry.request.body
                    || existing.response?.body !== entry.response.body;
            });
            history.unshift(entry);
            await saveRequestHistory(pruneRequestHistory(history));
            renderRequestHistoryOptions(entry.id);
        }

        function restoreRequestHistoryEntry() {
            if (!requestHistorySelect || requestHistorySelect.value === '') return;
            const entry = loadRequestHistory().find((item) => item.id === requestHistorySelect.value);
            if (!entry) return;

            const request = entry.request || {};
            const response = entry.response || {};

            methodSelect.value = request.method || 'GET';
            urlInput.value = request.url || request.finalUrl || '';
            bodyInput.value = request.body || '';
            reqNameInput.value = request.name || '';
            if (reqFolderInput) reqFolderInput.value = request.folder || '';
            stringToHeadersTable(request.headers || '');
            applyAuthSnapshot(request.auth || {});
            if (request.activeVariableSet && requestVariables.sets.some((set) => set.name === request.activeVariableSet)) {
                requestVariables.active_set = request.activeVariableSet;
                updateRequestVariablesButtonLabel();
            }
            parseUrlToParams();
            applyPathValuesSnapshot(request.pathValues || {});

            responseHeaders.innerText = response.headers || '(no headers)';
            responseBody.innerText = response.displayBody || response.body || '';
            resStatus.innerText = response.status ? `Status: ${response.status}` : 'Status: -';
            resStatus.className = response.statusClass || 'status-badge';
            resTime.innerText = response.duration_ms ? `Time: ${response.duration_ms} ms` : 'Time: - ms';
            resSize.innerText = response.size_kb ? `Size: ${response.size_kb} KB` : 'Size: -';
            latestCurlCommand = response.curl || '';
            latestResponseBody = response.body || response.displayBody || '';
            latestResponseHeaders = response.headers || '';
            latestResponseMeta = response.meta || null;
            if (viewCurlBtn) viewCurlBtn.disabled = !latestCurlCommand;
            if (openInspectorBtn) openInspectorBtn.disabled = !latestResponseBody;
        }

        loadRequestHistoryFromServer();

        if (requestHistorySelect) {
            requestHistorySelect.addEventListener('input', restoreRequestHistoryEntry);
            requestHistorySelect.addEventListener('change', restoreRequestHistoryEntry);
        }

        if (deleteRequestHistoryBtn) {
            deleteRequestHistoryBtn.addEventListener('click', () => {
                if (!requestHistorySelect || requestHistorySelect.value === '') return;
                if (!window.confirm('Delete the selected request history entry?')) return;

                const nextHistory = loadRequestHistory().filter((entry) => entry.id !== requestHistorySelect.value);
                saveRequestHistory(nextHistory);
                renderRequestHistoryOptions();
            });
        }

        if (clearRequestHistoryBtn) {
            clearRequestHistoryBtn.addEventListener('click', () => {
                if (!window.confirm('Clear all cached request history?')) return;

                saveRequestHistory([]);
                renderRequestHistoryOptions();
            });
        }

        if (savedRequestSearch) {
            savedRequestSearch.addEventListener('input', filterSavedRequests);
            filterSavedRequests();
        }

        if (newRequestBtn) {
            newRequestBtn.addEventListener('click', () => {
                const tab = createRequestWorkspaceTab({ title: 'New Request', method: 'GET', authType: 'none' });
                if (tab) {
                    if (typeof window.renderRequestWorkspaceTabs === 'function') {
                        window.renderRequestWorkspaceTabs();
                    }
                    window.location.href = `/requests?tab=${encodeURIComponent(tab.id)}`;
                    return;
                }
                resetRequestBuilder();
            });
        }

        if (createRequestFolderBtn && createRequestFolderForm && newRequestFolderName) {
            createRequestFolderBtn.addEventListener('click', () => {
                const folderName = window.prompt('Folder name');
                if (!folderName || folderName.trim() === '') return;

                newRequestFolderName.value = normalizeFolderPath(folderName);
                createRequestFolderForm.submit();
            });
        }

        if (savedList) {
            savedList.addEventListener('dragstart', (event) => {
                const folder = event.target.closest('.saved-req-folder[data-folder]');
                const item = event.target.closest('.saved-req-item');

                if (folder && folder.dataset.folder) {
                    writeRequestDragPayload(event, {
                        type: 'folder',
                        folder: normalizeFolderPath(folder.dataset.folder),
                    });
                    folder.classList.add('dragging');
                    return;
                }

                if (item) {
                    writeRequestDragPayload(event, {
                        type: 'request',
                        name: item.dataset.name || '',
                        folder: normalizeFolderPath(item.dataset.folder || ''),
                    });
                    item.classList.add('dragging');
                }
            });

            savedList.addEventListener('dragend', () => {
                clearRequestDropTargets();
            });

            savedList.addEventListener('dragover', (event) => {
                event.preventDefault();
                event.dataTransfer.dropEffect = 'move';
                savedList.querySelectorAll('.drop-target, .drop-target-invalid').forEach((element) => {
                    element.classList.remove('drop-target', 'drop-target-invalid');
                });
                savedList.classList.remove('drop-target', 'drop-target-invalid');

                const folder = event.target.closest('.saved-req-folder[data-folder]');
                let payload = {};
                try {
                    payload = readRequestDragPayload(event);
                } catch {
                    payload = {};
                }

                if (folder) {
                    const targetFolder = normalizeFolderPath(folder.dataset.folder || '');
                    const draggedFolder = normalizeFolderPath(payload.folder || '');
                    const invalid = payload.type === 'folder' && (!draggedFolder || draggedFolder === targetFolder || isSameOrChildFolder(targetFolder, draggedFolder));
                    folder.classList.add(invalid ? 'drop-target-invalid' : 'drop-target');
                    event.dataTransfer.dropEffect = invalid ? 'none' : 'move';
                } else {
                    savedList.classList.add('drop-target');
                }
            });

            savedList.addEventListener('dragleave', (event) => {
                const target = event.target.closest('.saved-req-folder');
                if (target) target.classList.remove('drop-target', 'drop-target-invalid');
                if (!savedList.contains(event.relatedTarget)) {
                    savedList.classList.remove('drop-target', 'drop-target-invalid');
                }
            });

            savedList.addEventListener('drop', async (event) => {
                event.preventDefault();

                let payload;
                try {
                    payload = readRequestDragPayload(event);
                } catch {
                    return;
                } finally {
                    clearRequestDropTargets();
                }

                const targetFolder = normalizeFolderPath(event.target.closest('.saved-req-folder[data-folder]')?.dataset.folder || '');
                try {
                    if (payload.type === 'request') {
                        await postRequestMove('/requests/move', {
                            name: payload.name,
                            folder: payload.folder,
                            new_folder: targetFolder,
                        });
                        window.location.reload();
                    } else if (payload.type === 'folder') {
                        const draggedFolder = normalizeFolderPath(payload.folder);
                        if (!draggedFolder || draggedFolder === targetFolder || isSameOrChildFolder(targetFolder, draggedFolder)) return;
                        await postRequestMove('/requests/folder/move', {
                            folder_name: draggedFolder,
                            new_parent: targetFolder,
                        });
                        window.location.reload();
                    }
                } catch (error) {
                    window.alert(`Move failed: ${error.message}`);
                }
            });
        }

        if (requestVariablesBtn && requestVariablesModal) {
            requestVariablesBtn.addEventListener('click', () => {
                renderRequestVariables();
                requestVariablesStatus.textContent = '';
                requestVariablesModal.showModal();
            });
        }

        if (closeRequestVariablesBtn && requestVariablesModal) {
            closeRequestVariablesBtn.addEventListener('click', () => {
                renderRequestVariables();
                requestVariablesModal.close();
            });
        }

        if (requestVariableSetSelect) {
            requestVariableSetSelect.addEventListener('change', () => {
                collectRequestVariables();
                requestVariables.active_set = requestVariableSetSelect.value;
                renderRequestVariables();
                requestVariablesStatus.textContent = '';
            });
        }

        function createNamedVariableSet() {
            const setName = newRequestVariableSetName?.value || '';
            if (variableSetDialogMode === 'rename') {
                renameActiveVariableSet(setName);
            } else if (variableSetDialogMode === 'copy') {
                copyActiveVariableSet(setName);
            } else {
                addVariableSet(setName);
            }

            if (!requestVariablesStatus.textContent) {
                requestVariablesStatus.textContent = '';
                if (newRequestVariableSetName) newRequestVariableSetName.value = '';
                if (newRequestVariableSetModal?.open) newRequestVariableSetModal.close();
            }
        }

        if (addRequestVariableSetBtn) {
            addRequestVariableSetBtn.addEventListener('click', () => openVariableSetNameDialog('create'));
        }

        if (renameRequestVariableSetBtn) {
            renameRequestVariableSetBtn.addEventListener('click', () => openVariableSetNameDialog('rename'));
        }

        if (copyRequestVariableSetBtn) {
            copyRequestVariableSetBtn.addEventListener('click', () => openVariableSetNameDialog('copy'));
        }

        if (deleteRequestVariableSetBtn) {
            deleteRequestVariableSetBtn.addEventListener('click', deleteActiveVariableSet);
        }

        if (cancelRequestVariableSetBtn && newRequestVariableSetModal) {
            cancelRequestVariableSetBtn.addEventListener('click', () => {
                if (newRequestVariableSetName) newRequestVariableSetName.value = '';
                newRequestVariableSetModal.close();
            });
        }

        if (createRequestVariableSetBtn) {
            createRequestVariableSetBtn.addEventListener('click', createNamedVariableSet);
        }

        if (newRequestVariableSetName) {
            newRequestVariableSetName.addEventListener('keydown', (event) => {
                if (event.key === 'Enter') {
                    event.preventDefault();
                    createNamedVariableSet();
                }
            });
        }

        if (addRequestVariableBtn) {
            addRequestVariableBtn.addEventListener('click', () => addRequestVariableRow('', ''));
        }

        if (saveRequestVariablesBtn) {
            saveRequestVariablesBtn.addEventListener('click', async () => {
                requestVariables = collectRequestVariables();
                requestVariablesStatus.textContent = 'Saving...';
                try {
                    const resp = await fetch('/requests/variables', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(requestVariables),
                    });
                    if (!resp.ok) throw new Error(await resp.text());
                    requestVariables = normalizeRequestVariables(await resp.json());
                    renderRequestVariables();
                    requestVariablesStatus.textContent = 'Saved';
                    window.setTimeout(() => {
                        if (requestVariablesModal?.open) requestVariablesModal.close();
                    }, 300);
                } catch (err) {
                    requestVariablesStatus.textContent = 'Save failed: ' + err.message;
                }
            });
        }

        function countPostmanItems(items, folderDepth = 0, result = { requests: 0, folders: 0 }) {
            if (!Array.isArray(items)) return result;
            items.forEach((item) => {
                if (Array.isArray(item.item)) {
                    result.folders += 1;
                    countPostmanItems(item.item, folderDepth + 1, result);
                } else if (item.request) {
                    result.requests += 1;
                }
            });
            return result;
        }

        function renderPostmanPreview(collection) {
            const counts = countPostmanItems(collection.item || []);
            const variables = Array.isArray(collection.variable) ? collection.variable.length : 0;
            const collectionName = collection.info?.name || 'Unnamed collection';
            postmanImportPreview.textContent = '';
            const title = document.createElement('strong');
            title.textContent = collectionName;
            const summary = document.createElement('div');
            summary.textContent = `${counts.requests} requests, ${counts.folders} folders, ${variables} variables found.`;
            postmanImportPreview.appendChild(title);
            postmanImportPreview.appendChild(summary);
        }

        if (importPostmanBtn && postmanImportModal) {
            importPostmanBtn.addEventListener('click', () => {
                pendingPostmanCollection = null;
                confirmPostmanImportBtn.disabled = true;
                postmanImportPreview.textContent = 'Choose a Postman collection export to preview it.';
                postmanImportFile.value = '';
                postmanImportModal.showModal();
            });
        }

        if (closePostmanImportBtn && postmanImportModal) {
            closePostmanImportBtn.addEventListener('click', () => postmanImportModal.close());
        }

        if (postmanImportFile) {
            postmanImportFile.addEventListener('change', async () => {
                pendingPostmanCollection = null;
                confirmPostmanImportBtn.disabled = true;
                const file = postmanImportFile.files?.[0];
                if (!file) return;
                try {
                    const text = await file.text();
                    const collection = JSON.parse(text);
                    if (!Array.isArray(collection.item)) {
                        throw new Error('This does not look like a Postman collection export.');
                    }
                    pendingPostmanCollection = collection;
                    renderPostmanPreview(collection);
                    confirmPostmanImportBtn.disabled = false;
                } catch (err) {
                    postmanImportPreview.textContent = 'Could not read collection: ' + err.message;
                }
            });
        }

        if (confirmPostmanImportBtn) {
            confirmPostmanImportBtn.addEventListener('click', async () => {
                if (!pendingPostmanCollection) return;
                confirmPostmanImportBtn.disabled = true;
                postmanImportPreview.textContent = 'Importing...';
                try {
                    const resp = await fetch('/requests/import/postman', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({
                            collection: pendingPostmanCollection,
                            duplicate_mode: postmanDuplicateMode?.value || 'rename',
                        }),
                    });
                    if (!resp.ok) throw new Error(await resp.text());
                    const result = await resp.json();
                    const warningText = result.warnings?.length ? ` ${result.warnings.length} warnings.` : '';
                    postmanImportPreview.textContent = `Imported ${result.imported} requests and ${result.variables} variables.${warningText}`;
                    window.setTimeout(() => window.location.reload(), 700);
                } catch (err) {
                    confirmPostmanImportBtn.disabled = false;
                    postmanImportPreview.textContent = 'Import failed: ' + err.message;
                }
            });
        }

        if (viewCurlBtn && curlViewModal && curlViewOutput) {
            viewCurlBtn.addEventListener('click', () => {
                curlViewOutput.textContent = latestCurlCommand || 'Run a request to generate curl.';
                curlViewModal.showModal();
            });
        }

        if (closeCurlViewBtn && curlViewModal) {
            closeCurlViewBtn.addEventListener('click', () => curlViewModal.close());
        }

        if (curlImportApplyBtn && curlImportInput) {
            curlImportApplyBtn.addEventListener('click', () => {
                try {
                    const parsed = populateFromCurlCommand(curlImportInput.value);
                    if (curlImportStatus) curlImportStatus.textContent = `Imported ${parsed.method} ${parsed.url}`;
                } catch (error) {
                    if (curlImportStatus) curlImportStatus.textContent = error.message;
                }
            });
        }

        if (openInspectorBtn) {
            openInspectorBtn.addEventListener('click', async () => {
                const payload = {
                    source: 'requests',
                    body: latestResponseBody || responseBody.innerText || '',
                    headers: latestResponseHeaders || responseHeaders.innerText || '',
                    meta: latestResponseMeta || {},
                    captured_at: new Date().toISOString(),
                };
                sessionStorage.setItem(INSPECTOR_PENDING_PAYLOAD_KEY, JSON.stringify(payload));
                try {
                    await requestHistorySavePromise;
                } catch (_) {}
                window.location.href = '/inspector';
            });
        }

        function populateSaveFormFields() {
            saveMethod.value = methodSelect.value;
            saveUrl.value = urlInput.value;
            const explicitHeaders = getKvPairs('headers-container');
            const reqPayload = getRequestBodyAndHeaders(
                methodSelect.value,
                explicitHeaders.map(([k, v]) => [k, v])
            );
            saveHeaders.value = explicitHeaders.map(([k, v]) => `${k}: ${v}`).join('\n');
            saveBody.value = reqPayload.body;
            saveAuthType.value = authTypeSelect.value;

            saveOAuthTokenUrl.value = '';
            saveOAuthClientId.value = '';
            saveOAuthClientSecret.value = '';
            saveOAuthScope.value = '';
            
            if (authTypeSelect.value === 'oauth2') {
                saveOAuthTokenUrl.value = document.getElementById('oauth-token-url')?.value || '';
                saveOAuthClientId.value = document.getElementById('oauth-client-id')?.value || '';
                saveOAuthClientSecret.value = document.getElementById('oauth-client-secret')?.value || '';
                saveOAuthScope.value = document.getElementById('oauth-scope')?.value || '';
            }
        }

        function syncSavedRequestDomFromSaveForm() {
            const name = reqNameInput.value || '';
            const folder = reqFolderInput ? reqFolderInput.value || '' : '';
            const link = findSavedRequestLink(name, folder);

            if (!link) return;

            link.dataset.method = saveMethod.value;
            link.dataset.url = saveUrl.value;
            link.dataset.headers = saveHeaders.value;
            link.dataset.body = saveBody.value;
            link.dataset.authType = saveAuthType.value || 'none';
            link.dataset.oauthTokenUrl = saveOAuthTokenUrl.value;
            link.dataset.oauthClientId = saveOAuthClientId.value;
            link.dataset.oauthClientSecret = saveOAuthClientSecret.value;
            link.dataset.oauthScope = saveOAuthScope.value;
            link.dataset.name = name;
            link.dataset.folder = folder;
            link.textContent = name;

            const item = link.closest('.saved-req-item');
            if (item) {
                item.dataset.name = name;
                item.dataset.folder = folder;
                const methodBadge = item.querySelector('.req-method');
                if (methodBadge) {
                    methodBadge.textContent = saveMethod.value;
                    methodBadge.className = `req-method ${saveMethod.value.toLowerCase()}`;
                }
                item.classList.add('selected');
            }
        }

        function findSavedRequestLink(name, folder) {
            return Array.from(document.querySelectorAll('.req-link')).find((candidate) => (
                candidate.dataset.name === name && (candidate.dataset.folder || '') === (folder || '')
            ));
        }

        async function refreshSavedRequestsList(selectedName = '', selectedFolder = '') {
            const response = await fetch('/requests', { cache: 'no-store' });
            if (!response.ok) throw new Error(await response.text());

            const html = await response.text();
            const doc = new DOMParser().parseFromString(html, 'text/html');
            const freshList = doc.getElementById('saved-list');
            if (!freshList) throw new Error('Saved request list was missing from the refreshed page.');

            savedList.innerHTML = freshList.innerHTML;
            filterSavedRequests();

            const selectedLink = findSavedRequestLink(selectedName, selectedFolder);
            if (selectedLink) {
                selectedLink.closest('.saved-req-item')?.classList.add('selected');
            }
        }

        function openSaveRequestModal() {
            if (!saveRequestModal) return;
            saveRequestNameInput.value = reqNameInput.value || '';
            saveRequestFolderSelect.value = reqFolderInput ? reqFolderInput.value : '';
            saveRequestModal.showModal();
            saveRequestNameInput.focus();
        }

        function closeSaveRequestModal() {
            if (saveRequestModal?.open) {
                saveRequestModal.close();
            }
        }

        toggleSaveBtn.addEventListener('click', openSaveRequestModal);

        if (cancelSaveRequestBtn) {
            cancelSaveRequestBtn.addEventListener('click', closeSaveRequestModal);
        }

        if (saveRequestModal) {
            saveRequestModal.addEventListener('click', (event) => {
                if (event.target === saveRequestModal) {
                    closeSaveRequestModal();
                }
            });
        }

        if (confirmSaveRequestBtn) {
            confirmSaveRequestBtn.addEventListener('click', async () => {
                const name = saveRequestNameInput.value.trim();
                if (!name) {
                    saveRequestNameInput.focus();
                    return;
                }

                reqNameInput.value = name;
                if (reqFolderInput) {
                    reqFolderInput.value = saveRequestFolderSelect.value || '';
                }
                populateSaveFormFields();

                const originalText = confirmSaveRequestBtn.textContent;
                confirmSaveRequestBtn.disabled = true;
                confirmSaveRequestBtn.textContent = 'Saving...';
                try {
                    const payload = new URLSearchParams(new FormData(saveControls));
                    const response = await fetch(saveControls.action, {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/x-www-form-urlencoded;charset=UTF-8' },
                        body: payload,
                    });
                    if (!response.ok) throw new Error(await response.text());

                    try {
                        await refreshSavedRequestsList(reqNameInput.value || '', reqFolderInput ? reqFolderInput.value || '' : '');
                    } catch (refreshError) {
                        syncSavedRequestDomFromSaveForm();
                    }
                    saveActiveRequestWorkspace();
                    confirmSaveRequestBtn.textContent = 'Saved';
                    window.setTimeout(() => {
                        confirmSaveRequestBtn.textContent = originalText;
                        confirmSaveRequestBtn.disabled = false;
                    }, 700);
                    closeSaveRequestModal();
                } catch (error) {
                    confirmSaveRequestBtn.disabled = false;
                    confirmSaveRequestBtn.textContent = originalText;
                    window.alert(`Save failed: ${error.message}`);
                }
            });
        }

        savedList.addEventListener('click', (e) => {
            const openTabButton = e.target.closest('.saved-req-open-tab');
            if (openTabButton) {
                e.preventDefault();
                e.stopPropagation();
                const link = openTabButton.closest('.saved-req-item')?.querySelector('.req-link');
                openSavedRequestInNewTab(link);
                return;
            }

            const renameButton = e.target.closest('.rename-saved-request-btn');
            if (renameButton) {
                e.preventDefault();
                const form = renameButton.closest('form');
                const currentName = form?.querySelector('input[name="name"]')?.value || '';
                const nextName = window.prompt('Rename saved request', currentName);
                if (!nextName || nextName.trim() === '' || nextName.trim() === currentName) return;

                form.querySelector('input[name="new_name"]').value = nextName.trim();
                form.submit();
                return;
            }

            if (e.target.closest('.delete-request-folder-form')) {
                return;
            }

            const folder = e.target.closest('.saved-req-folder');
            if (folder) {
                const folderKey = folder.dataset.folder || '';
                if (collapsedRequestFolders.has(folderKey)) {
                    collapsedRequestFolders.delete(folderKey);
                } else {
                    collapsedRequestFolders.add(folderKey);
                }
                saveCollapsedRequestFolders();
                filterSavedRequests();
                return;
            }

            const requestItem = e.target.closest('.saved-req-item');
            const link = e.target.closest('.req-link') || requestItem?.querySelector('.req-link');
            if (link) {
                e.preventDefault();
                loadRequestWorkspace(requestWorkspaceFromLink(link));
            }
        });

        document.getElementById('format-json-btn').addEventListener('click', () => {
            try { bodyInput.value = JSON.stringify(JSON.parse(bodyInput.value), null, 4); } catch(e) { alert('Invalid JSON'); }
        });

        async function sendRequest() {
            if (sendBtn.disabled) return;
            responseBody.innerText = 'Loading...';
            responseHeaders.innerText = 'Loading headers...';
            resStatus.innerText = 'Status: -';
            resTime.innerText = 'Time: -';
            resSize.innerText = 'Size: -';
            requestDebugInfo.innerHTML = ''; // Keep old hidden debug area clear.
            requestDebugInfo.style.display = 'none';
            latestCurlCommand = '';
            latestResponseBody = '';
            latestResponseHeaders = '';
            latestResponseMeta = null;
            if (viewCurlBtn) viewCurlBtn.disabled = true;
            if (openInspectorBtn) openInspectorBtn.disabled = true;
            currentAbortController = new AbortController();
            currentRequestId = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
            cancelBtn.disabled = false;
            
            const startTime = performance.now();
            
            // 1. Substitute Path Variables
            let finalUrl = urlInput.value;
            const pathMap = getKvMap('path-container');
            const missingPathVariables = Object.entries(pathMap)
                .filter(([_key, value]) => !String(value || '').trim())
                .map(([key]) => key);
            if (missingPathVariables.length > 0) {
                responseHeaders.innerText = '(not sent)';
                responseBody.innerText = `Request was not sent. Missing path variable value${missingPathVariables.length === 1 ? '' : 's'}: ${missingPathVariables.join(', ')}`;
                resStatus.innerText = 'Status: not sent';
                resTime.innerText = 'Time: - ms';
                resSize.innerText = 'Size: -';
                cancelBtn.disabled = true;
                currentAbortController = null;
                currentRequestId = null;
                return;
            }
            for (const [key, val] of Object.entries(pathMap)) {
                finalUrl = finalUrl.split(`{${key}}`).join(val);
            }
            finalUrl = substituteRequestVariables(finalUrl);

            const requestParts = getRequestBodyAndHeaders(methodSelect.value, constructHeaders());
            const headers = requestParts.headerPairs.map(([key, value]) => [
                substituteRequestVariables(key),
                substituteRequestVariables(value),
            ]);
            const body = substituteRequestVariables(requestParts.body);
            const unresolvedVariables = new Set([
                ...getUnresolvedRequestVariables(finalUrl),
                ...headers.flatMap(([key, value]) => [
                    ...getUnresolvedRequestVariables(key),
                    ...getUnresolvedRequestVariables(value),
                ]),
                ...getUnresolvedRequestVariables(body),
            ]);
            if (unresolvedVariables.size > 0) {
                const names = Array.from(unresolvedVariables);
                responseHeaders.innerText = '(not sent)';
                responseBody.innerText = `Request was not sent. Unresolved variable${names.length === 1 ? '' : 's'}: ${names.join(', ')}\n\nAdd the value in the active Variables set or replace the {{name}} token before sending.`;
                resStatus.innerText = 'Status: not sent';
                resTime.innerText = 'Time: - ms';
                resSize.innerText = 'Size: -';
                cancelBtn.disabled = true;
                currentAbortController = null;
                currentRequestId = null;
                return;
            }

            const options = {
                method: methodSelect.value,
                url: finalUrl, // Use substituted URL
                headers: headers,
                body: ''
            };

            let curlCmd = `curl -X ${methodSelect.value} "${finalUrl}"`;
            for (const [key, val] of headers) {
                curlCmd += ` \\\n  -H "${key}: ${val}"`;
            }
            
            if (methodSelect.value !== 'GET' && methodSelect.value !== 'HEAD') {
                options.body = body;
                if (body) {
                    // Simple escape for single quotes for display purposes
                    const safeBody = body.replace(/'/g, "'\\''");
                    curlCmd += ` \\\n  -d '${safeBody}'`;
                }
            }
            
            latestCurlCommand = curlCmd;
            if (viewCurlBtn) viewCurlBtn.disabled = false;
            const requestHistoryDetails = { finalUrl };

            try {
                const resp = await fetch('/requests/run', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    signal: currentAbortController.signal,
                    body: JSON.stringify({
                        ...options,
                        headers: headerPairsToPayload(headers),
                        request_id: currentRequestId
                    })
                });
                
                if (!resp.ok) {
                    const errText = await resp.text();
                    throw new Error(errText || 'Request proxy failed');
                }

                const run = await resp.json();
                const duration = run.duration_ms ?? (performance.now() - startTime).toFixed(0);
                const statusCode = Number(run.status || 0);
                const ok = run.curl_exit === 0 && statusCode >= 200 && statusCode < 400;

                resStatus.innerText = `Status: ${statusCode || '0'}`;
                resStatus.className = 'status-badge ' + (ok ? 'success' : 'error');
                resTime.innerText = `Time: ${duration} ms`;
                responseHeaders.innerText = run.headers || '(no headers)';
                latestResponseHeaders = run.headers || '';

                const bodyText = run.body || '';
                latestResponseBody = bodyText;
                latestResponseMeta = {
                    method: methodSelect.value,
                    url: finalUrl,
                    status: statusCode || 0,
                    duration_ms: duration,
                    size_kb: (bodyText.length / 1024).toFixed(2),
                };
                if (openInspectorBtn) openInspectorBtn.disabled = !bodyText;
                resSize.innerText = 'Size: ' + (bodyText.length / 1024).toFixed(2) + ' KB';

                if (run.curl_exit !== 0) {
                    responseBody.innerText = (run.stderr || 'Request failed').trim();
                    await cacheRequestHistory(requestHistoryDetails, {
                        status: statusCode || 0,
                        statusClass: resStatus.className,
                        duration_ms: duration,
                        size_kb: (responseBody.innerText.length / 1024).toFixed(2),
                        headers: latestResponseHeaders,
                        body: bodyText,
                        displayBody: responseBody.innerText,
                        curl: latestCurlCommand,
                        meta: latestResponseMeta,
                    });
                    return;
                }

                try {
                    const json = JSON.parse(bodyText);
                    responseBody.innerText = JSON.stringify(json, null, 4);
                } catch(e) {
                    responseBody.innerText = bodyText;
                }

                await cacheRequestHistory(requestHistoryDetails, {
                    status: statusCode || 0,
                    statusClass: resStatus.className,
                    duration_ms: duration,
                    size_kb: (bodyText.length / 1024).toFixed(2),
                    headers: latestResponseHeaders,
                    body: bodyText,
                    displayBody: responseBody.innerText,
                    curl: latestCurlCommand,
                    meta: latestResponseMeta,
                });
                
            } catch (err) {
                responseHeaders.innerText = '';
                if (err && err.name === 'AbortError') {
                    responseBody.innerText = 'Request cancelled.';
                    resStatus.innerText = 'Status: cancelled';
                } else {
                    responseBody.innerText = 'Error: ' + err.message;
                    resStatus.innerText = 'Error';
                }
                resStatus.className = 'status-badge error';
                await cacheRequestHistory(requestHistoryDetails, {
                    status: 0,
                    statusClass: resStatus.className,
                    duration_ms: Math.round(performance.now() - startTime),
                    size_kb: (responseBody.innerText.length / 1024).toFixed(2),
                    headers: '',
                    body: responseBody.innerText,
                    displayBody: responseBody.innerText,
                    curl: latestCurlCommand,
                    meta: {
                        method: methodSelect.value,
                        url: finalUrl,
                        status: 0,
                        duration_ms: Math.round(performance.now() - startTime),
                        size_kb: (responseBody.innerText.length / 1024).toFixed(2),
                    },
                });
            } finally {
                cancelBtn.disabled = true;
                currentRequestId = null;
                currentAbortController = null;
            }
        }

        sendBtn.addEventListener('click', sendRequest);

        document.addEventListener('keydown', (event) => {
            if (!(event.metaKey || event.ctrlKey) || event.key !== 'Enter') return;
            if (event.target.closest('dialog[open]')) return;

            event.preventDefault();
            sendRequest();
        });

        cancelBtn.addEventListener('click', async () => {
            const requestId = currentRequestId;
            if (!requestId) return;
            if (currentAbortController) currentAbortController.abort();
            cancelBtn.disabled = true;
            try {
                await fetch('/requests/cancel', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ request_id: requestId })
                });
            } catch (_) {}
        });
        
        downloadResBtn.addEventListener('click', () => {
            const content = responseBody.innerText;
            if (!content) return;
            
            const blob = new Blob([content], { type: 'application/json' });
            const link = document.createElement('a');
            link.href = URL.createObjectURL(blob);
            link.download = 'response.json';
            link.click();
            URL.revokeObjectURL(link.href);
        });

        // Response Resizing Only
        const requestDetailsPanel = document.querySelector('.request-details-panel');
        const respSection = document.getElementById('response-section');
        const respResizer = document.getElementById('response-resizer');
        let isRespResizing = false;
        let isHeadersResizing = false;

        // Restore persisted heights
        const savedDetailsHeight = localStorage.getItem(REQUEST_DETAILS_HEIGHT_KEY);
        if (savedDetailsHeight && requestDetailsPanel) {
            requestDetailsPanel.style.flexBasis = savedDetailsHeight;
        }
        const savedHeadersHeight = localStorage.getItem(RESPONSE_HEADERS_HEIGHT_KEY);
        if (savedHeadersHeight) {
            responseHeaders.style.height = savedHeadersHeight;
        }
        
        respResizer.addEventListener('mousedown', (e) => {
            isRespResizing = true;
            respResizer.classList.add('resizing');
            document.body.style.cursor = 'row-resize';
            document.body.style.userSelect = 'none';
        });

        headersBodyResizer.addEventListener('mousedown', (e) => {
            isHeadersResizing = true;
            headersBodyResizer.classList.add('resizing');
            document.body.style.cursor = 'row-resize';
            document.body.style.userSelect = 'none';
        });

        document.addEventListener('mousemove', (e) => {
            if (isRespResizing && requestDetailsPanel) {
                const detailsTop = requestDetailsPanel.getBoundingClientRect().top;
                const mainAreaHeight = document.querySelector('.main-area').offsetHeight;
                const minDetails = 32;
                const maxDetails = Math.max(minDetails, mainAreaHeight - 180);
                const nextDetails = Math.max(minDetails, Math.min(e.clientY - detailsTop, maxDetails));
                requestDetailsPanel.style.flexBasis = `${Math.floor(nextDetails)}px`;
            }

            if (isHeadersResizing) {
                const respRect = respSection.getBoundingClientRect();
                const topOffset = e.clientY - respRect.top;
                const headerOffset = document.querySelector('.response-header').offsetHeight;
                const debugOffset = 0;
                const minHeaders = 50;
                const minBody = 80;
                const maxHeaders = respRect.height - headerOffset - debugOffset - minBody - headersBodyResizer.offsetHeight;
                const nextHeaders = Math.max(minHeaders, Math.min(topOffset - headerOffset - debugOffset, maxHeaders));
                responseHeaders.style.height = `${Math.floor(nextHeaders)}px`;
            }
        });

        document.addEventListener('mouseup', (e) => {
            if (isRespResizing) {
                isRespResizing = false;
                respResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                if (requestDetailsPanel) {
                    localStorage.setItem(REQUEST_DETAILS_HEIGHT_KEY, requestDetailsPanel.style.flexBasis);
                }
            }

            if (isHeadersResizing) {
                isHeadersResizing = false;
                headersBodyResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem(RESPONSE_HEADERS_HEIGHT_KEY, responseHeaders.style.height);
            }
        });
