(() => {
                const LOCAL_SHORTCUTS_KEY = 'ogdevdesk_local_shortcuts';
                const LOCAL_HIDDEN_SHORTCUTS_KEY = 'ogdevdesk_local_hidden_shortcuts';
                const LOCAL_SHORTCUT_GROUPS_KEY = 'ogdevdesk_local_shortcut_groups';
                const LOCAL_SHORTCUT_GROUP_NAMES_KEY = 'ogdevdesk_local_shortcut_group_names';
                let draggedShortcut = null;

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

                function readShortcutGroupNames() {
                    try {
                        const names = JSON.parse(localStorage.getItem(LOCAL_SHORTCUT_GROUP_NAMES_KEY) || '[]');
                        return Array.isArray(names) ? names.filter(Boolean) : [];
                    } catch (_) {
                        return [];
                    }
                }

                function writeShortcutGroupNames(names) {
                    const cleanNames = Array.from(new Set(names.map((name) => name.trim()).filter(Boolean)));
                    cleanNames.sort((a, b) => a.localeCompare(b));
                    localStorage.setItem(LOCAL_SHORTCUT_GROUP_NAMES_KEY, JSON.stringify(cleanNames));
                }

                function buildShortcutPath(key) {
                    return '/' + encodeURIComponent(key).replace(/%2F/g, '/');
                }

                function groupShortcuts(shortcuts, shortcutGroups, groupNames) {
                    const grouped = new Map([['Ungrouped', []]]);
                    groupNames.forEach((group) => {
                        if (group && !grouped.has(group)) grouped.set(group, []);
                    });
                    Object.entries(shortcuts)
                        .sort((a, b) => a[0].localeCompare(b[0]))
                        .forEach(([key, url]) => {
                            const group = shortcutGroups[key] || 'Ungrouped';
                            if (!grouped.has(group)) grouped.set(group, []);
                            grouped.get(group).push([key, url]);
                        });
                    return Array.from(grouped.entries()).sort((a, b) => {
                        if (a[0] === 'Ungrouped') return -1;
                        if (b[0] === 'Ungrouped') return 1;
                        return a[0].localeCompare(b[0]);
                    });
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
                    const shortcutGroups = readShortcutBucket(LOCAL_SHORTCUT_GROUPS_KEY);
                    delete shortcutGroups[shortcutKey];
                    writeShortcutBucket(LOCAL_SHORTCUT_GROUPS_KEY, shortcutGroups);
                    renderLocalShortcutSections();
                }

                function renderLocalShortcutTable(containerId, storageKey, emptyMessage) {
                    const container = document.getElementById(containerId);
                    if (!container) return;

                    const shortcuts = readShortcutBucket(storageKey);
                    const shortcutGroups = readShortcutBucket(LOCAL_SHORTCUT_GROUPS_KEY);
                    const grouped = groupShortcuts(shortcuts, shortcutGroups, readShortcutGroupNames());

                    if (Object.keys(shortcuts).length === 0 && grouped.length <= 1) {
                        container.innerHTML = `<p class="shortcut-empty">${escapeHtml(emptyMessage)}</p>`;
                        return;
                    }

                    container.innerHTML = `
                        <div class="shortcut-group-board">
                            ${grouped.map(([group, entries]) => {
                                const groupAttr = escapeHtml(group === 'Ungrouped' ? '' : group);
                                const rows = entries.length === 0
                                    ? '<tr><td colspan="4" class="shortcut-empty">Drop aliases here.</td></tr>'
                                    : entries.map(([key, url]) => {
                            const escapedKey = escapeHtml(key);
                            const escapedUrl = escapeHtml(url);
                            const storageAttr = escapeHtml(storageKey);
                            return `
                                            <tr class="shortcut-alias-row" draggable="true" data-shortcut-scope="local" data-storage-key="${storageAttr}" data-shortcut-key="${escapedKey}">
                                                <td class="keys"><a href="${buildShortcutPath(key)}" title="Open ${escapedKey}">${escapedKey}</a></td>
                                                <td class="url"><a href="${escapedUrl}" title="${escapedUrl}">${escapedUrl}</a></td>
                                                <td class="shortcut-group-name">${group === 'Ungrouped' ? '' : escapeHtml(group)}</td>
                                                <td class="shortcut-actions">
                                    <button
                                        type="button"
                                        class="local-shortcut-delete"
                                        data-storage-key="${storageAttr}"
                                        data-shortcut-key="${escapedKey}"
                                        title="Delete local shortcut ${escapedKey}"
                                    >Delete</button>
                                                </td>
                                            </tr>
                            `;
                                        }).join('');
                                return `
                                    <section class="shortcut-group-card" data-shortcut-scope="local" data-shortcut-group="${groupAttr}">
                                        <h3>${escapeHtml(group)}</h3>
                                        <table class="grid shortcut-grid">
                                            <thead>
                                                <tr><th>Alias</th><th>Destination URL</th><th>Group</th><th></th></tr>
                                            </thead>
                                            <tbody>${rows}</tbody>
                                        </table>
                                    </section>
                                `;
                            }).join('')}
                        </div>
                    `;
                }

                function renderLocalShortcutSections() {
                    renderLocalShortcutTable('local-shortcuts-table', LOCAL_SHORTCUTS_KEY, 'No local shortcuts saved in this browser.');

                    document.querySelectorAll('.local-shortcut-delete').forEach((button) => {
                        button.onclick = () => removeLocalShortcut(button.dataset.storageKey, button.dataset.shortcutKey);
                    });
                    bindShortcutDragAndDrop();
                }

                async function moveGlobalShortcutToGroup(key, group) {
                    const body = new URLSearchParams();
                    body.set('scope', 'visible');
                    body.set('key', key);
                    body.set('group_name', group);
                    const response = await fetch('/shortcut_group/move', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                        body
                    });
                    if (!response.ok) throw new Error(await response.text());
                    window.location.reload();
                }

                function moveLocalShortcutToGroup(key, group) {
                    const shortcutGroups = readShortcutBucket(LOCAL_SHORTCUT_GROUPS_KEY);
                    if (group) {
                        shortcutGroups[key] = group;
                    } else {
                        delete shortcutGroups[key];
                    }
                    writeShortcutBucket(LOCAL_SHORTCUT_GROUPS_KEY, shortcutGroups);
                    renderLocalShortcutSections();
                }

                function bindShortcutDragAndDrop() {
                    document.querySelectorAll('.shortcut-alias-row').forEach((row) => {
                        if (row.dataset.shortcutDndBound === 'true') return;
                        row.dataset.shortcutDndBound = 'true';
                        row.addEventListener('dragstart', (event) => {
                            draggedShortcut = {
                                scope: row.dataset.shortcutScope,
                                key: row.dataset.shortcutKey,
                            };
                            row.classList.add('dragging');
                            event.dataTransfer.effectAllowed = 'move';
                            event.dataTransfer.setData('text/plain', row.dataset.shortcutKey || '');
                        });
                        row.addEventListener('dragend', () => {
                            row.classList.remove('dragging');
                            document.querySelectorAll('.shortcut-group-card.drop-target').forEach((card) => card.classList.remove('drop-target'));
                            draggedShortcut = null;
                        });
                    });

                    document.querySelectorAll('.shortcut-group-card').forEach((card) => {
                        if (card.dataset.shortcutDndBound === 'true') return;
                        card.dataset.shortcutDndBound = 'true';
                        card.addEventListener('dragover', (event) => {
                            if (!draggedShortcut || draggedShortcut.scope !== card.dataset.shortcutScope) return;
                            event.preventDefault();
                            card.classList.add('drop-target');
                        });
                        card.addEventListener('dragleave', () => {
                            card.classList.remove('drop-target');
                        });
                        card.addEventListener('drop', async (event) => {
                            if (!draggedShortcut || draggedShortcut.scope !== card.dataset.shortcutScope) return;
                            event.preventDefault();
                            card.classList.remove('drop-target');
                            const group = card.dataset.shortcutGroup || '';
                            try {
                                if (draggedShortcut.scope === 'local') {
                                    moveLocalShortcutToGroup(draggedShortcut.key, group);
                                } else {
                                    await moveGlobalShortcutToGroup(draggedShortcut.key, group);
                                }
                            } catch (error) {
                                window.alert(`Could not move alias: ${error.message}`);
                            }
                        });
                    });
                }

                function bindLocalGroupForm() {
                    const form = document.querySelector('[data-local-shortcut-group-form]');
                    if (!form) return;
                    form.addEventListener('submit', (event) => {
                        event.preventDefault();
                        const input = form.querySelector('input[name="group_name"]');
                        const groupName = input ? input.value.trim() : '';
                        if (!groupName) return;
                        const names = readShortcutGroupNames();
                        names.push(groupName);
                        writeShortcutGroupNames(names);
                        input.value = '';
                        renderLocalShortcutSections();
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

                document.addEventListener('DOMContentLoaded', () => {
                    bindLocalGroupForm();
                    renderLocalShortcutSections();
                });
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
