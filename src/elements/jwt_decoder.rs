pub fn get_css() -> String {
    r#"
<style>
    #floating-jwt-decoder {
        position: fixed;
        top: 140px;
        right: 20px;
        width: 460px;
        max-width: calc(100vw - 40px);
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
        z-index: 1001;
        display: none;
        flex-direction: column;
        user-select: none;
        overflow: hidden;
    }

    .jwt-header {
        background-color: var(--tertiary-bg);
        padding: 8px 12px;
        cursor: move;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid var(--border-color);
        font-weight: bold;
        font-size: 0.9em;
    }

    .jwt-close-btn {
        background: none;
        border: none;
        color: var(--text-color);
        font-size: 1.2em;
        cursor: pointer;
        opacity: 0.6;
        padding: 0 5px;
        margin: 0;
    }

    .jwt-close-btn:hover {
        opacity: 1;
        color: #f44336;
        background: none;
    }

    .jwt-body {
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }

    .jwt-body label {
        font-size: 0.85em;
        opacity: 0.9;
        margin: 0;
    }

    .jwt-input {
        width: 100%;
        min-height: 84px;
        max-height: 160px;
        resize: vertical;
        border: 1px solid var(--border-color);
        border-radius: 4px;
        background: var(--primary-bg);
        color: var(--text-color);
        font-family: monospace;
        font-size: 0.9em;
        line-height: 1.35;
        padding: 8px;
        box-sizing: border-box;
    }

    .jwt-actions {
        display: flex;
        gap: 8px;
        flex-wrap: wrap;
    }

    .jwt-action-btn {
        margin: 0;
        padding: 6px 12px;
        border: none;
        border-radius: 4px;
        cursor: pointer;
    }

    .jwt-decode-btn {
        background: #4caf50;
        color: #fff;
    }

    .jwt-clear-btn {
        background: #d33;
        color: #fff;
    }

    .jwt-output {
        display: grid;
        gap: 8px;
    }

    .jwt-output-block {
        border: 1px solid var(--border-color);
        border-radius: 4px;
        overflow: hidden;
    }

    .jwt-output-title {
        margin: 0;
        padding: 6px 8px;
        font-size: 0.8em;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        background: var(--tertiary-bg);
        border-bottom: 1px solid var(--border-color);
    }

    .jwt-output pre {
        margin: 0;
        padding: 8px;
        background: var(--primary-bg);
        color: var(--text-color);
        font-size: 0.8em;
        line-height: 1.35;
        max-height: 170px;
        overflow: auto;
        white-space: pre-wrap;
        word-break: break-word;
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }

    .jwt-meta {
        border: 1px solid var(--border-color);
        border-radius: 4px;
        background: var(--primary-bg);
        padding: 8px;
        font-size: 0.82em;
        line-height: 1.4;
    }

    .jwt-meta-line {
        margin: 2px 0;
    }

    .jwt-error {
        border: 1px solid #a33;
        border-radius: 4px;
        background: rgba(180, 40, 40, 0.12);
        color: #ff9a9a;
        padding: 8px;
        font-size: 0.84em;
        display: none;
    }

    @media (max-width: 680px) {
        #floating-jwt-decoder {
            right: 10px;
            left: 10px;
            width: auto;
            top: 70px;
        }
    }
</style>
    "#.to_string()
}

pub fn get_html() -> String {
    r#"
    <div id="floating-jwt-decoder">
        <div class="jwt-header" id="jwt-drag-handle">
            <span>JWT Decoder</span>
            <button class="jwt-close-btn" onclick="toggleJwtDecoder()">&times;</button>
        </div>
        <div class="jwt-body">
            <label for="jwt-input">Paste JWT / API key token</label>
            <textarea id="jwt-input" class="jwt-input" placeholder="eyJhbGciOi..." spellcheck="false"></textarea>

            <div class="jwt-actions">
                <button class="jwt-action-btn jwt-decode-btn" id="jwt-decode-btn">Decode</button>
                <button class="jwt-action-btn jwt-clear-btn" id="jwt-clear-btn">Clear</button>
            </div>

            <div id="jwt-error" class="jwt-error"></div>

            <div class="jwt-output">
                <div class="jwt-output-block">
                    <h4 class="jwt-output-title">Header</h4>
                    <pre id="jwt-header-output">Awaiting token...</pre>
                </div>
                <div class="jwt-output-block">
                    <h4 class="jwt-output-title">Payload</h4>
                    <pre id="jwt-payload-output">Awaiting token...</pre>
                </div>
                <div class="jwt-output-block">
                    <h4 class="jwt-output-title">Signature</h4>
                    <pre id="jwt-signature-output">Awaiting token...</pre>
                </div>
            </div>

            <div id="jwt-meta" class="jwt-meta">
                <div class="jwt-meta-line">Expiration: N/A</div>
                <div class="jwt-meta-line">Issued At: N/A</div>
            </div>
        </div>
    </div>
    "#.to_string()
}

pub fn get_js() -> String {
    r#"
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
    "#.to_string()
}
