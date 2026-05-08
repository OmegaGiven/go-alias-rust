(function() {
    document.addEventListener('DOMContentLoaded', () => {
        const root = document.getElementById('floating-scratch-pad');
        if (!root) return;

        const tabsEl = document.getElementById('scratchpad-tabs');
        const addBtn = document.getElementById('scratchpad-add-tab');
        const deleteBtn = document.getElementById('scratchpad-delete-tab');
        const editor = document.getElementById('scratchpad-editor');
        const titleInput = document.getElementById('scratchpad-title');
        const statusEl = document.getElementById('scratchpad-save-status');
        const dragHandle = document.getElementById('scratchpad-drag-handle');

        const STORAGE_KEY = 'ogdevdesk_scratchpads';
        const ACTIVE_KEY = 'ogdevdesk_scratchpad_active';
        const VISIBLE_KEY = 'scratchpad-visible';
        const POS_KEY = 'scratchpad-pos';

        let pads = [];
        let activeId = '';
        let saveTimer = null;

        function defaultPad() {
            return {
                id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
                title: 'Scratch 1',
                text: '',
                updatedAt: new Date().toISOString(),
            };
        }

        function normalizePads(value) {
            if (Array.isArray(value) && value.length > 0) {
                return value.map((pad, index) => ({
                    id: pad.id || `${Date.now()}-${index}`,
                    title: pad.title || `Scratch ${index + 1}`,
                    text: pad.text || '',
                    updatedAt: pad.updatedAt || new Date().toISOString(),
                }));
            }
            return [defaultPad()];
        }

        function loadPads() {
            try {
                const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
                return normalizePads(parsed);
            } catch (_) {}
            return [defaultPad()];
        }

        function savePads() {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(pads));
            localStorage.setItem(ACTIVE_KEY, activeId);
            fetch('/scratchpads', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(pads),
            }).catch((err) => console.error('Failed to save scratch pads to app database', err));
            setStatus('Saved');
        }

        async function loadPadsFromServer() {
            try {
                const resp = await fetch('/scratchpads');
                if (!resp.ok) throw new Error(await resp.text());
                const serverPads = await resp.json();
                if (!Array.isArray(serverPads) || serverPads.length === 0) {
                    savePads();
                    return;
                }

                pads = normalizePads(serverPads);
                activeId = localStorage.getItem(ACTIVE_KEY) || pads[0].id;
                if (!pads.some((pad) => pad.id === activeId)) activeId = pads[0].id;
                localStorage.setItem(STORAGE_KEY, JSON.stringify(pads));
                renderActivePad();
                setStatus('Saved');
            } catch (err) {
                console.error('Failed to load scratch pads from app database', err);
            }
        }

        function setStatus(text) {
            if (statusEl) statusEl.textContent = text;
        }

        function activePad() {
            return pads.find((pad) => pad.id === activeId) || pads[0];
        }

        function scheduleSave() {
            setStatus('Saving...');
            window.clearTimeout(saveTimer);
            saveTimer = window.setTimeout(savePads, 200);
        }

        function renderTabs() {
            tabsEl.innerHTML = '';
            pads.forEach((pad) => {
                const tab = document.createElement('button');
                tab.type = 'button';
                tab.className = `scratchpad-tab${pad.id === activeId ? ' active' : ''}`;
                tab.textContent = pad.title || 'Untitled';
                tab.title = pad.title || 'Untitled';
                tab.setAttribute('role', 'tab');
                tab.setAttribute('aria-selected', (pad.id === activeId).toString());
                tab.addEventListener('click', () => setActivePad(pad.id));
                tabsEl.appendChild(tab);
            });
        }

        function renderActivePad() {
            const pad = activePad();
            activeId = pad.id;
            editor.value = pad.text || '';
            titleInput.value = pad.title || 'Untitled';
            renderTabs();
            localStorage.setItem(ACTIVE_KEY, activeId);
        }

        function setActivePad(id) {
            activeId = id;
            renderActivePad();
        }

        function addPad() {
            const next = defaultPad();
            next.title = `Scratch ${pads.length + 1}`;
            pads.push(next);
            activeId = next.id;
            renderActivePad();
            savePads();
            editor.focus();
        }

        function deleteActivePad() {
            const pad = activePad();
            if (!window.confirm(`Delete scratch pad "${pad.title || 'Untitled'}"?`)) return;

            if (pads.length <= 1) {
                pad.title = 'Scratch 1';
                pad.text = '';
                pad.updatedAt = new Date().toISOString();
                renderActivePad();
                savePads();
                return;
            }

            const currentIndex = pads.findIndex((pad) => pad.id === activeId);
            pads = pads.filter((pad) => pad.id !== activeId);
            const nextIndex = Math.max(0, currentIndex - 1);
            activeId = pads[nextIndex].id;
            renderActivePad();
            savePads();
        }

        editor.addEventListener('input', () => {
            const pad = activePad();
            pad.text = editor.value;
            pad.updatedAt = new Date().toISOString();
            scheduleSave();
        });

        titleInput.addEventListener('input', () => {
            const pad = activePad();
            pad.title = titleInput.value.trim() || 'Untitled';
            pad.updatedAt = new Date().toISOString();
            renderTabs();
            scheduleSave();
        });

        addBtn.addEventListener('click', addPad);
        deleteBtn.addEventListener('click', deleteActivePad);

        window.toggleScratchPad = function() {
            const isVisible = root.style.display === 'flex';
            root.style.display = isVisible ? 'none' : 'flex';
            localStorage.setItem(VISIBLE_KEY, (!isVisible).toString());
            if (!isVisible) editor.focus();
        };

        function restorePosition() {
            const savedPos = localStorage.getItem(POS_KEY);
            if (!savedPos) return { x: 0, y: 0 };
            try {
                const pos = JSON.parse(savedPos);
                const x = pos.x || 0;
                const y = pos.y || 0;
                root.style.transform = `translate3d(${x}px, ${y}px, 0)`;
                return { x, y };
            } catch (_) {
                return { x: 0, y: 0 };
            }
        }

        function bindDrag() {
            if (!dragHandle) return;

            let isDragging = false;
            let currentX = 0;
            let currentY = 0;
            let initialX = 0;
            let initialY = 0;
            let { x: xOffset, y: yOffset } = restorePosition();

            dragHandle.addEventListener('mousedown', (event) => {
                initialX = event.clientX - xOffset;
                initialY = event.clientY - yOffset;
                if (event.target === dragHandle || dragHandle.contains(event.target)) {
                    isDragging = true;
                }
            });

            root.addEventListener('dblclick', (event) => {
                if (!isOuterEdgeClick(event)) return;

                const rect = root.getBoundingClientRect();
                const targetLeft = Math.max(0, (window.innerWidth - rect.width) / 2);
                const targetTop = Math.max(0, (window.innerHeight - rect.height) / 2);
                xOffset += targetLeft - rect.left;
                yOffset += targetTop - rect.top;
                currentX = xOffset;
                currentY = yOffset;
                root.style.transform = `translate3d(${xOffset}px, ${yOffset}px, 0)`;
                localStorage.setItem(POS_KEY, JSON.stringify({ x: xOffset, y: yOffset }));
            });

            document.addEventListener('mousemove', (event) => {
                if (!isDragging) return;
                event.preventDefault();
                currentX = event.clientX - initialX;
                currentY = event.clientY - initialY;
                xOffset = currentX;
                yOffset = currentY;
                root.style.transform = `translate3d(${currentX}px, ${currentY}px, 0)`;
            });

            document.addEventListener('mouseup', () => {
                if (!isDragging) return;
                isDragging = false;
                localStorage.setItem(POS_KEY, JSON.stringify({ x: xOffset, y: yOffset }));
            });

            function isOuterEdgeClick(event) {
                if (event.target.closest('button, input, textarea, select, a')) return false;
                const rect = root.getBoundingClientRect();
                const edge = 18;
                const onEdge = event.clientX - rect.left <= edge ||
                    rect.right - event.clientX <= edge ||
                    event.clientY - rect.top <= edge ||
                    rect.bottom - event.clientY <= edge;
                return onEdge || dragHandle.contains(event.target);
            }
        }

        pads = loadPads();
        activeId = localStorage.getItem(ACTIVE_KEY) || pads[0].id;
        if (!pads.some((pad) => pad.id === activeId)) activeId = pads[0].id;

        renderActivePad();
        loadPadsFromServer();
        bindDrag();
        setStatus('Saved');

        if (localStorage.getItem(VISIBLE_KEY) === 'true') {
            root.style.display = 'flex';
        }
    });
})();
