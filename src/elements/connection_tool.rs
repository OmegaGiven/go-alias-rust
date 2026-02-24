pub fn get_css() -> String {
    r#"
<style>
    #floating-connection-tool {
        position: fixed;
        top: 180px;
        right: 20px;
        width: 420px;
        max-width: calc(100vw - 40px);
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.35);
        z-index: 1002;
        display: none;
        flex-direction: column;
        overflow: hidden;
    }

    .conn-tool-header {
        background-color: var(--tertiary-bg);
        padding: 8px 10px;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        align-items: center;
        justify-content: space-between;
        cursor: move;
        user-select: none;
        gap: 8px;
    }

    .conn-tool-title {
        font-weight: bold;
        font-size: 0.9em;
    }

    .conn-tool-header-actions {
        display: flex;
        gap: 6px;
        align-items: center;
    }

    .conn-tool-close,
    .conn-tool-disconnect {
        margin: 0 !important;
        padding: 4px 8px;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.75rem;
    }

    .conn-tool-disconnect {
        background: #c13b3b;
        color: #fff;
    }

    .conn-tool-close {
        background: transparent;
        color: var(--text-color);
        font-size: 1.1rem;
        line-height: 1;
        padding: 0 4px;
    }

    .conn-tool-body {
        display: flex;
        flex-direction: column;
        gap: 10px;
        padding: 10px;
    }

    .conn-tool-card {
        border: 1px solid var(--border-color);
        border-radius: 6px;
        background: var(--primary-bg);
        padding: 8px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .conn-tool-row {
        display: flex;
        gap: 8px;
        align-items: center;
    }

    .conn-tool-input {
        flex: 1;
        background: var(--secondary-bg);
        border: 1px solid var(--border-color);
        color: var(--text-color);
        border-radius: 4px;
        padding: 6px;
        min-width: 0;
    }

    .conn-tool-btn {
        margin: 0 !important;
        padding: 6px 10px;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        background: var(--tertiary-bg);
        color: var(--text-color);
        font-size: 0.8rem;
    }

    .conn-tool-btn.primary {
        background: #2f80ed;
        color: #fff;
    }

    .conn-tool-status {
        font-size: 0.8rem;
        opacity: 0.85;
        line-height: 1.3;
        border: 1px solid var(--border-color);
        border-radius: 6px;
        background: var(--primary-bg);
        padding: 8px;
    }

    .conn-tool-chat {
        display: none;
        flex-direction: column;
        gap: 8px;
    }

    .conn-tool-chat.open {
        display: flex;
    }

    #conn-chat-messages {
        border: 1px solid var(--border-color);
        border-radius: 4px;
        min-height: 120px;
        max-height: 180px;
        overflow-y: auto;
        padding: 6px;
        background: var(--secondary-bg);
        font-size: 0.85rem;
    }

    #conn-chat-input {
        flex: 1;
    }

    @media (max-width: 680px) {
        #floating-connection-tool {
            top: 70px;
            left: 10px;
            right: 10px;
            width: auto;
        }
    }
</style>
    "#.to_string()
}

pub fn get_html() -> String {
    r#"
    <div id="floating-connection-tool">
        <div class="conn-tool-header" id="conn-tool-drag-handle">
            <div class="conn-tool-title">Connection</div>
            <div class="conn-tool-header-actions">
                <button id="conn-disconnect-btn" class="conn-tool-disconnect" disabled>Disconnect</button>
                <button class="conn-tool-close" onclick="toggleConnectionTool()">&times;</button>
            </div>
        </div>

        <div class="conn-tool-body">
            <div id="conn-status" class="conn-tool-status">Status: disconnected</div>

            <div class="conn-tool-card">
                <div class="conn-tool-row">
                    <label for="conn-display-name" style="font-size:0.8rem; min-width: 50px;">Name</label>
                    <input id="conn-display-name" class="conn-tool-input" type="text" placeholder="Your name">
                </div>
            </div>

            <div class="conn-tool-card">
                <div class="conn-tool-row">
                    <input id="conn-room-id" class="conn-tool-input" type="text" placeholder="Room key">
                    <button id="conn-create-btn" class="conn-tool-btn primary">Create</button>
                    <button id="conn-join-btn" class="conn-tool-btn primary">Join</button>
                </div>
            </div>

            <div id="conn-chat-wrap" class="conn-tool-chat conn-tool-card">
                <div id="conn-chat-messages"></div>
                <div class="conn-tool-row">
                    <input id="conn-chat-input" class="conn-tool-input" type="text" placeholder="Send message...">
                    <button id="conn-chat-send" class="conn-tool-btn primary">Send</button>
                </div>
            </div>
        </div>
    </div>
    "#.to_string()
}

