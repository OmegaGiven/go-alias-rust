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
        const requestDebugInfo = document.getElementById('request-debug-info');
        const RESPONSE_HEIGHT_KEY = 'request-response-height';
        const RESPONSE_HEADERS_HEIGHT_KEY = 'request-response-headers-height';
        
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

        // Auth Elements
        const authTypeSelect = document.getElementById('auth-type');
        const authInputs = document.getElementById('auth-inputs');
        let fetchedOAuthToken = ''; // Store the token here
        let currentRequestId = null;
        let currentAbortController = null;

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
            const regex = /\{([^}]+)\}/g;
            let match;
            const foundKeys = new Set();
            while ((match = regex.exec(url)) !== null) { foundKeys.add(match[1]); }

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

        toggleSaveBtn.addEventListener('click', () => {
            saveControls.style.display = saveControls.style.display === 'flex' ? 'none' : 'flex';
            if (saveControls.style.display === 'flex') {
                document.getElementById('req-name').focus();
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

        document.getElementById('saved-list').addEventListener('click', (e) => {
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
                document.getElementById('req-name').value = link.dataset.name; 
                
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
            requestDebugInfo.innerHTML = ''; // Clear old debug info
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

            const requestParts = getRequestBodyAndHeaders(methodSelect.value, constructHeaders());
            const headers = requestParts.headerPairs;
            const body = requestParts.body;

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
            
            // Display the debug info
            requestDebugInfo.style.display = 'block';
            requestDebugInfo.innerText = curlCmd;

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

                const bodyText = run.body || '';
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
                const debugOffset = requestDebugInfo.style.display === 'none' ? 0 : requestDebugInfo.offsetHeight;
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