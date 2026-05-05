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
        const RESPONSE_HEIGHT_KEY = 'request-response-height';
        const RESPONSE_HEADERS_HEIGHT_KEY = 'request-response-headers-height';
        const SAVED_REQUEST_FOLDERS_COLLAPSED_KEY = 'saved-request-folders-collapsed';
        const INSPECTOR_PENDING_PAYLOAD_KEY = 'inspector_pending_payload';
        let pendingPostmanCollection = null;
        let collapsedRequestFolders = readCollapsedRequestFolders();
        let latestCurlCommand = '';
        let latestResponseBody = '';
        let latestResponseHeaders = '';
        let latestResponseMeta = null;
        let variableSetDialogMode = 'create';
        
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
        const reqFolderSelect = document.getElementById('req-folder');

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
            const removeBtn = isReadOnlyKey ? '' : `<button class="kv-remove" onclick="this.parentElement.remove(); onKvChange('${containerId}')">x</button>`;
            const readOnlyAttr = isReadOnlyKey ? 'readonly' : '';
            const keyClass = isReadOnlyKey ? 'kv-input key readonly' : 'kv-input key';
            
            row.innerHTML = `
                <input type="text" class="${keyClass}" placeholder="Key" value="${key}" ${readOnlyAttr} oninput="onKvChange('${containerId}')">
                <input type="text" class="kv-input val" placeholder="Value" value="${val}" oninput="onKvChange('${containerId}')">
                ${removeBtn}
            `;
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
            if (type === 'raw') {
                document.getElementById('body-raw-container').style.display = 'flex';
                document.getElementById('body-form-container').style.display = 'none';
            } else {
                document.getElementById('body-raw-container').style.display = 'none';
                document.getElementById('body-form-container').style.display = 'flex';
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
        
        urlInput.addEventListener('input', () => { parseUrlToParams(); detectPathVariables(); });

        // --- Auth UI ---
        authTypeSelect.addEventListener('change', () => renderAuthInputs());

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

        function constructHeaders() {
            const headers = getKvPairs('headers-container');
            const authType = authTypeSelect.value;

            if (authType === 'bearer' || authType === 'oauth2') {
                const token = authType === 'oauth2' ? fetchedOAuthToken : document.getElementById('auth-bearer-token')?.value;
                if(token) headers.push(['Authorization', `Bearer ${token}`]);
            }
            if (authType === 'basic') {
                const user = document.getElementById('auth-basic-user')?.value || '';
                const pass = document.getElementById('auth-basic-pass')?.value || '';
                headers.push(['Authorization', `Basic ${btoa(`${user}:${pass}`)}`]);
            }
            if (authType === 'apikey') {
                const key = document.getElementById('auth-api-key')?.value?.trim() || '';
                const val = document.getElementById('auth-api-val')?.value || '';
                if (key) headers.push([key, val]);
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

        function headersToString() {
            const h = constructHeaders();
            return h.map(([k, v]) => `${k}: ${v}`).join('\\n');
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

        function filterSavedRequests() {
            if (!savedRequestSearch || !savedList) return;

            const filter = savedRequestSearch.value.toUpperCase();
            const folderStates = [];
            let currentFolder = null;
            let currentFolderKey = '';
            let currentFolderCollapsed = false;
            let folderHasVisibleRequest = false;

            Array.from(savedList.children).forEach((item) => {
                if (item.classList.contains('saved-req-folder')) {
                    if (currentFolder) {
                        folderStates.push({
                            element: currentFolder,
                            hasVisibleRequest: folderHasVisibleRequest,
                            collapsed: currentFolderCollapsed,
                        });
                    }

                    currentFolder = item;
                    currentFolderKey = item.dataset.folder || '';
                    currentFolderCollapsed = collapsedRequestFolders.has(currentFolderKey);
                    folderHasVisibleRequest = false;
                    item.style.display = 'none';
                    item.classList.toggle('collapsed', currentFolderCollapsed);
                    const toggle = item.querySelector('.saved-req-folder-toggle');
                    if (toggle) toggle.textContent = currentFolderCollapsed ? '▸' : '▾';
                    return;
                }

                const link = item.querySelector('.req-link');
                if (!link) return;

                const itemText = [
                    link.dataset.name || '',
                    link.dataset.method || '',
                    link.dataset.url || '',
                ].join(' ');
                const matchesFilter = itemText.toUpperCase().includes(filter);
                const isVisible = matchesFilter && !currentFolderCollapsed;
                item.style.display = isVisible ? 'flex' : 'none';
                if (matchesFilter) {
                    folderHasVisibleRequest = true;
                }
            });

            if (currentFolder) {
                folderStates.push({
                    element: currentFolder,
                    hasVisibleRequest: folderHasVisibleRequest,
                    collapsed: currentFolderCollapsed,
                });
            }

            folderStates.forEach((folder) => {
                folder.element.style.display = folder.hasVisibleRequest || filter === '' ? 'flex' : 'none';
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
            if (reqFolderSelect) reqFolderSelect.value = '';
            saveControls.style.display = 'none';
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

        if (savedRequestSearch) {
            savedRequestSearch.addEventListener('input', filterSavedRequests);
            filterSavedRequests();
        }

        if (newRequestBtn) {
            newRequestBtn.addEventListener('click', resetRequestBuilder);
        }

        if (createRequestFolderBtn && createRequestFolderForm && newRequestFolderName) {
            createRequestFolderBtn.addEventListener('click', () => {
                const folderName = window.prompt('Folder name');
                if (!folderName || folderName.trim() === '') return;

                newRequestFolderName.value = folderName.trim();
                createRequestFolderForm.submit();
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

        if (openInspectorBtn) {
            openInspectorBtn.addEventListener('click', () => {
                const payload = {
                    source: 'requests',
                    body: latestResponseBody || responseBody.innerText || '',
                    headers: latestResponseHeaders || responseHeaders.innerText || '',
                    meta: latestResponseMeta || {},
                    captured_at: new Date().toISOString(),
                };
                sessionStorage.setItem(INSPECTOR_PENDING_PAYLOAD_KEY, JSON.stringify(payload));
                window.location.href = '/inspector';
            });
        }

        toggleSaveBtn.addEventListener('click', () => {
            saveControls.style.display = saveControls.style.display === 'flex' ? 'none' : 'flex';
            if (saveControls.style.display === 'flex') {
                reqNameInput.focus();
            }
        });

        saveControls.addEventListener('submit', () => {
            saveMethod.value = methodSelect.value;
            saveUrl.value = urlInput.value;
            const headers = constructHeaders();
            const reqPayload = getRequestBodyAndHeaders(methodSelect.value, headers);
            saveHeaders.value = reqPayload.headerPairs.map(([k, v]) => `${k}: ${v}`).join('\\n');
            saveBody.value = reqPayload.body;
            saveAuthType.value = authTypeSelect.value;
            
            if (authTypeSelect.value === 'oauth2') {
                saveOAuthTokenUrl.value = document.getElementById('oauth-token-url')?.value || '';
                saveOAuthClientId.value = document.getElementById('oauth-client-id')?.value || '';
                saveOAuthClientSecret.value = document.getElementById('oauth-client-secret')?.value || '';
                saveOAuthScope.value = document.getElementById('oauth-scope')?.value || '';
            }
        });

        savedList.addEventListener('click', (e) => {
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

            const link = e.target.closest('.req-link');
            if (link) {
                e.preventDefault();
                
                // Toggle selection class
                document.querySelectorAll('.saved-req-item').forEach(el => el.classList.remove('selected'));
                link.closest('.saved-req-item').classList.add('selected');

                methodSelect.value = link.dataset.method;
                urlInput.value = link.dataset.url;
                stringToHeadersTable(link.dataset.headers);
                bodyInput.value = link.dataset.body;
                reqNameInput.value = link.dataset.name;
                if (reqFolderSelect) {
                    reqFolderSelect.value = link.dataset.folder || '';
                }
                
                let savedAuthType = link.dataset.authType || 'none';
                if (savedAuthType === 'none') {
                    const inferred = inferAuthTypeFromHeaders();
                    if (inferred !== 'none') savedAuthType = inferred;
                }
                authTypeSelect.value = savedAuthType;
                const savedAuthData = {
                    oauth_token_url: link.dataset.oauthTokenUrl,
                    oauth_client_id: link.dataset.oauthClientId,
                    oauth_client_secret: link.dataset.oauthClientSecret,
                    oauth_scope: link.dataset.oauthScope
                };
                renderAuthInputs(savedAuthData);
                applyAuthDefaultsFromHeaders(savedAuthType);
                parseUrlToParams(); 
                detectPathVariables();
            }
        });

        document.getElementById('format-json-btn').addEventListener('click', () => {
            try { bodyInput.value = JSON.stringify(JSON.parse(bodyInput.value), null, 4); } catch(e) { alert('Invalid JSON'); }
        });

        sendBtn.addEventListener('click', async () => {
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
                    return;
                }

                try {
                    const json = JSON.parse(bodyText);
                    responseBody.innerText = JSON.stringify(json, null, 4);
                } catch(e) {
                    responseBody.innerText = bodyText;
                }
                
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
            } finally {
                cancelBtn.disabled = true;
                currentRequestId = null;
                currentAbortController = null;
            }
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
        const respSection = document.getElementById('response-section');
        const respResizer = document.getElementById('response-resizer');
        let isRespResizing = false;
        let isHeadersResizing = false;

        // Restore persisted heights
        const savedRespHeight = localStorage.getItem(RESPONSE_HEIGHT_KEY);
        if (savedRespHeight) {
            respSection.style.height = savedRespHeight;
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
            if (isRespResizing) {
                const containerHeight = document.querySelector('.main-area').offsetHeight;
                
                const distFromBottom = window.innerHeight - e.clientY - 20; // 20 padding
                if (distFromBottom > 50 && distFromBottom < containerHeight - 100) {
                    respSection.style.height = distFromBottom + 'px';
                }
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
                localStorage.setItem(RESPONSE_HEIGHT_KEY, respSection.style.height);
            }

            if (isHeadersResizing) {
                isHeadersResizing = false;
                headersBodyResizer.classList.remove('resizing');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem(RESPONSE_HEADERS_HEIGHT_KEY, responseHeaders.style.height);
            }
        });