pub fn get_js() -> String {
    r#"
        (function() {
            const root = document.getElementById('floating-connection-tool');
            if (!root) return;

            const statusEl = document.getElementById('conn-status');
            const disconnectBtn = document.getElementById('conn-disconnect-btn');
            const createBtn = document.getElementById('conn-create-btn');
            const joinBtn = document.getElementById('conn-join-btn');
            const roomIdEl = document.getElementById('conn-room-id');
            const chatWrap = document.getElementById('conn-chat-wrap');
            const chatMessages = document.getElementById('conn-chat-messages');
            const chatInput = document.getElementById('conn-chat-input');
            const chatSend = document.getElementById('conn-chat-send');
            const displayNameEl = document.getElementById('conn-display-name');
            let latestStatusText = 'disconnected';

            function appendMessage(text, alignRight) {
                if (!chatMessages) return;
                const div = document.createElement('div');
                if (alignRight) div.style.textAlign = 'right';
                div.textContent = text;
                chatMessages.appendChild(div);
                chatMessages.scrollTop = chatMessages.scrollHeight;
            }

            function refreshState() {
                if (!window.p2p) return;
                const info = window.p2p.getSessionInfo();
                const connected = !!info.connected;
                const role = info.role || 'none';
                const rid = info.roomId || '';

                if (statusEl) {
                    statusEl.textContent = connected
                        ? `Status: connected as ${role} (${rid})`
                        : `Status: ${latestStatusText || 'disconnected'}`;
                }

                if (disconnectBtn) {
                    disconnectBtn.disabled = !connected;
                    disconnectBtn.style.display = connected ? 'inline-block' : 'none';
                }
                if (roomIdEl && connected) roomIdEl.value = rid;
                if (chatWrap) chatWrap.classList.toggle('open', connected);
            }

            async function onCreate() {
                if (!window.p2p) return;
                try {
                    const session = await window.p2p.createRoom();
                    if (roomIdEl) roomIdEl.value = session.roomId || '';
                    refreshState();
                } catch (err) {
                    latestStatusText = `error: ${err.message || err}`;
                    if (statusEl) statusEl.textContent = `Status: ${latestStatusText}`;
                }
            }

            async function onJoin() {
                if (!window.p2p) return;
                try {
                    const session = await window.p2p.joinRoom(roomIdEl ? roomIdEl.value : '');
                    if (roomIdEl) roomIdEl.value = session.roomId || '';
                    refreshState();
                } catch (err) {
                    latestStatusText = `error: ${err.message || err}`;
                    if (statusEl) statusEl.textContent = `Status: ${latestStatusText}`;
                }
            }

            async function onDisconnect() {
                if (!window.p2p) return;
                await window.p2p.disconnect(true);
                refreshState();
            }

            function onSendChat() {
                if (!window.p2p || !chatInput) return;
                const text = (chatInput.value || '').trim();
                if (!text) return;
                const ok = window.p2p.sendMessage(text);
                if (ok) {
                    const who = window.p2p.getDisplayName();
                    appendMessage(`${who}: ${text}`, true);
                    chatInput.value = '';
                }
            }

            if (createBtn) createBtn.addEventListener('click', onCreate);
            if (joinBtn) joinBtn.addEventListener('click', onJoin);
            if (disconnectBtn) disconnectBtn.addEventListener('click', onDisconnect);
            if (roomIdEl) {
                roomIdEl.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter') {
                        e.preventDefault();
                        onJoin();
                    }
                });
            }
            if (chatSend) chatSend.addEventListener('click', onSendChat);
            if (chatInput) {
                chatInput.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter') {
                        e.preventDefault();
                        if (document.activeElement === chatInput) onSendChat();
                    }
                });
            }

            if (displayNameEl && window.p2p) {
                displayNameEl.value = window.p2p.getDisplayName();
                displayNameEl.addEventListener('change', () => {
                    const next = window.p2p.setDisplayName(displayNameEl.value);
                    displayNameEl.value = next;
                });
            }

            window.addEventListener('p2p-status', (event) => {
                const detail = event.detail || {};
                if (statusEl && detail.status) {
                    latestStatusText = detail.status;
                    statusEl.textContent = `Status: ${latestStatusText}`;
                }
                refreshState();
            });

            window.addEventListener('p2p-message', (event) => {
                const msg = event.detail || {};
                if (msg.type === 'chat') {
                    const who = (msg.name || 'Peer').trim() || 'Peer';
                    appendMessage(`${who}: ${msg.text || ''}`, false);
                }
            });

            window.addEventListener('p2p-name', () => {
                if (displayNameEl && window.p2p) {
                    displayNameEl.value = window.p2p.getDisplayName();
                }
            });

            window.toggleConnectionTool = function() {
                const isVisible = root.style.display === 'flex';
                root.style.display = isVisible ? 'none' : 'flex';
                localStorage.setItem('conn-tool-visible', (!isVisible).toString());
                if (!isVisible) refreshState();
            };

            const dragHandle = document.getElementById('conn-tool-drag-handle');
            if (dragHandle) {
                let isDragging = false;
                let currentX = 0;
                let currentY = 0;
                let initialX = 0;
                let initialY = 0;
                let xOffset = 0;
                let yOffset = 0;

                const savedPos = localStorage.getItem('conn-tool-pos');
                if (savedPos) {
                    try {
                        const pos = JSON.parse(savedPos);
                        xOffset = pos.x || 0;
                        yOffset = pos.y || 0;
                        setTranslate(xOffset, yOffset, root);
                    } catch (_) {}
                }

                dragHandle.addEventListener('mousedown', (e) => {
                    initialX = e.clientX - xOffset;
                    initialY = e.clientY - yOffset;
                    isDragging = true;
                });

                document.addEventListener('mousemove', (e) => {
                    if (!isDragging) return;
                    e.preventDefault();
                    currentX = e.clientX - initialX;
                    currentY = e.clientY - initialY;
                    xOffset = currentX;
                    yOffset = currentY;
                    setTranslate(currentX, currentY, root);
                });

                document.addEventListener('mouseup', () => {
                    if (!isDragging) return;
                    isDragging = false;
                    localStorage.setItem('conn-tool-pos', JSON.stringify({ x: xOffset, y: yOffset }));
                });

                function setTranslate(xPos, yPos, el) {
                    el.style.transform = `translate3d(${xPos}px, ${yPos}px, 0)`;
                }
            }

            refreshState();
            if (localStorage.getItem('conn-tool-visible') === 'true') {
                root.style.display = 'flex';
            }
        })();
    "#.to_string()
}
