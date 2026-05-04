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
                if (typeof value !== 'number') return 'N/A';
                const d = new Date(value * 1000);
                if (Number.isNaN(d.getTime())) return 'N/A';
                return `${d.toLocaleString()} (${value})`;
            }

            function renderMeta(payload) {
                const exp = unixToDisplay(payload.exp);
                const iat = unixToDisplay(payload.iat);
                metaEl.innerHTML = `<div class=\"jwt-meta-line\">Expiration: ${exp}</div><div class=\"jwt-meta-line\">Issued At: ${iat}</div>`;
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
                    payloadEl.textContent = formatJSON(decoded.payload);
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
                metaEl.innerHTML = '<div class="jwt-meta-line">Expiration: N/A</div><div class="jwt-meta-line">Issued At: N/A</div>';
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
            }

            if (localStorage.getItem('jwt-visible') === 'true') {
                root.style.display = 'flex';
            }
        })();
