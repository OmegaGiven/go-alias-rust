const INSPECTOR_PENDING_PAYLOAD_KEY = 'inspector_pending_payload';
const JSON_NODE_LIMIT = 25000;

const contentInput = document.getElementById('content-input');
const fileInput = document.getElementById('file-input');
const lineInput = document.getElementById('line-num');
const colInput = document.getElementById('col-num');
const resultSection = document.getElementById('result-section');
const indicator = document.getElementById('type-indicator');
const prettifyButton = document.getElementById('prettify-btn');
const sourceMeta = document.getElementById('inspector-source-meta');
const inputResizer = document.getElementById('inspector-input-resizer');

const jsonTools = document.getElementById('json-tools');
const jsonSummaryLine = document.getElementById('json-summary-line');
const jsonRawPreview = document.getElementById('json-raw-preview');
const jsonTreeView = document.getElementById('json-tree-view');
const jsonSummaryView = document.getElementById('json-summary-view');
const jsonTableView = document.getElementById('json-table-view');
const jsonTableStatus = document.getElementById('json-table-status');
const jsonCopyCsvBtn = document.getElementById('json-copy-csv-btn');
const jsonSearchInput = document.getElementById('json-search-input');
const jsonSearchCount = document.getElementById('json-search-count');
const jsonTreeLayout = document.querySelector('.json-tree-layout');
const jsonDetailResizer = document.getElementById('json-detail-resizer');

const detailPath = document.getElementById('json-detail-path');
const detailType = document.getElementById('json-detail-type');
const detailSize = document.getElementById('json-detail-size');
const detailPreview = document.getElementById('json-detail-preview');

let jsonState = {
    parsed: null,
    selectedPath: '$',
    selectedValue: null,
    matches: [],
    matchIndex: -1,
    csv: '',
};

fileInput.addEventListener('change', (event) => {
    const file = event.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (readerEvent) => {
        contentInput.value = readerEvent.target.result;
        detectContent();
    };
    reader.readAsText(file);
});

document.querySelectorAll('[data-json-view]').forEach((button) => {
    button.addEventListener('click', () => showJsonView(button.dataset.jsonView));
});

document.getElementById('json-expand-one-btn').addEventListener('click', () => renderJsonTree(1));
document.getElementById('json-expand-all-btn').addEventListener('click', () => renderJsonTree(999));
document.getElementById('json-collapse-all-btn').addEventListener('click', () => renderJsonTree(0));
document.getElementById('json-copy-path-btn').addEventListener('click', () => copyText(jsonState.selectedPath || '$'));
document.getElementById('json-copy-value-btn').addEventListener('click', () => copyText(formatJsonValue(jsonState.selectedValue, false)));
document.getElementById('json-copy-pretty-btn').addEventListener('click', () => copyText(formatJsonValue(jsonState.selectedValue, true)));
document.getElementById('json-search-prev-btn').addEventListener('click', () => stepJsonSearch(-1));
document.getElementById('json-search-next-btn').addEventListener('click', () => stepJsonSearch(1));
jsonSearchInput.addEventListener('input', runJsonSearch);
jsonCopyCsvBtn.addEventListener('click', () => copyText(jsonState.csv));

