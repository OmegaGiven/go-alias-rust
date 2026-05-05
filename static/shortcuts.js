(() => {
                const LOCAL_SHORTCUTS_KEY = 'go_service_local_shortcuts';
                const LOCAL_HIDDEN_SHORTCUTS_KEY = 'go_service_local_hidden_shortcuts';

                function readShortcutBucket(key) {
                    try {
                        return JSON.parse(localStorage.getItem(key) || '{}');
                    } catch (_) {
                        return {};
                    }
                }

                function writeShortcutBucket(key, value) {
                    localStorage.setItem(key, JSON.stringify(value));
                }

                function buildShortcutPath(key) {
                    return '/' + encodeURIComponent(key).replace(/%2F/g, '/');
                }

                function groupByUrl(shortcuts) {
                    const grouped = new Map();
                    Object.entries(shortcuts)
                        .sort((a, b) => a[0].localeCompare(b[0]))
                        .forEach(([key, url]) => {
                            if (!grouped.has(url)) {
                                grouped.set(url, []);
                            }
                            grouped.get(url).push(key);
                        });
                    return Array.from(grouped.entries()).sort((a, b) => a[0].localeCompare(b[0]));
                }

                function escapeHtml(value) {
                    return value
                        .replaceAll('&', '&amp;')
                        .replaceAll('<', '&lt;')
                        .replaceAll('>', '&gt;')
                        .replaceAll('"', '&quot;')
                        .replaceAll("'", '&#39;');
                }

                function removeLocalShortcut(storageKey, shortcutKey) {
                    if (!window.confirm(`Delete local shortcut ${shortcutKey}?`)) return;

                    const bucket = readShortcutBucket(storageKey);
                    delete bucket[shortcutKey];
                    writeShortcutBucket(storageKey, bucket);
                    renderLocalShortcutSections();
                }

                function renderLocalShortcutTable(containerId, storageKey, emptyMessage) {
                    const container = document.getElementById(containerId);
                    if (!container) return;

                    const shortcuts = readShortcutBucket(storageKey);
                    const grouped = groupByUrl(shortcuts);

                    if (grouped.length === 0) {
                        container.innerHTML = `<p class="shortcut-empty">${escapeHtml(emptyMessage)}</p>`;
                        return;
                    }

                    const rows = grouped.map(([url, keys]) => {
                        const keyLinks = keys.map((key) => {
                            const escapedKey = escapeHtml(key);
                            const escapedUrl = escapeHtml(url);
                            const storageAttr = escapeHtml(storageKey);
                            return `
                                <span class="shortcut-key-chip">
                                    <a href="${escapedUrl}" title="Open ${escapedKey}">${escapedKey}</a>
                                    <button
                                        type="button"
                                        class="local-shortcut-delete"
                                        data-storage-key="${storageAttr}"
                                        data-shortcut-key="${escapedKey}"
                                        title="Delete local shortcut ${escapedKey}"
                                    >Delete</button>
                                </span>
                            `;
                        }).join(' ');

                        return `<tr><td class="keys">${keyLinks}</td><td class="url">${escapeHtml(url)}</td></tr>`;
                    }).join('');

                    container.innerHTML = `
                        <table class="grid shortcut-grid">
                            <thead>
                                <tr><th>Shortcut Keys</th><th>Destination URL</th></tr>
                            </thead>
                            <tbody>${rows}</tbody>
                        </table>
                    `;
                }

                function renderLocalShortcutSections() {
                    renderLocalShortcutTable('local-shortcuts-table', LOCAL_SHORTCUTS_KEY, 'No local shortcuts saved in this browser.');

                    document.querySelectorAll('.local-shortcut-delete').forEach((button) => {
                        button.onclick = () => removeLocalShortcut(button.dataset.storageKey, button.dataset.shortcutKey);
                    });
                }

                window.resolveLocalShortcutPath = function(reqPath) {
                    const localShortcuts = readShortcutBucket(LOCAL_SHORTCUTS_KEY);
                    const hiddenLocalShortcuts = readShortcutBucket(LOCAL_HIDDEN_SHORTCUTS_KEY);
                    const direct = localShortcuts[reqPath] || hiddenLocalShortcuts[reqPath];
                    if (direct) {
                        return direct;
                    }

                    const slashIndex = reqPath.indexOf('/');
                    if (slashIndex === -1) {
                        return null;
                    }

                    const alias = reqPath.slice(0, slashIndex);
                    const remainder = reqPath.slice(slashIndex + 1);
                    const baseUrl = localShortcuts[alias] || hiddenLocalShortcuts[alias];
                    if (!baseUrl) {
                        return null;
                    }

                    return baseUrl.endsWith('/') ? `${baseUrl}${remainder}` : `${baseUrl}/${remainder}`;
                };

                document.addEventListener('DOMContentLoaded', renderLocalShortcutSections);
            })();

document.addEventListener('DOMContentLoaded', () => {
    const popup = document.getElementById('shortcut-not-found-popup');
    if (!popup) return;

    const requestedPath = popup.dataset.requestedPath || '';
    if (requestedPath && typeof window.resolveLocalShortcutPath === 'function') {
        const localUrl = window.resolveLocalShortcutPath(requestedPath);
        if (localUrl) {
            window.location.replace(localUrl);
            return;
        }
    }

    const close = popup.querySelector('.shortcut-not-found-popup-close');
    if (close) close.addEventListener('click', () => popup.classList.add('is-hidden'));
    window.setTimeout(() => popup.classList.add('is-hidden'), 7000);
});
