document.addEventListener('DOMContentLoaded', () => {
    const assistantWindow = document.getElementById('floating-ai-assistant');
    const settingsWindow = document.getElementById('floating-ai-settings');
    const dragHandle = document.getElementById('ai-assistant-drag-handle');
    const settingsDragHandle = document.getElementById('ai-settings-drag-handle');
    const settingsToggle = document.getElementById('ai-assistant-settings-toggle');
    const settingsForm = document.getElementById('ai-assistant-settings');
    const agentSelect = document.getElementById('ai-agent');
    const agentNameInput = document.getElementById('ai-agent-name');
    const providerSelect = document.getElementById('ai-provider');
    const modelInput = document.getElementById('ai-model');
    const baseUrlField = document.getElementById('ai-base-url-field');
    const baseUrlInput = document.getElementById('ai-base-url');
    const apiKeyField = document.getElementById('ai-api-key-field');
    const apiKeyInput = document.getElementById('ai-api-key');
    const providerStatus = document.getElementById('ai-assistant-provider-status');
    const settingsStatus = document.getElementById('ai-settings-status');
    const testProviderBtn = document.getElementById('ai-test-provider');
    const loadModelsBtn = document.getElementById('ai-load-models');
    const newAgentBtn = document.getElementById('ai-new-agent');
    const contextToggle = document.getElementById('ai-context-toggle');
    const contextMenu = document.getElementById('ai-context-menu');
    const contextSummary = document.getElementById('ai-context-summary');
    const messagesEl = document.getElementById('ai-assistant-messages');
    const chatForm = document.getElementById('ai-assistant-chat-form');
    const input = document.getElementById('ai-assistant-input');
    const sendBtn = document.getElementById('ai-assistant-send');
    const includePage = document.getElementById('ai-include-page');
    const includeSchema = document.getElementById('ai-include-schema');
    const includeSqlTables = document.getElementById('ai-include-sql-tables');
    const includeSqlFunctions = document.getElementById('ai-include-sql-functions');
    const includeEditor = document.getElementById('ai-include-editor');
    const includeResponse = document.getElementById('ai-include-response');
    const includeSqlOutput = document.getElementById('ai-include-sql-output');
    const includeHeaders = document.getElementById('ai-include-headers');

    if (!assistantWindow) return;

    const VISIBLE_KEY = 'ai-assistant-visible';
    const POS_KEY = 'ai-assistant-pos';
    const SETTINGS_VISIBLE_KEY = 'ai-settings-visible';
    const SETTINGS_POS_KEY = 'ai-settings-pos';
    const sensitiveKeyPattern = /authorization|cookie|token|secret|password|api[-_ ]?key|x-api-key/i;
    let profileCache = [];
    let saveTimer = null;
    let isApplyingProfile = false;
    let isSavingSettings = false;
    let saveAgainAfterCurrent = false;
    let latestRemoteContext = null;
    const contextChannel = typeof BroadcastChannel !== 'undefined'
        ? new BroadcastChannel('ogdevdesk-ai-context')
        : null;
    const contextWindowId = `${Date.now()}-${Math.random().toString(16).slice(2)}`;

    function setStatus(text) {
        if (settingsStatus) settingsStatus.textContent = text || '';
    }

    function providerLabel(settings) {
        if (!settings || !settings.model) return 'No provider configured';
        const keyStatus = settings.provider === 'ollama'
            ? 'local'
            : settings.has_api_key ? 'key saved' : 'no key';
        const agentName = settings.name ? `${settings.name}: ` : '';
        return `${agentName}${settings.provider} / ${settings.model} (${keyStatus})`;
    }

    async function loadSettings() {
        try {
            const response = await fetch('/ai/settings');
            const settings = await response.json();
            profileCache = Array.isArray(settings.profiles) ? settings.profiles : [];
            if (!profileCache.length) {
                profileCache = [{
                    id: settings.id || 'default',
                    name: settings.name || 'Default',
                    provider: settings.provider || 'openai',
                    model: settings.model || 'gpt-4.1-mini',
                    base_url: settings.base_url || '',
                    has_api_key: Boolean(settings.has_api_key),
                    is_active: true,
                }];
            }
            renderAgentOptions(settings.id || activeProfile()?.id || 'default');
            applyProfile(profileCache.find((profile) => profile.id === agentSelect.value) || profileCache[0]);
        } catch (error) {
            providerStatus.textContent = `AI settings unavailable: ${error.message}`;
        }
    }

    function syncCredentialVisibility() {
        const usesApiKey = providerSelect.value !== 'ollama';
        if (apiKeyField) apiKeyField.hidden = !usesApiKey;
        if (apiKeyInput) {
            apiKeyInput.disabled = !usesApiKey;
            apiKeyInput.placeholder = usesApiKey ? 'Leave blank to keep saved key' : 'Ollama does not need an API key';
            if (!usesApiKey) apiKeyInput.value = '';
        }
    }

    function syncBaseUrlVisibility() {
        const isCustomProvider = providerSelect.value === 'custom_openai_compatible';
        const isOllama = providerSelect.value === 'ollama';
        if (baseUrlField) baseUrlField.hidden = !isCustomProvider;
        if (baseUrlInput) {
            baseUrlInput.required = isCustomProvider;
            baseUrlInput.placeholder = isOllama
                ? 'Uses http://localhost:11434 by default'
                : isCustomProvider
                ? 'http://localhost:11434/v1 or provider endpoint'
                : 'Built-in provider default';
        }
        syncCredentialVisibility();
    }

    function activeProfile() {
        return profileCache.find((profile) => profile.is_active) || profileCache[0] || null;
    }

    function renderAgentOptions(selectedId) {
        if (!agentSelect) return;
        agentSelect.innerHTML = '';
        profileCache.forEach((profile) => {
            const option = document.createElement('option');
            option.value = profile.id;
            option.textContent = profile.name || profile.id;
            agentSelect.appendChild(option);
        });
        if (selectedId && !profileCache.some((profile) => profile.id === selectedId)) {
            const option = document.createElement('option');
            option.value = selectedId;
            option.textContent = selectedId;
            agentSelect.appendChild(option);
        }
        agentSelect.value = selectedId || profileCache[0]?.id || 'default';
    }

    function applyProfile(profile) {
        if (!profile) return;
        isApplyingProfile = true;
        if (agentSelect) agentSelect.value = profile.id || 'default';
        if (agentNameInput) agentNameInput.value = profile.name || profile.id || 'Default';
        providerSelect.value = profile.provider || 'openai';
        ensureModelOption(profile.model || 'gpt-4.1-mini');
        modelInput.value = profile.model || 'gpt-4.1-mini';
        baseUrlInput.value = profile.base_url || '';
        syncBaseUrlVisibility();
        apiKeyInput.value = '';
        providerStatus.textContent = providerLabel(profile);
        isApplyingProfile = false;
    }

    function profileIdFromName(name) {
        const normalized = String(name || '')
            .trim()
            .toLowerCase()
            .replace(/[^a-z0-9_-]+/g, '-')
            .replace(/^-+|-+$/g, '');
        return normalized || `agent-${Date.now()}`;
    }

    function appendMessage(kind, text) {
        const message = document.createElement('div');
        message.className = `ai-message ai-message-${kind}`;
        message.textContent = text;
        messagesEl.appendChild(message);
        messagesEl.scrollTop = messagesEl.scrollHeight;
        return message;
    }

    function resizeInput() {
        if (!input) return;
        const styles = window.getComputedStyle(input);
        const lineHeight = Number.parseFloat(styles.lineHeight) || 18;
        const paddingTop = Number.parseFloat(styles.paddingTop) || 0;
        const paddingBottom = Number.parseFloat(styles.paddingBottom) || 0;
        const borderTop = Number.parseFloat(styles.borderTopWidth) || 0;
        const borderBottom = Number.parseFloat(styles.borderBottomWidth) || 0;
        const maxHeight = Math.ceil((lineHeight * 10) + paddingTop + paddingBottom + borderTop + borderBottom);
        const minHeight = Math.ceil(lineHeight + paddingTop + paddingBottom + borderTop + borderBottom);
        input.style.height = `${minHeight}px`;
        input.style.height = '0px';
        const nextHeight = Math.max(minHeight, Math.min(input.scrollHeight, maxHeight));
        input.style.height = `${nextHeight}px`;
        input.style.overflowY = input.scrollHeight > maxHeight ? 'auto' : 'hidden';
    }

    function currentPageName() {
        const path = window.location.pathname;
        if (path.startsWith('/sql')) return 'sql';
        if (path.startsWith('/requests')) return 'requests';
        if (path.startsWith('/inspector')) return 'inspector';
        if (path === '/') return 'aliases';
        return 'unknown';
    }

    function contextPageLabel(page = currentPageName()) {
        if (page === 'sql') return 'SQL';
        if (page === 'requests') return 'Requests';
        if (page === 'inspector') return 'Inspector';
        if (page === 'aliases') return 'Web Aliases';
        return 'Current Page';
    }

    function updateContextLabels() {
        const page = latestRemoteContext?.page || currentPageName();
        const pageLabel = contextPageLabel(page);
        const labels = {
            page: `${pageLabel}: Page fields`,
            schema: `${pageLabel}: List metadata`,
            'sql-tables': 'SQL: Tables and columns',
            'sql-functions': 'SQL: Functions',
            editor: page === 'requests' ? 'Requests: Body/editor text' : 'SQL: Editor query',
            response: 'Requests: Response body',
            'sql-output': 'SQL: Output/results',
            headers: page === 'requests' ? 'Requests: Headers' : `${pageLabel}: Headers`,
        };
        Object.entries(labels).forEach(([key, value]) => {
            const el = document.querySelector(`[data-context-label="${key}"]`);
            if (el) el.textContent = value;
        });
        if (includeResponse) {
            includeResponse.closest('label')?.toggleAttribute('hidden', page !== 'requests');
        }
        if (includeSqlOutput) {
            includeSqlOutput.closest('label')?.toggleAttribute('hidden', page !== 'sql');
        }
        if (includeSqlTables) {
            includeSqlTables.closest('label')?.toggleAttribute('hidden', page !== 'sql');
        }
        if (includeSqlFunctions) {
            includeSqlFunctions.closest('label')?.toggleAttribute('hidden', page !== 'sql');
        }
        if (includeSchema) {
            includeSchema.closest('label')?.toggleAttribute('hidden', page === 'sql');
        }
        if (includeHeaders) {
            includeHeaders.closest('label')?.toggleAttribute('hidden', page !== 'requests');
        }
    }

    function positionContextMenu() {
        if (!contextMenu || !contextToggle || contextMenu.hidden) return;
        const toggleRect = contextToggle.getBoundingClientRect();
        const menuRect = contextMenu.getBoundingClientRect();
        const gap = 6;
        const viewportPadding = 8;
        const left = Math.min(
            Math.max(viewportPadding, toggleRect.right - menuRect.width),
            Math.max(viewportPadding, window.innerWidth - menuRect.width - viewportPadding),
        );
        const top = Math.max(viewportPadding, toggleRect.top - menuRect.height - gap);
        contextMenu.style.left = `${left}px`;
        contextMenu.style.top = `${top}px`;
        contextMenu.style.right = 'auto';
    }

    function redacted(value) {
        if (Array.isArray(value)) return value.map(redacted);
        if (value && typeof value === 'object') {
            return Object.fromEntries(Object.entries(value).map(([key, item]) => {
                if (sensitiveKeyPattern.test(key)) return [key, '[redacted]'];
                return [key, redacted(item)];
            }));
        }
        if (typeof value === 'string') {
            const trimmed = value.trim();
            if (/^(Bearer|Basic)\s+/i.test(trimmed)) return '[redacted]';
            if (isJwtLikeSecret(trimmed)) return '[redacted]';
        }
        return value;
    }

    function isJwtLikeSecret(value) {
        if (/\s/.test(value)) return false;
        const parts = value.split('.');
        if (parts.length !== 3) return false;
        return parts.every((part) => part.length > 10 && /^[A-Za-z0-9_-]+={0,2}$/.test(part));
    }

    function activeFlags() {
        return {
            includePage: Boolean(includePage?.checked),
            includeSchema: Boolean(includeSchema?.checked),
            includeSqlTables: Boolean(includeSqlTables?.checked),
            includeSqlFunctions: Boolean(includeSqlFunctions?.checked),
            includeEditor: Boolean(includeEditor?.checked),
            includeResponse: Boolean(includeResponse?.checked),
            includeSqlOutput: Boolean(includeSqlOutput?.checked),
            includeHeaders: Boolean(includeHeaders?.checked),
        };
    }

    async function collectContext() {
        const flags = activeFlags();
        let context = null;

        if (!window.OGDEVDESK_DESKTOP_TOOL && hasLocalContextCollector()) {
            context = collectLocalContext(flags);
        } else {
            context = await requestRemoteContext(flags);
        }

        if (!context) {
            const fallbackPage = latestRemoteContext?.page || currentPageName();
            context = latestRemoteContext || {
                page: fallbackPage,
                note: 'No active SQL, Requests, or Inspector page context is available.',
            };
        }

        latestRemoteContext = context;
        const page = context.page || currentPageName();
        const summary = summarizeContext(context, flags);
        if (contextSummary) contextSummary.textContent = summary;
        return { page, context, summary };
    }

    function summarizeContext(context, flags) {
        const page = context.page || currentPageName();
        const parts = [`Page: ${page}`];
        if (context.connection) parts.push(`Connection: ${context.connection}`);
        if (context.method || context.url) parts.push(`Request: ${context.method || ''} ${context.url || ''}`.trim());
        if (context.sqlEditor) parts.push(`SQL editor: ${context.sqlEditor.length} chars`);
        if (context.editorSql && !context.sqlEditor) parts.push(`SQL editor: ${context.editorSql.length} chars`);
        if (context.body) parts.push(`Body: ${context.body.length} chars`);
        if (context.databaseSchema?.tables) parts.push(`Tables: ${context.databaseSchema.tables.length}`);
        if (context.databaseFunctions) parts.push(`Functions: ${context.databaseFunctions.length}`);
        if (context.schema) {
            const tables = Array.isArray(context.schema.tables) ? context.schema.tables.length : 0;
            const functions = Array.isArray(context.schema.functions) ? context.schema.functions.length : 0;
            parts.push(`Schema: ${tables} tables, ${functions} functions`);
        }
        if (context.responseBody) parts.push(`Response body: ${context.responseBody.length} chars`);
        if (context.sqlOutput) parts.push(`SQL output: ${context.sqlOutput.length} chars`);
        if (context.outputPreview && !context.sqlOutput) parts.push(`SQL output: ${context.outputPreview.length} chars`);
        if (flags.includeHeaders) parts.push('Headers included with redaction');
        return parts.join(' | ');
    }

    function collectLocalContext(flags = activeFlags()) {
        const page = currentPageName();
        let context = { page, note: 'No page-specific context collector is available.' };
        if (page === 'sql' && typeof window.getSqlAssistantContext === 'function') {
            context = window.getSqlAssistantContext(flags);
        } else if (page === 'requests' && typeof window.getRequestsAssistantContext === 'function') {
            context = window.getRequestsAssistantContext(flags);
        } else if (page === 'inspector' && typeof window.getInspectorAssistantContext === 'function') {
            context = window.getInspectorAssistantContext(flags);
        }
        return redacted(context);
    }

    function hasLocalContextCollector() {
        const page = currentPageName();
        return (page === 'sql' && typeof window.getSqlAssistantContext === 'function') ||
            (page === 'requests' && typeof window.getRequestsAssistantContext === 'function') ||
            (page === 'inspector' && typeof window.getInspectorAssistantContext === 'function');
    }

    function requestRemoteContext(flags) {
        if (!contextChannel) return Promise.resolve(null);
        const requestId = `${contextWindowId}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

        return new Promise((resolve) => {
            const timeout = window.setTimeout(() => {
                contextChannel.removeEventListener('message', handleResponse);
                resolve(null);
            }, 800);

            function handleResponse(event) {
                const message = event.data || {};
                if (message.type !== 'ai-context-response' || message.requestId !== requestId) return;
                window.clearTimeout(timeout);
                contextChannel.removeEventListener('message', handleResponse);
                resolve(message.context || null);
            }

            contextChannel.addEventListener('message', handleResponse);
            contextChannel.postMessage({
                type: 'ai-context-request',
                requestId,
                requesterId: contextWindowId,
                flags,
            });
        });
    }

    contextChannel?.addEventListener('message', (event) => {
        const message = event.data || {};
        if (message.type !== 'ai-context-request') return;
        if (message.requesterId === contextWindowId) return;
        if (window.OGDEVDESK_DESKTOP_TOOL) return;
        if (document.visibilityState === 'hidden') return;
        if (!hasLocalContextCollector()) return;

        const context = collectLocalContext(message.flags || activeFlags());
        contextChannel.postMessage({
            type: 'ai-context-response',
            requestId: message.requestId,
            responderId: contextWindowId,
            context,
        });
    });

    function scheduleSettingsSave({ immediate = false } = {}) {
        if (isApplyingProfile) return;
        window.clearTimeout(saveTimer);
        setStatus('Unsaved changes...');
        if (immediate) {
            saveSettings();
            return;
        }
        saveTimer = window.setTimeout(saveSettings, 650);
    }

    async function saveSettings(event) {
        event?.preventDefault();
        if (isSavingSettings) {
            saveAgainAfterCurrent = true;
            return;
        }
        setStatus('Saving...');
        isSavingSettings = true;
        let agentId = agentSelect?.value || 'default';
        const agentName = (agentNameInput?.value || agentId || 'Default').trim();
        if (!agentId || !profileCache.some((profile) => profile.id === agentId)) {
            agentId = profileIdFromName(agentName);
        }
        const payload = {
            id: agentId,
            name: agentName,
            provider: providerSelect.value,
            model: modelInput.value,
            base_url: providerSelect.value === 'custom_openai_compatible' ? baseUrlInput.value : '',
            api_key: providerSelect.value === 'ollama' ? '' : apiKeyInput.value,
        };

        try {
            const response = await fetch('/ai/settings', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload),
            });
            const body = await response.json();
            if (!response.ok) throw new Error(body.error || 'Failed to save settings.');
            apiKeyInput.value = '';
            setStatus('Saved');
            await loadSettings();
            if (body.id && agentSelect) {
                agentSelect.value = body.id;
                const profile = profileCache.find((item) => item.id === body.id);
                if (profile) applyProfile(profile);
            }
        } catch (error) {
            setStatus(error.message);
        } finally {
            isSavingSettings = false;
            if (saveAgainAfterCurrent) {
                saveAgainAfterCurrent = false;
                scheduleSettingsSave({ immediate: true });
            }
        }
    }

    async function testProvider() {
        setStatus('Testing...');
        try {
            const response = await fetch('/ai/test', { method: 'POST' });
            const body = await response.json();
            if (!response.ok) throw new Error(body.error || 'Provider test failed.');
            setStatus(body.message || 'Provider reachable');
        } catch (error) {
            setStatus(error.message);
        }
    }

    async function loadModels() {
        setStatus('Loading models...');
        try {
            const response = await fetch('/ai/models');
            const body = await response.json();
            if (!response.ok) throw new Error(body.error || 'Failed to load models.');

            const models = Array.isArray(body.models) ? body.models : [];
            const currentModel = modelInput.value;
            modelInput.innerHTML = '';
            models.forEach((model) => {
                const option = document.createElement('option');
                option.value = model;
                option.textContent = model;
                modelInput.appendChild(option);
            });
            if (currentModel) ensureModelOption(currentModel);
            if (models.includes(currentModel)) {
                modelInput.value = currentModel;
            } else if (models.length) {
                modelInput.value = models[0];
                scheduleSettingsSave();
            }
            setStatus(`Loaded ${models.length} model${models.length === 1 ? '' : 's'}`);
        } catch (error) {
            setStatus(error.message);
        }
    }

    function ensureModelOption(model) {
        if (!model || !modelInput) return;
        const exists = Array.from(modelInput.options).some((option) => option.value === model);
        if (exists) return;
        const option = document.createElement('option');
        option.value = model;
        option.textContent = model;
        modelInput.appendChild(option);
    }

    function createNewAgent() {
        const name = window.prompt('New AI agent name', 'New AI Agent');
        if (!name || !name.trim()) return;
        let id = profileIdFromName(name);
        let suffix = 2;
        while (profileCache.some((profile) => profile.id === id)) {
            id = `${profileIdFromName(name)}-${suffix}`;
            suffix += 1;
        }
        const profile = {
            id,
            name: name.trim(),
            provider: 'openai',
            model: 'gpt-4.1-mini',
            base_url: '',
            has_api_key: false,
            is_active: false,
            is_unsaved: true,
        };
        profileCache.push(profile);
        renderAgentOptions(id);
        applyProfile(profile);
        setStatus('New AI agent created');
        scheduleSettingsSave();
    }

    async function setActiveAgent(profile) {
        if (!profile || profile.is_unsaved) return;
        setStatus('Switching agent...');
        try {
            const response = await fetch('/ai/settings/active', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ id: profile.id }),
            });
            const body = await response.json();
            if (!response.ok) throw new Error(body.error || 'Failed to switch AI agent.');
            profileCache = profileCache.map((item) => ({
                ...item,
                is_active: item.id === profile.id,
            }));
            setStatus('Agent selected');
        } catch (error) {
            setStatus(error.message);
        }
    }


    async function sendMessage(event) {
        event.preventDefault();
        const message = input.value.trim();
        if (!message) return;
        input.value = '';
        resizeInput();
        appendMessage('user', message);
        const pending = appendMessage('assistant', 'Thinking...');
        sendBtn.disabled = true;

        const { page, context, summary } = await collectContext();
        try {
            const response = await fetch('/ai/chat', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    message,
                    page,
                    context,
                    context_summary: summary,
                }),
            });
            const body = await response.json();
            if (!response.ok) throw new Error(body.error || 'AI request failed.');
            pending.textContent = body.message || '(no response)';
        } catch (error) {
            pending.classList.add('ai-message-error');
            pending.textContent = error.message;
        } finally {
            sendBtn.disabled = false;
            input.focus();
        }
    }

    function centerAssistantWindow() {
        const rect = assistantWindow.getBoundingClientRect();
        assistantWindow.style.left = `${Math.max(10, (window.innerWidth - rect.width) / 2)}px`;
        assistantWindow.style.top = `${Math.max(10, (window.innerHeight - rect.height) / 2)}px`;
        assistantWindow.style.right = 'auto';
        savePosition();
    }

    function bringToFront() {
        if (window.bringFloatingWindowToFront) {
            window.bringFloatingWindowToFront(assistantWindow);
            return;
        }
        assistantWindow.style.zIndex = '10040';
    }

    function bringSettingsToFront() {
        if (!settingsWindow) return;
        if (window.bringFloatingWindowToFront) {
            window.bringFloatingWindowToFront(settingsWindow);
            return;
        }
        settingsWindow.style.zIndex = '10050';
    }

    function savePosition() {
        const rect = assistantWindow.getBoundingClientRect();
        localStorage.setItem(POS_KEY, JSON.stringify({
            left: rect.left,
            top: rect.top,
        }));
    }

    function restorePosition() {
        const savedPos = localStorage.getItem(POS_KEY);
        if (!savedPos) return;
        try {
            const pos = JSON.parse(savedPos);
            const width = assistantWindow.offsetWidth || 520;
            const height = assistantWindow.offsetHeight || 680;
            const left = Math.min(Math.max(0, Number(pos.left) || 0), Math.max(0, window.innerWidth - width));
            const top = Math.min(Math.max(0, Number(pos.top) || 0), Math.max(0, window.innerHeight - height));
            assistantWindow.style.left = `${left}px`;
            assistantWindow.style.top = `${top}px`;
            assistantWindow.style.right = 'auto';
        } catch (_) { }
    }

    function openAssistantWindow({ focusInput = true } = {}) {
        assistantWindow.style.display = 'flex';
        localStorage.setItem(VISIBLE_KEY, 'true');
        restorePosition();
        bringToFront();
        collectContext();
        loadSettings();
        if (focusInput) setTimeout(() => input?.focus(), 0);
    }

    function toggleAiAssistantInPage() {
        const isOpen = assistantWindow.style.display === 'flex';
        if (isOpen) {
            assistantWindow.style.display = 'none';
            if (settingsWindow) settingsWindow.style.display = 'none';
            localStorage.setItem(VISIBLE_KEY, 'false');
            localStorage.setItem(SETTINGS_VISIBLE_KEY, 'false');
            return;
        }

        openAssistantWindow();
    }

    window.toggleAiAssistant = function () {
        if (window.openDesktopToolWindow?.('ai', toggleAiAssistantInPage)) return;
        toggleAiAssistantInPage();
    };

    window.toggleAiSettingsWindow = function () {
        if (!settingsWindow) return;
        const isOpen = settingsWindow.style.display === 'flex';
        settingsWindow.style.display = isOpen ? 'none' : 'flex';
        localStorage.setItem(SETTINGS_VISIBLE_KEY, (!isOpen).toString());
        if (!isOpen) {
            restoreWindowPosition(settingsWindow, SETTINGS_POS_KEY);
            bringSettingsToFront();
            loadSettings();
        }
    };

    settingsToggle?.addEventListener('click', window.toggleAiSettingsWindow);
    settingsForm?.addEventListener('submit', saveSettings);
    testProviderBtn?.addEventListener('click', testProvider);
    loadModelsBtn?.addEventListener('click', loadModels);
    newAgentBtn?.addEventListener('click', createNewAgent);
    contextToggle?.addEventListener('click', () => {
        if (!contextMenu) return;
        contextMenu.hidden = !contextMenu.hidden;
        if (!contextMenu.hidden) {
            updateContextLabels();
            collectContext();
            positionContextMenu();
        }
    });
    document.addEventListener('click', (event) => {
        if (!contextMenu || contextMenu.hidden) return;
        if (event.target.closest('.ai-context-menu-wrap')) return;
        contextMenu.hidden = true;
    });
    providerSelect?.addEventListener('change', () => {
        syncBaseUrlVisibility();
        scheduleSettingsSave();
    });
    agentNameInput?.addEventListener('input', () => scheduleSettingsSave());
    modelInput?.addEventListener('change', () => scheduleSettingsSave());
    baseUrlInput?.addEventListener('input', () => scheduleSettingsSave());
    apiKeyInput?.addEventListener('change', () => scheduleSettingsSave({ immediate: true }));
    agentSelect?.addEventListener('change', () => {
        const profile = profileCache.find((item) => item.id === agentSelect.value);
        applyProfile(profile);
        setActiveAgent(profile);
    });
    chatForm?.addEventListener('submit', sendMessage);
    input?.addEventListener('input', resizeInput);
    input?.addEventListener('keydown', (event) => {
        if (event.key !== 'Enter' || event.shiftKey || event.metaKey || event.ctrlKey || event.altKey) return;
        event.preventDefault();
        chatForm?.requestSubmit();
    });
    window.addEventListener('resize', positionContextMenu);
    window.addEventListener('scroll', positionContextMenu, true);
    [
        includePage,
        includeSchema,
        includeSqlTables,
        includeSqlFunctions,
        includeEditor,
        includeResponse,
        includeSqlOutput,
        includeHeaders,
    ].forEach((checkbox) => {
        checkbox?.addEventListener('change', collectContext);
    });

    if (dragHandle) {
        let isDragging = false;
        let dragOffsetX = 0;
        let dragOffsetY = 0;

        dragHandle.addEventListener('mousedown', (event) => {
            if (event.button !== 0) return;
            const rect = assistantWindow.getBoundingClientRect();
            isDragging = true;
            dragOffsetX = event.clientX - rect.left;
            dragOffsetY = event.clientY - rect.top;
            assistantWindow.style.left = `${rect.left}px`;
            assistantWindow.style.top = `${rect.top}px`;
            assistantWindow.style.right = 'auto';
            bringToFront();
            document.body.style.userSelect = 'none';
        });

        document.addEventListener('mousemove', (event) => {
            if (!isDragging) return;
            const width = assistantWindow.offsetWidth;
            const height = assistantWindow.offsetHeight;
            const nextLeft = Math.min(Math.max(0, event.clientX - dragOffsetX), window.innerWidth - width);
            const nextTop = Math.min(Math.max(0, event.clientY - dragOffsetY), window.innerHeight - height);
            assistantWindow.style.left = `${nextLeft}px`;
            assistantWindow.style.top = `${nextTop}px`;
        });

        document.addEventListener('mouseup', () => {
            if (!isDragging) return;
            isDragging = false;
            document.body.style.userSelect = '';
            savePosition();
        });

        assistantWindow.addEventListener('dblclick', (event) => {
            const rect = assistantWindow.getBoundingClientRect();
            const edgeSize = 12;
            const onEdge = event.clientX - rect.left <= edgeSize
                || rect.right - event.clientX <= edgeSize
                || event.clientY - rect.top <= edgeSize
                || rect.bottom - event.clientY <= edgeSize;
            if (onEdge) centerAssistantWindow();
        });
    }

    bindWindowDrag(settingsWindow, settingsDragHandle, SETTINGS_POS_KEY, bringSettingsToFront);

    function restoreWindowPosition(element, key) {
        if (!element) return;
        const savedPos = localStorage.getItem(key);
        if (!savedPos) return;
        try {
            const pos = JSON.parse(savedPos);
            const width = element.offsetWidth || 380;
            const height = element.offsetHeight || 300;
            const left = Math.min(Math.max(0, Number(pos.left) || 0), Math.max(0, window.innerWidth - width));
            const top = Math.min(Math.max(0, Number(pos.top) || 0), Math.max(0, window.innerHeight - height));
            element.style.left = `${left}px`;
            element.style.top = `${top}px`;
            element.style.right = 'auto';
        } catch (_) { }
    }

    function saveWindowPosition(element, key) {
        if (!element) return;
        const rect = element.getBoundingClientRect();
        localStorage.setItem(key, JSON.stringify({
            left: rect.left,
            top: rect.top,
        }));
    }

    function bindWindowDrag(element, handle, key, bringForward) {
        if (!element || !handle) return;
        let isDragging = false;
        let dragOffsetX = 0;
        let dragOffsetY = 0;

        handle.addEventListener('mousedown', (event) => {
            if (event.button !== 0) return;
            const rect = element.getBoundingClientRect();
            isDragging = true;
            dragOffsetX = event.clientX - rect.left;
            dragOffsetY = event.clientY - rect.top;
            element.style.left = `${rect.left}px`;
            element.style.top = `${rect.top}px`;
            element.style.right = 'auto';
            bringForward?.();
            document.body.style.userSelect = 'none';
        });

        document.addEventListener('mousemove', (event) => {
            if (!isDragging) return;
            const width = element.offsetWidth;
            const height = element.offsetHeight;
            const nextLeft = Math.min(Math.max(0, event.clientX - dragOffsetX), window.innerWidth - width);
            const nextTop = Math.min(Math.max(0, event.clientY - dragOffsetY), window.innerHeight - height);
            element.style.left = `${nextLeft}px`;
            element.style.top = `${nextTop}px`;
        });

        document.addEventListener('mouseup', () => {
            if (!isDragging) return;
            isDragging = false;
            document.body.style.userSelect = '';
            saveWindowPosition(element, key);
        });
    }

    loadSettings();
    updateContextLabels();
    resizeInput();
    syncBaseUrlVisibility();
    restorePosition();
    if (localStorage.getItem(VISIBLE_KEY) === 'true') {
        openAssistantWindow({ focusInput: false });
    }
    restoreWindowPosition(settingsWindow, SETTINGS_POS_KEY);
    if (localStorage.getItem(SETTINGS_VISIBLE_KEY) === 'true') {
        if (settingsWindow) settingsWindow.style.display = 'flex';
        bringSettingsToFront();
    }
});