function initInspectorResizers() {
    let activeResize = null;

    if (inputResizer) {
        inputResizer.addEventListener('mousedown', (event) => {
            const inputTop = contentInput.getBoundingClientRect().top;
            activeResize = {
                type: 'input',
                inputTop,
            };
            inputResizer.classList.add('resizing');
            document.body.style.cursor = 'row-resize';
            document.body.style.userSelect = 'none';
            event.preventDefault();
        });
    }

    if (jsonDetailResizer && jsonTreeLayout) {
        jsonDetailResizer.addEventListener('mousedown', (event) => {
            const details = document.querySelector('.json-node-details');
            if (!details) return;

            const isStacked = window.matchMedia('(max-width: 640px)').matches;
            activeResize = {
                type: isStacked ? 'details-height' : 'details-width',
                startX: event.clientX,
                startY: event.clientY,
                startBasis: isStacked ? details.offsetHeight : details.offsetWidth,
                details,
                layout: jsonTreeLayout,
            };
            jsonDetailResizer.classList.add('resizing');
            document.body.style.cursor = isStacked ? 'row-resize' : 'col-resize';
            document.body.style.userSelect = 'none';
            event.preventDefault();
        });
    }

    document.addEventListener('mousemove', (event) => {
        if (!activeResize) return;

        if (activeResize.type === 'input') {
            const containerHeight = document.querySelector('.input-section')?.clientHeight || window.innerHeight;
            const nextHeight = event.clientY - activeResize.inputTop;
            const maxHeight = Math.max(120, containerHeight - 120);
            contentInput.style.height = `${Math.min(Math.max(nextHeight, 56), maxHeight)}px`;
            return;
        }

        if (activeResize.type === 'details-width') {
            const layoutWidth = activeResize.layout.clientWidth;
            const nextWidth = activeResize.startBasis - (event.clientX - activeResize.startX);
            const maxWidth = Math.max(260, layoutWidth - 220);
            activeResize.details.style.flexBasis = `${Math.min(Math.max(nextWidth, 220), maxWidth)}px`;
            return;
        }

        if (activeResize.type === 'details-height') {
            const layoutHeight = activeResize.layout.clientHeight;
            const nextHeight = activeResize.startBasis - (event.clientY - activeResize.startY);
            const maxHeight = Math.max(160, layoutHeight - 160);
            activeResize.details.style.flexBasis = `${Math.min(Math.max(nextHeight, 120), maxHeight)}px`;
        }
    });

    document.addEventListener('mouseup', () => {
        if (!activeResize) return;
        inputResizer?.classList.remove('resizing');
        jsonDetailResizer?.classList.remove('resizing');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        activeResize = null;
    });
}

initInspectorResizers();

function loadPendingInspectorPayload() {
    const raw = sessionStorage.getItem(INSPECTOR_PENDING_PAYLOAD_KEY);
    if (!raw) return;

    try {
        const payload = JSON.parse(raw);
        contentInput.value = payload.body || '';
        if (sourceMeta) {
            const meta = payload.meta || {};
            const parts = [
                meta.method && meta.url ? `${meta.method} ${meta.url}` : '',
                meta.status ? `Status ${meta.status}` : '',
                meta.duration_ms ? `${meta.duration_ms} ms` : '',
                meta.size_kb ? `${meta.size_kb} KB` : '',
            ].filter(Boolean);
            sourceMeta.textContent = parts.length ? `Loaded from Requests: ${parts.join(' | ')}` : 'Loaded from Requests';
            sourceMeta.hidden = false;
        }
        sessionStorage.removeItem(INSPECTOR_PENDING_PAYLOAD_KEY);
    } catch (_) {
        sessionStorage.removeItem(INSPECTOR_PENDING_PAYLOAD_KEY);
    }
}

function detectContentType(text) {
    if (text.length === 0) return 'empty';
    if ((text.startsWith('{') || text.startsWith('[')) && isValidJSON(text)) return 'json';
    if (text.startsWith('<') && isValidXML(text)) return 'xml';
    if (isLikelySQL(text)) return 'sql';
    return 'text';
}

function detectContent() {
    const text = contentInput.value.trim();
    const contentType = detectContentType(text);

    if (contentType === 'empty') {
        indicator.textContent = 'Empty';
        indicator.className = 'indicator';
        prettifyButton.textContent = 'Prettify';
        prettifyButton.disabled = true;
        clearJsonTools();
        return;
    }

    if (contentType === 'json') {
        indicator.textContent = 'Valid JSON';
        indicator.className = 'indicator valid-json';
        prettifyButton.textContent = 'Prettify JSON';
        prettifyButton.disabled = false;
        renderJsonTools(text);
        return;
    }

    clearJsonTools();
    if (contentType === 'xml') {
        indicator.textContent = 'Valid XML';
        indicator.className = 'indicator valid-xml';
        prettifyButton.textContent = 'Prettify XML';
        prettifyButton.disabled = false;
        return;
    }

    if (contentType === 'sql') {
        indicator.textContent = 'SQL';
        indicator.className = 'indicator';
        prettifyButton.textContent = 'Prettify SQL';
        prettifyButton.disabled = false;
        return;
    }

    indicator.textContent = 'Plain Text';
    indicator.className = 'indicator';
    prettifyButton.textContent = 'Prettify';
    prettifyButton.disabled = true;
}

function isValidJSON(text) {
    try {
        JSON.parse(text);
        return true;
    } catch (_) {
        return false;
    }
}

function isValidXML(text) {
    try {
        const parser = new DOMParser();
        const doc = parser.parseFromString(text, 'application/xml');
        return !doc.querySelector('parsererror');
    } catch (_) {
        return false;
    }
}

