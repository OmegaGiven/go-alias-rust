        (function() {
            const root = document.getElementById('floating-jwt-decoder');
            if (!root) return;

            const input = document.getElementById('jwt-input');
            const decodeBtn = document.getElementById('jwt-decode-btn');
            const clearBtn = document.getElementById('jwt-clear-btn');
            const errorEl = document.getElementById('jwt-error');
            const headerEl = document.getElementById('jwt-header-output');
            const payloadEl = document.getElementById('jwt-payload-output');
            const signatureEl = document.getElementById('jwt-signature-output');
            const metaEl = document.getElementById('jwt-meta');

            function setError(message) {
                errorEl.textContent = message;
                errorEl.style.display = 'block';
            }

            function clearError() {
                errorEl.textContent = '';
                errorEl.style.display = 'none';
            }

            function formatJSON(value) {
                return JSON.stringify(value, null, 2);
            }

            function isDateLikeKey(key) {
                const normalized = String(key || '').toLowerCase();
                return normalized === 'exp' ||
                    normalized === 'iat' ||
                    normalized === 'nbf' ||
                    normalized === 'expires' ||
                    normalized === 'expiration' ||
                    normalized === 'expires_at' ||
                    normalized === 'expiresat' ||
                    normalized === 'issued_at' ||
                    normalized === 'issuedat' ||
                    normalized === 'not_before' ||
                    normalized === 'notbefore' ||
                    normalized === 'created_at' ||
                    normalized === 'createdat' ||
                    normalized === 'updated_at' ||
                    normalized === 'updatedat' ||
                    normalized === 'deleted_at' ||
                    normalized === 'deletedat' ||
                    normalized === 'date' ||
                    normalized.endsWith('_date') ||
                    normalized.endsWith('date') ||
                    normalized.endsWith('_at') ||
                    normalized.endsWith('at') ||
                    normalized.endsWith('_timestamp') ||
                    normalized.endsWith('timestamp');
            }

            function parseDateLikeValue(value) {
                if (typeof value === 'number' && Number.isFinite(value)) {
                    const milliseconds = value > 100000000000 ? value : value * 1000;
                    const date = new Date(milliseconds);
                    return Number.isNaN(date.getTime()) ? null : date;
                }

                if (typeof value === 'string') {
                    const trimmed = value.trim();
                    if (!trimmed) return null;

                    if (/^\d+(\.\d+)?$/.test(trimmed)) {
                        const parsed = Number(trimmed);
                        if (!Number.isFinite(parsed)) return null;
                        const milliseconds = parsed > 100000000000 ? parsed : parsed * 1000;
                        const date = new Date(milliseconds);
                        return Number.isNaN(date.getTime()) ? null : date;
                    }

                    const date = new Date(trimmed);
                    return Number.isNaN(date.getTime()) ? null : date;
                }

                return null;
            }

            function dateToReadable(date) {
                return `${date.toLocaleString()} (${date.toISOString()})`;
            }

            function addReadableDates(value) {
                if (Array.isArray(value)) {
                    return value.map(addReadableDates);
                }

                if (!value || typeof value !== 'object') {
                    return value;
                }

                const next = {};
                Object.entries(value).forEach(([key, entryValue]) => {
                    next[key] = addReadableDates(entryValue);

                    if (isDateLikeKey(key)) {
                        const date = parseDateLikeValue(entryValue);
                        if (date) {
                            next[`${key}_readable`] = dateToReadable(date);
                        }
                    }
                });

                return next;
            }

            function decodeBase64Url(str) {
                const normalized = str.replace(/-/g, '+').replace(/_/g, '/');
                const padded = normalized + '='.repeat((4 - (normalized.length % 4)) % 4);
                const decoded = atob(padded);

                let utf8 = '';
                for (let i = 0; i < decoded.length; i++) {
                    utf8 += '%' + decoded.charCodeAt(i).toString(16).padStart(2, '0');
                }
                try {
                    return decodeURIComponent(utf8);
                } catch (_) {
                    return decoded;
                }
            }

            function parseJwt(rawToken) {
                let token = rawToken.trim();
                token = token.replace(/^Bearer\s+/i, '');

                const parts = token.split('.');
                if (parts.length !== 3) {
                    throw new Error('Token must have 3 dot-separated sections (header.payload.signature).');
                }

                const headerText = decodeBase64Url(parts[0]);
                const payloadText = decodeBase64Url(parts[1]);
                const signature = parts[2];

                const header = JSON.parse(headerText);
                const payload = JSON.parse(payloadText);

                return { header, payload, signature };
            }

            function unixToDisplay(value) {
                const d = parseDateLikeValue(value);
                if (!d) return 'N/A';
                return dateToReadable(d);
            }

            function renderMeta(payload) {
                const exp = unixToDisplay(payload.exp);
                const iat = unixToDisplay(payload.iat);
                const nbf = unixToDisplay(payload.nbf);
                metaEl.textContent = '';

                [
                    ['Expiration', exp],
                    ['Issued At', iat],
                    ['Not Before', nbf],
                ].forEach(([label, value]) => {
                    const line = document.createElement('div');
                    line.className = 'jwt-meta-line';
                    line.textContent = `${label}: ${value}`;
                    metaEl.appendChild(line);
                });
            }

            function decodeAndRender() {
                clearError();

                const raw = input.value;
                if (!raw || !raw.trim()) {
                    setError('Paste a JWT token first.');
                    return;
                }

                try {
                    const decoded = parseJwt(raw);
                    headerEl.textContent = formatJSON(decoded.header);
                    payloadEl.textContent = formatJSON(addReadableDates(decoded.payload));
                    signatureEl.textContent = decoded.signature || '(empty signature)';
                    renderMeta(decoded.payload);
                } catch (err) {
                    setError(err.message || 'Failed to decode token.');
                }
            }

            function resetDecoder() {
                input.value = '';
                clearError();
                headerEl.textContent = 'Awaiting token...';
                payloadEl.textContent = 'Awaiting token...';
                signatureEl.textContent = 'Awaiting token...';
                renderMeta({});
            }

            if (decodeBtn) decodeBtn.addEventListener('click', decodeAndRender);
            if (clearBtn) clearBtn.addEventListener('click', resetDecoder);
            if (input) {
                input.addEventListener('keydown', (event) => {
                    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
                        event.preventDefault();
                        decodeAndRender();
                    }
                });
            }

            window.toggleJwtDecoder = function() {
                const isVisible = root.style.display === 'flex';
                root.style.display = isVisible ? 'none' : 'flex';
                localStorage.setItem('jwt-visible', (!isVisible).toString());
            };

            const dragHandle = document.getElementById('jwt-drag-handle');
            if (dragHandle) {
                let isDragging = false;
                let currentX = 0;
                let currentY = 0;
                let initialX = 0;
                let initialY = 0;
                let xOffset = 0;
                let yOffset = 0;

                const savedPos = localStorage.getItem('jwt-pos');
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
                    if (e.target === dragHandle || dragHandle.contains(e.target)) {
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
                    setTranslate(xOffset, yOffset, root);
                    localStorage.setItem('jwt-pos', JSON.stringify({ x: xOffset, y: yOffset }));
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
                    localStorage.setItem('jwt-pos', JSON.stringify({ x: xOffset, y: yOffset }));
                });

                function setTranslate(xPos, yPos, el) {
                    el.style.transform = `translate3d(${xPos}px, ${yPos}px, 0)`;
                }

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

            if (localStorage.getItem('jwt-visible') === 'true') {
                root.style.display = 'flex';
            }
        })();
