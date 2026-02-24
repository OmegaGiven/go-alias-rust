(() => {
    if (window.p2p) return;

    let pc = null;
    let dataChannel = null;
    let roomId = null;
    let role = null; // 'host' | 'guest'
    let answerPollTimer = null;
    let icePollTimer = null;
    let reconnectTimer = null;
    let reconnectAttempts = 0;
    let resumeAttempted = false;
    let manualClose = false;
    const seenIce = new Set();
    const pendingRemoteIce = [];

    const SESSION_KEY = 'p2p-session';
    const NAME_KEY = 'p2p-display-name';
    const ICE_SERVERS_KEY = 'p2p-ice-servers';

    // Default is strict local-only (no outside STUN/TURN).
    // To use self-hosted ICE, set localStorage `p2p-ice-servers` to JSON:
    // [{"urls":"stun:your-stun-host:3478"},{"urls":"turn:your-turn-host:3478","username":"u","credential":"p"}]
    function buildRtcConfig() {
        const raw = localStorage.getItem(ICE_SERVERS_KEY);
        if (!raw) return {};
        const parsed = safeParseJson(raw, null);
        if (!Array.isArray(parsed)) return {};
        return { iceServers: parsed };
    }

    const config = buildRtcConfig();

    function safeParseJson(text, fallback = null) {
        try { return JSON.parse(text); } catch (_) { return fallback; }
    }

    function setSession(session) {
        if (!session) {
            sessionStorage.removeItem(SESSION_KEY);
            return;
        }
        sessionStorage.setItem(SESSION_KEY, JSON.stringify(session));
    }

    function getSession() {
        return safeParseJson(sessionStorage.getItem(SESSION_KEY), null);
    }

    function getDisplayName() {
        return localStorage.getItem(NAME_KEY) || 'Anonymous';
    }

    function setDisplayName(name) {
        const normalized = (name || '').trim() || 'Anonymous';
        localStorage.setItem(NAME_KEY, normalized);
        emit('p2p-name', { name: normalized });
        return normalized;
    }

    function emit(name, detail) {
        window.dispatchEvent(new CustomEvent(name, { detail }));
    }

    function updateStatus(msg) {
        const el = role === 'host' ? 'host-status' : 'join-status';
        const target = document.getElementById(el);
        if (target) target.innerText = msg;
        emit('p2p-status', { roomId, role, status: msg, connected: isConnected() });
    }

    function clearTimers() {
        if (answerPollTimer) {
            clearInterval(answerPollTimer);
            answerPollTimer = null;
        }
        if (icePollTimer) {
            clearInterval(icePollTimer);
            icePollTimer = null;
        }
        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
        }
    }

    function teardownPeer() {
        clearTimers();
        if (dataChannel) {
            try { dataChannel.close(); } catch (_) {}
            dataChannel = null;
        }
        if (pc) {
            try { pc.close(); } catch (_) {}
            pc = null;
        }
        seenIce.clear();
        pendingRemoteIce.length = 0;
    }

    function isConnected() {
        return !!(pc && (pc.connectionState === 'connected' || dataChannel?.readyState === 'open'));
    }

    async function postSignal(path, payload) {
        const res = await fetch(path, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        if (!res.ok) {
            const body = await res.text();
            throw new Error(`Signal error ${res.status}: ${body}`);
        }
    }

    async function getRoomInfo(id) {
        const res = await fetch(`/signal/room/${id}`);
        if (!res.ok) {
            return { exists: false, has_offer: false, has_answer: false };
        }
        return res.json();
    }

    async function setupPeerConnection() {
        teardownPeer();
        pc = new RTCPeerConnection(config);

        pc.onicecandidate = async (event) => {
            if (!event.candidate) return;
            const candidateText = JSON.stringify(event.candidate);
            const fingerprint = `${role}:${candidateText}`;
            if (seenIce.has(fingerprint)) return;
            seenIce.add(fingerprint);

            try {
                await postSignal('/signal/ice', {
                    room_id: roomId,
                    data: candidateText,
                    role
                });
            } catch (err) {
                console.error('Failed to send ICE candidate', err);
            }
        };

        pc.onconnectionstatechange = () => {
            const state = pc.connectionState;
            updateStatus(`Connection State: ${state}`);
            if (state === 'connected') {
                reconnectAttempts = 0;
                setSession({ roomId, role, active: true });
            }
            if (state === 'failed' || state === 'disconnected' || state === 'closed') {
                emit('p2p-status', { roomId, role, status: state, connected: false });
                if (!manualClose) scheduleReconnect();
            }
        };

        pc.ondatachannel = (event) => setupDataChannel(event.channel);
    }

    async function waitForIceGatheringComplete(timeoutMs = 4000) {
        if (!pc) return;
        if (pc.iceGatheringState === 'complete') return;

        await new Promise((resolve) => {
            let done = false;
            const finish = () => {
                if (done) return;
                done = true;
                try { pc?.removeEventListener('icegatheringstatechange', onStateChange); } catch (_) {}
                clearTimeout(timer);
                resolve();
            };
            const onStateChange = () => {
                if (pc && pc.iceGatheringState === 'complete') finish();
            };
            const timer = setTimeout(finish, timeoutMs);
            try {
                pc.addEventListener('icegatheringstatechange', onStateChange);
            } catch (_) {
                finish();
            }
        });
    }

    function setupDataChannel(channel) {
        dataChannel = channel;

        dataChannel.onopen = () => {
            updateStatus('P2P Data Channel Open');
            emit('p2p-ready', { connected: true, roomId, role });
        };

        dataChannel.onclose = () => {
            updateStatus('P2P Data Channel Closed');
            emit('p2p-ready', { connected: false, roomId, role });
            if (!manualClose) scheduleReconnect();
        };

        dataChannel.onerror = (e) => {
            console.error('Data channel error', e);
        };

        dataChannel.onmessage = (event) => {
            const msg = safeParseJson(event.data, { type: 'chat', text: event.data, name: 'Peer' });
            emit('p2p-message', msg);

            if (msg.type === 'chat') {
                const msgList = document.getElementById('chat-messages');
                if (msgList) {
                    const div = document.createElement('div');
                    const speaker = (msg.name || 'Peer').trim() || 'Peer';
                    div.textContent = `${speaker}: ${msg.text ?? ''}`;
                    msgList.appendChild(div);
                }
            }
        };
    }

    function scheduleReconnect() {
        const s = getSession();
        if (!s || !s.active || !s.roomId || !s.role) return;
        if (reconnectTimer) return;
        if (isConnected()) return;

        reconnectAttempts += 1;
        const waitMs = Math.min(1200 * reconnectAttempts, 8000);
        updateStatus(`Reconnecting in ${Math.ceil(waitMs / 1000)}s... (attempt ${reconnectAttempts})`);

        reconnectTimer = setTimeout(async () => {
            reconnectTimer = null;
            try {
                if (s.role === 'host') {
                    await hostConnect(s.roomId);
                } else {
                    await guestConnect(s.roomId);
                }
            } catch (err) {
                console.error('Reconnect attempt failed', err);
                scheduleReconnect();
            }
        }, waitMs);
    }

    function hasRemoteDescription() {
        return !!(pc && pc.remoteDescription && pc.remoteDescription.type);
    }

    async function tryAddRemoteIce(candidateText) {
        if (!pc) return false;
        if (!hasRemoteDescription()) return false;
        try {
            await pc.addIceCandidate(new RTCIceCandidate(JSON.parse(candidateText)));
            return true;
        } catch (err) {
            console.error('ICE add error', err);
            return false;
        }
    }

    async function flushPendingRemoteIce() {
        if (!pc || !hasRemoteDescription() || pendingRemoteIce.length === 0) return;
        const toRetry = pendingRemoteIce.splice(0, pendingRemoteIce.length);
        for (const candidateText of toRetry) {
            const added = await tryAddRemoteIce(candidateText);
            if (!added) pendingRemoteIce.push(candidateText);
        }
    }

    async function pollForIce() {
        if (!roomId || !role || !pc) return;

        try {
            const res = await fetch(`/signal/ice/${roomId}/${role}`);
            if (!res.ok) return;
            const candidates = await res.json();

            for (const c of candidates) {
                const fingerprint = `remote:${c}`;
                if (seenIce.has(fingerprint)) continue;
                const added = await tryAddRemoteIce(c);
                if (added) {
                    seenIce.add(fingerprint);
                } else if (!pendingRemoteIce.includes(c)) {
                    pendingRemoteIce.push(c);
                }
            }
            await flushPendingRemoteIce();
        } catch (err) {
            console.error('ICE poll failed', err);
        }
    }

    function startIcePolling() {
        if (icePollTimer) clearInterval(icePollTimer);
        icePollTimer = setInterval(pollForIce, 1500);
        pollForIce();
    }

    async function waitForAnswer() {
        if (answerPollTimer) clearInterval(answerPollTimer);

        answerPollTimer = setInterval(async () => {
            try {
                const res = await fetch(`/signal/answer/${roomId}`);
                if (!res.ok) return;
                const answerText = await res.text();
                if (!answerText) return;

                await pc.setRemoteDescription(new RTCSessionDescription(JSON.parse(answerText)));
                clearInterval(answerPollTimer);
                answerPollTimer = null;
                updateStatus('Answer received. Exchanging network candidates...');
                await flushPendingRemoteIce();
                startIcePolling();
            } catch (err) {
                console.error('Waiting for answer failed', err);
            }
        }, 1200);
    }

    async function hostConnect(existingRoomId) {
        role = 'host';
        manualClose = false;
        updateStatus('Creating room...');

        if (!existingRoomId) {
            const res = await fetch('/signal/create', { method: 'POST' });
            if (!res.ok) throw new Error('Failed to create room');
            const data = await res.json();
            roomId = data.room_id;
            updateStatus(`Room created (${roomId}). Preparing peer connection...`);
        } else {
            roomId = existingRoomId;
            updateStatus(`Rejoining room (${roomId}). Preparing peer connection...`);
        }

        const roomDisplay = document.getElementById('room-id-display');
        if (roomDisplay) roomDisplay.innerText = roomId;

        await setupPeerConnection();
        setupDataChannel(pc.createDataChannel('go_service_data'));

        updateStatus('Creating offer...');
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        await waitForIceGatheringComplete();

        updateStatus('Sending offer...');
        await postSignal('/signal/offer', {
            room_id: roomId,
            data: JSON.stringify(pc.localDescription || offer),
            role
        });

        setSession({ roomId, role, active: true });
        updateStatus('Room created. Waiting for peer to join...');
        waitForAnswer();
    }

    async function guestConnect(id) {
        role = 'guest';
        manualClose = false;
        roomId = (id || '').trim();
        if (!roomId) throw new Error('Enter Room ID');

        updateStatus(`Joining room ${roomId}...`);
        updateStatus('Checking room key...');
        const roomInfo = await getRoomInfo(roomId);
        if (!roomInfo.exists) {
            throw new Error('Room key does not exist');
        }
        updateStatus('Room found. Preparing peer connection...');

        await setupPeerConnection();

        let offerText = '';
        let offerFound = false;
        updateStatus('Waiting for host offer...');
        for (let i = 0; i < 12; i++) {
            const offerRes = await fetch(`/signal/offer/${roomId}`);
            if (offerRes.ok) {
                offerText = await offerRes.text();
                if (offerText) {
                    offerFound = true;
                    break;
                }
            }
            await new Promise((resolve) => setTimeout(resolve, 800));
        }

        if (!offerFound) throw new Error('Room not found or no offer yet');

        updateStatus('Offer received. Creating answer...');
        await pc.setRemoteDescription(new RTCSessionDescription(JSON.parse(offerText)));
        await flushPendingRemoteIce();
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        await waitForIceGatheringComplete();

        updateStatus('Sending answer to host...');
        await postSignal('/signal/answer', {
            room_id: roomId,
            data: JSON.stringify(pc.localDescription || answer),
            role
        });

        setSession({ roomId, role, active: true });
        updateStatus('Answer sent. Exchanging network candidates...');
        startIcePolling();
    }

    function sendRaw(messageObj) {
        if (!dataChannel || dataChannel.readyState !== 'open') return false;
        dataChannel.send(JSON.stringify(messageObj));
        return true;
    }

    const api = {
        createRoom: async () => {
            manualClose = false;
            await hostConnect(null);
            return { roomId, role };
        },

        joinRoom: async (id) => {
            manualClose = false;
            const input = document.getElementById('join-room-id');
            const roomInput = id ?? (input ? input.value : '');
            await guestConnect(roomInput);
            return { roomId, role };
        },

        resumeSession: async () => {
            if (resumeAttempted || isConnected()) return;
            resumeAttempted = true;

            const s = getSession();
            if (!s || !s.roomId || !s.role || !s.active) return;

            try {
                if (s.role === 'host') {
                    await hostConnect(s.roomId);
                } else {
                    await guestConnect(s.roomId);
                }
            } catch (err) {
                console.error('P2P resume failed', err);
            }
        },

        disconnect: async (destroyRoom = true) => {
            manualClose = true;
            const activeRoom = roomId;
            teardownPeer();

            if (destroyRoom && activeRoom) {
                try {
                    await postSignal('/signal/disconnect', { room_id: activeRoom });
                } catch (err) {
                    console.error('Disconnect room cleanup failed', err);
                }
            }

            setSession(null);
            roomId = null;
            role = null;
            reconnectAttempts = 0;
            emit('p2p-ready', { connected: false, roomId: null, role: null });
            updateStatus('Disconnected');
        },

        isConnected,

        getSessionInfo: () => ({
            roomId,
            role,
            connected: isConnected()
        }),

        getDisplayName,
        setDisplayName,

        sendMessage: (customText = null) => {
            const input = document.getElementById('chat-msg-input');
            const msg = customText ?? (input ? input.value : '');
            if (!msg) return false;

            const payload = {
                type: 'chat',
                text: msg,
                name: getDisplayName(),
                ts: Date.now()
            };

            const ok = sendRaw(payload);
            if (!ok) return false;

            const msgList = document.getElementById('chat-messages');
            if (msgList) {
                const div = document.createElement('div');
                div.style.textAlign = 'right';
                div.textContent = `${payload.name}: ${msg}`;
                msgList.appendChild(div);
            }

            if (input) input.value = '';
            return true;
        },

        sendToolMessage: (tool, action, payload) => {
            return sendRaw({
                type: 'tool',
                tool,
                action,
                payload,
                ts: Date.now(),
                role
            });
        },

        sendToolState: (tool, state) => {
            return sendRaw({
                type: 'tool',
                tool,
                action: 'state',
                payload: state,
                ts: Date.now(),
                role
            });
        },

        copyRoomId: () => {
            const id = roomId || (document.getElementById('room-id-display')?.innerText || '').trim();
            if (!id) return;
            navigator.clipboard.writeText(id).catch((err) => console.error('Clipboard failed', err));
        }
    };

    window.p2p = api;

    document.addEventListener('DOMContentLoaded', () => {
        const s = getSession();
        if (s && s.roomId) {
            const roomDisplay = document.getElementById('room-id-display');
            if (roomDisplay && s.role === 'host') roomDisplay.innerText = s.roomId;

            const joinInput = document.getElementById('join-room-id');
            if (joinInput && s.role === 'guest') joinInput.value = s.roomId;
        }

        setDisplayName(getDisplayName());
        api.resumeSession();
    });
})();