function isLikelySQL(text) {
    const normalized = text.trim().replace(/\s+/g, ' ').toUpperCase();
    if (!normalized) return false;

    return [
        /^SELECT\b.*\bFROM\b/,
        /^WITH\b/,
        /^INSERT\s+INTO\b/,
        /^UPDATE\b.*\bSET\b/,
        /^DELETE\s+FROM\b/,
        /^CREATE\s+(TABLE|VIEW|INDEX|OR\s+REPLACE)\b/,
        /^ALTER\b/,
        /^DROP\b/,
    ].some((pattern) => pattern.test(normalized));
}

function toggleWrap() {
    contentInput.style.whiteSpace = document.getElementById('wrap-toggle').checked ? 'pre-wrap' : 'pre';
}

function formatJSON() {
    try {
        if (!contentInput.value) return;
        contentInput.value = JSON.stringify(JSON.parse(contentInput.value), null, 4);
        detectContent();
    } catch (error) {
        alert('Invalid JSON: ' + error.message);
    }
}

function formatXML() {
    try {
        const val = contentInput.value;
        if (!val) return;

        const parser = new DOMParser();
        const doc = parser.parseFromString(val, 'application/xml');
        if (doc.querySelector('parsererror')) throw new Error('Invalid XML');

        const serializer = new XMLSerializer();
        const formatted = serializer
            .serializeToString(doc)
            .replace(/>\s*</g, '><')
            .replace(/(>)(<)(\/*)/g, '$1\n$2$3');

        let indentLevel = 0;
        contentInput.value = formatted
            .split('\n')
            .map((line) => {
                const trimmed = line.trim();
                if (!trimmed) return '';
                if (/^<\//.test(trimmed)) indentLevel = Math.max(indentLevel - 1, 0);
                const result = `${'    '.repeat(indentLevel)}${trimmed}`;
                if (/^<[^!?/][^>]*[^/]?>$/.test(trimmed)) indentLevel += 1;
                return result;
            })
            .filter(Boolean)
            .join('\n');
        detectContent();
    } catch (error) {
        alert('Invalid XML: ' + error.message);
    }
}

function mapOutsideSqlLiterals(sql, mapper) {
    let output = '';
    let outside = '';
    let mode = null;

    for (let i = 0; i < sql.length; i++) {
        const ch = sql[i];
        const next = i + 1 < sql.length ? sql[i + 1] : '';

        if (!mode) {
            if (ch === "'" || ch === '"' || ch === '`') {
                output += mapper(outside);
                outside = '';
                mode = ch;
                output += ch;
            } else {
                outside += ch;
            }
            continue;
        }

        output += ch;
        if (mode === "'" && ch === "'" && next === "'") {
            output += next;
            i++;
            continue;
        }
        if (ch === mode && sql[i - 1] !== '\\') mode = null;
    }

    output += mapper(outside);
    return output;
}

function formatSQL() {
    const val = contentInput.value;
    if (!val || !val.trim()) return;

    let formatted = mapOutsideSqlLiterals(val, (segment) => segment.replace(/\s+/g, ' ')).trim();
    [
        'WITH', 'SELECT', 'FROM', 'WHERE', 'GROUP BY', 'ORDER BY',
        'HAVING', 'LIMIT', 'OFFSET', 'INSERT INTO', 'VALUES',
        'UPDATE', 'SET', 'DELETE FROM', 'UNION ALL', 'UNION',
    ].forEach((clause) => {
        const escaped = clause.replace(/\s+/g, '\\s+');
        formatted = mapOutsideSqlLiterals(formatted, (segment) =>
            segment.replace(new RegExp(`\\b${escaped}\\b`, 'gi'), `\n${clause}`)
        );
    });

    formatted = mapOutsideSqlLiterals(formatted, (segment) =>
        segment
            .replace(/\b(LEFT|RIGHT|INNER|FULL|CROSS)\s+JOIN\b/gi, '\n$1 JOIN')
            .replace(/\bJOIN\b/gi, '\nJOIN')
            .replace(/\bON\b/gi, '\n  ON')
            .replace(/\bAND\b/gi, '\n  AND')
            .replace(/\bOR\b/gi, '\n  OR')
            .replace(/,\s*/g, ',\n  ')
            .replace(/\s*\(\s*/g, ' (')
            .replace(/\s*\)\s*/g, ') ')
    );

    contentInput.value = formatted.replace(/\n{2,}/g, '\n').replace(/[ \t]+\n/g, '\n').trim();
    detectContent();
}

function prettifyContent() {
    const text = contentInput.value.trim();
    const contentType = detectContentType(text);
    if (contentType === 'json') return formatJSON();
    if (contentType === 'xml') return formatXML();
    if (contentType === 'sql') formatSQL();
}

function clearJsonTools() {
    jsonTools.hidden = true;
    jsonState = { parsed: null, selectedPath: '$', selectedValue: null, matches: [], matchIndex: -1, csv: '' };
}

function renderJsonTools(text) {
    try {
        jsonState.parsed = JSON.parse(text);
    } catch (_) {
        clearJsonTools();
        return;
    }

    jsonTools.hidden = false;
    jsonRawPreview.textContent = JSON.stringify(jsonState.parsed, null, 2);
    const stats = summarizeJson(jsonState.parsed);
    jsonSummaryLine.textContent = `${describeNode(jsonState.parsed)} | ${stats.nodes} nodes | depth ${stats.maxDepth}`;
    renderJsonTree(stats.nodes > 700 ? 0 : 1);
    renderJsonSummary(stats);
    selectJsonNode('$', jsonState.parsed);
    runJsonSearch();
}

function showJsonView(viewName) {
    document.querySelectorAll('[data-json-view]').forEach((button) => {
        button.classList.toggle('active', button.dataset.jsonView === viewName);
    });
    document.querySelectorAll('.json-tool-panel').forEach((panel) => {
        panel.classList.toggle('json-tool-panel-active', panel.id === `json-panel-${viewName}`);
    });
}

function renderJsonTree(openDepth) {
    jsonTreeView.textContent = '';
    const count = { value: 0, limited: false };
    jsonTreeView.appendChild(createJsonNode(jsonState.parsed, '$', 'root', 0, openDepth, count));
    if (count.limited) {
        const warning = document.createElement('div');
        warning.className = 'json-tree-limit';
        warning.textContent = `Tree rendering stopped at ${JSON_NODE_LIMIT} nodes. Use search or inspect a smaller branch.`;
        jsonTreeView.appendChild(warning);
    }
}

function createJsonNode(value, path, label, depth, openDepth, count) {
    count.value += 1;
    if (count.value > JSON_NODE_LIMIT) {
        count.limited = true;
        const limited = document.createElement('div');
        limited.className = 'json-tree-limited-node';
        limited.textContent = '... node limit reached';
        return limited;
    }

    const node = document.createElement('div');
    node.className = 'json-node';
    node.dataset.path = path;
    node.dataset.searchText = `${label} ${primitivePreview(value)} ${path}`.toLowerCase();

    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'json-node-row';
    row.style.paddingLeft = `${depth * 14}px`;

    const isContainer = isObject(value) || Array.isArray(value);
    const marker = document.createElement('span');
    marker.className = 'json-node-marker';
    marker.textContent = isContainer ? (depth < openDepth ? '▾' : '▸') : '•';

    const key = document.createElement('span');
    key.className = 'json-node-key';
    key.textContent = label;

    const type = document.createElement('span');
    type.className = 'json-node-type';
    type.textContent = describeNode(value);

    const preview = document.createElement('span');
    preview.className = 'json-node-preview';
    preview.textContent = isContainer ? '' : primitivePreview(value);

    row.append(marker, key, type, preview);
    node.appendChild(row);

    const children = document.createElement('div');
    children.className = 'json-node-children';
    children.hidden = !(isContainer && depth < openDepth);
    node.appendChild(children);

    row.addEventListener('click', (event) => {
        event.stopPropagation();
        selectJsonNode(path, value);
        if (isContainer) {
            children.hidden = !children.hidden;
            marker.textContent = children.hidden ? '▸' : '▾';
        }
    });

    if (isContainer) {
        const entries = Array.isArray(value)
            ? value.map((item, index) => [index, item])
            : Object.entries(value);
        entries.forEach(([childKey, childValue]) => {
            const childPath = Array.isArray(value)
                ? `${path}[${childKey}]`
                : `${path}.${safePathKey(childKey)}`;
            children.appendChild(createJsonNode(childValue, childPath, String(childKey), depth + 1, openDepth, count));
        });
    }

    return node;
}

function selectJsonNode(path, value) {
    jsonState.selectedPath = path;
    jsonState.selectedValue = value;
    document.querySelectorAll('.json-node-row.selected').forEach((row) => row.classList.remove('selected'));
    const node = jsonTreeView.querySelector(`[data-path="${cssEscape(path)}"] > .json-node-row`);
    if (node) node.classList.add('selected');

    detailPath.textContent = path;
    detailType.textContent = nodeType(value);
    detailSize.textContent = describeNode(value);
    detailPreview.textContent = formatJsonValue(value, true);
    renderTablePreview(value);
}

function runJsonSearch() {
    const query = jsonSearchInput.value.trim().toLowerCase();
    document.querySelectorAll('.json-node-search-match').forEach((row) => row.classList.remove('json-node-search-match'));
    jsonState.matches = [];
    jsonState.matchIndex = -1;

    if (!query) {
        jsonSearchCount.textContent = '0 matches';
        return;
    }

    jsonState.matches = Array.from(jsonTreeView.querySelectorAll('.json-node'))
        .filter((node) => node.dataset.searchText.includes(query));
    jsonSearchCount.textContent = `${jsonState.matches.length} matches`;
    if (jsonState.matches.length > 0) stepJsonSearch(1);
}

function stepJsonSearch(direction) {
    if (jsonState.matches.length === 0) return;
    jsonState.matchIndex = (jsonState.matchIndex + direction + jsonState.matches.length) % jsonState.matches.length;
    document.querySelectorAll('.json-node-search-current').forEach((row) => row.classList.remove('json-node-search-current'));
    const node = jsonState.matches[jsonState.matchIndex];
    expandParents(node);
    const row = node.querySelector(':scope > .json-node-row');
    row.classList.add('json-node-search-match', 'json-node-search-current');
    row.scrollIntoView({ block: 'center' });
}

function expandParents(node) {
    let current = node.parentElement;
    while (current && current !== jsonTreeView) {
        if (current.classList.contains('json-node-children')) {
            current.hidden = false;
            const marker = current.parentElement?.querySelector(':scope > .json-node-row .json-node-marker');
            if (marker) marker.textContent = '▾';
        }
        current = current.parentElement;
    }
}

function renderJsonSummary(stats) {
    const root = jsonState.parsed;
    const lines = [
        ['Root', describeNode(root)],
        ['Total nodes', String(stats.nodes)],
        ['Max depth', String(stats.maxDepth)],
        ['Objects', String(stats.objects)],
        ['Arrays', String(stats.arrays)],
        ['Strings', String(stats.strings)],
        ['Numbers', String(stats.numbers)],
        ['Booleans', String(stats.booleans)],
        ['Nulls', String(stats.nulls)],
    ];
    jsonSummaryView.innerHTML = lines
        .map(([key, value]) => `<div><span>${escapeHtml(key)}</span><strong>${escapeHtml(value)}</strong></div>`)
        .join('');
}

function renderTablePreview(value) {
    jsonTableView.textContent = '';
    jsonState.csv = '';
    jsonCopyCsvBtn.disabled = true;

    if (!Array.isArray(value) || !value.every(isObject)) {
        jsonTableStatus.textContent = 'Select an array of objects in the tree.';
        return;
    }

    const rows = value.slice(0, 100);
    const columns = Array.from(new Set(rows.flatMap((row) => Object.keys(row)))).slice(0, 24);
    if (columns.length === 0) {
        jsonTableStatus.textContent = `Array has ${value.length} objects with no enumerable keys.`;
        return;
    }

    jsonTableStatus.textContent = `Showing ${rows.length} of ${value.length} rows and ${columns.length} columns.`;
    const table = document.createElement('table');
    const thead = document.createElement('thead');
    const headRow = document.createElement('tr');
    columns.forEach((column) => {
        const th = document.createElement('th');
        th.textContent = column;
        headRow.appendChild(th);
    });
    thead.appendChild(headRow);
    table.appendChild(thead);

    const tbody = document.createElement('tbody');
    rows.forEach((row) => {
        const tr = document.createElement('tr');
        columns.forEach((column) => {
            const td = document.createElement('td');
            td.textContent = cellPreview(row[column]);
            tr.appendChild(td);
        });
        tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    jsonTableView.appendChild(table);

    jsonState.csv = [
        columns.map(csvEscape).join(','),
        ...rows.map((row) => columns.map((column) => csvEscape(cellPreview(row[column]))).join(',')),
    ].join('\n');
    jsonCopyCsvBtn.disabled = false;
}

function summarizeJson(value, depth = 0, stats = null) {
    const current = stats || {
        nodes: 0, objects: 0, arrays: 0, strings: 0, numbers: 0, booleans: 0, nulls: 0, maxDepth: 0,
    };
    current.nodes += 1;
    current.maxDepth = Math.max(current.maxDepth, depth);

    if (Array.isArray(value)) {
        current.arrays += 1;
        value.forEach((item) => summarizeJson(item, depth + 1, current));
    } else if (isObject(value)) {
        current.objects += 1;
        Object.values(value).forEach((item) => summarizeJson(item, depth + 1, current));
    } else if (typeof value === 'string') {
        current.strings += 1;
    } else if (typeof value === 'number') {
        current.numbers += 1;
    } else if (typeof value === 'boolean') {
        current.booleans += 1;
    } else if (value === null) {
        current.nulls += 1;
    }
    return current;
}

function describeNode(value) {
    if (Array.isArray(value)) return `array · ${value.length} items`;
    if (isObject(value)) return `object · ${Object.keys(value).length} keys`;
    if (typeof value === 'string') return `string · ${value.length} chars`;
    if (value === null) return 'null';
    return typeof value;
}

function nodeType(value) {
    if (Array.isArray(value)) return 'array';
    if (value === null) return 'null';
    return typeof value;
}

function primitivePreview(value) {
    if (typeof value === 'string') return JSON.stringify(value.length > 120 ? value.slice(0, 120) + '...' : value);
    if (value === null || typeof value !== 'object') return String(value);
    return '';
}

function cellPreview(value) {
    if (value === null || value === undefined) return '';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
}

function formatJsonValue(value, pretty) {
    if (value === undefined) return '';
    if (typeof value === 'string') return value;
    return JSON.stringify(value, null, pretty ? 2 : 0);
}

function safePathKey(key) {
    return /^[A-Za-z_$][\w$]*$/.test(key) ? key : `[${JSON.stringify(key)}]`;
}

function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function cssEscape(value) {
    if (window.CSS?.escape) return CSS.escape(value);
    return value.replace(/"/g, '\\"');
}

function csvEscape(value) {
    const text = String(value ?? '');
    return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function copyText(text) {
    navigator.clipboard?.writeText(text).catch(() => {
        const tmp = document.createElement('textarea');
        tmp.value = text;
        document.body.appendChild(tmp);
        tmp.select();
        document.execCommand('copy');
        tmp.remove();
    });
}

function inspectLocation() {
    const text = contentInput.value;
    const lineNum = parseInt(lineInput.value, 10);
    const colNum = parseInt(colInput.value, 10);

    if (!text) {
        alert('Please provide some content.');
        return;
    }

    const lines = text.split(/\r\n|\r|\n/);
    if (lineNum < 1 || lineNum > lines.length) {
        alert(`Line number must be between 1 and ${lines.length}`);
        return;
    }

    const line = lines[lineNum - 1];
    if (colNum < 1 || colNum > line.length + 1) {
        alert(`Column number must be between 1 and ${line.length + 1} for this line.`);
        return;
    }

    const charIndex = colNum - 1;
    const targetChar = line[charIndex] !== undefined ? line[charIndex] : '(EOL)';
    const start = Math.max(0, charIndex - 30);
    const end = Math.min(line.length, charIndex + 30);

    let contextHtml = '';
    if (start > 0) contextHtml += '<span style="color:#777">...</span>';
    contextHtml += escapeHtml(line.substring(start, charIndex));
    let displayChar = targetChar;
    if (displayChar === ' ') displayChar = '&nbsp;';
    if (displayChar === '\t') displayChar = '\\t';
    if (displayChar === '(EOL)') displayChar = '⏎';
    contextHtml += `<span class="highlight-char">${displayChar}</span>`;
    contextHtml += escapeHtml(line.substring(charIndex + 1, end));
    if (end < line.length) contextHtml += '<span style="color:#777">...</span>';

    document.getElementById('context-view').innerHTML = contextHtml;
    document.getElementById('char-preview').innerText = `"${targetChar}"`;
    if (line[charIndex]) {
        const code = line.charCodeAt(charIndex);
        const hex = code.toString(16).toUpperCase().padStart(4, '0');
        document.getElementById('char-code').innerText = `U+${hex} (Dec: ${code})`;
    } else {
        document.getElementById('char-code').innerText = 'End of Line (Newline)';
    }
    document.getElementById('line-context').innerText = `Line length: ${line.length} characters`;
    resultSection.style.display = 'block';
    resultSection.scrollIntoView({ behavior: 'smooth' });
}

function escapeHtml(text) {
    if (!text) return '';
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#039;');
}

window.detectContent = detectContent;
window.toggleWrap = toggleWrap;
window.prettifyContent = prettifyContent;
window.inspectLocation = inspectLocation;

loadPendingInspectorPayload();
detectContent();
