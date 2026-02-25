use actix_web::{get, web::Data, HttpResponse, Responder};
use std::{sync::Arc, collections::HashMap};

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

#[get("/inspector")]
pub async fn inspector_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_inspector_page(&current_theme, &saved_themes))
}

fn render_inspector_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let content = r#"
    <div class="inspector-container">
        <h1>Line & Column Inspector</h1>
        
        <div class="input-section">
            <div class="toolbar">
                <div class="form-group">
                    <label>Load File</label>
                    <input type="file" id="file-input">
                </div>
                
                <div style="flex-grow: 1;"></div>
                
                <!-- Format Indicator -->
                <div id="type-indicator" class="indicator">Text</div>

                <div class="checkbox-group">
                    <input type="checkbox" id="wrap-toggle" onchange="toggleWrap()">
                    <label for="wrap-toggle" style="font-weight: normal; cursor: pointer;">Word Wrap</label>
                </div>

                <button class="toolbar-action-btn" onclick="formatSQL()">Prettify SQL</button>
                <button class="toolbar-action-btn" onclick="formatJSON()">Prettify JSON</button>
            </div>
            
            <textarea id="content-input" placeholder="Paste JSON, XML, or Text content here..." oninput="detectContent()"></textarea>
            
            <div class="control-row">
                <div class="form-group">
                    <label>Line Number</label>
                    <input type="number" id="line-num" min="1" value="1" placeholder="e.g. 50">
                </div>
                <div class="form-group">
                    <label>Column Number</label>
                    <input type="number" id="col-num" min="1" value="1" placeholder="e.g. 12">
                </div>
                <button class="control-action-btn" onclick="inspectLocation()">Inspect Location</button>
            </div>
        </div>
        
        <div id="result-section" class="result-section">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:10px;">
                <h2 style="margin:0;">Inspection Result</h2>
                <button onclick="document.getElementById('result-section').style.display='none'" style="padding:5px 10px;">x</button>
            </div>
            <div id="context-view" class="context-display"></div>
            
            <div class="char-info">
                <span class="info-label">Character:</span>
                <span id="char-preview" style="font-weight: bold; font-family: monospace;"></span>
                
                <span class="info-label">Unicode/Hex:</span>
                <span id="char-code"></span>
                
                <span class="info-label">Line Context:</span>
                <span id="line-context"></span>
            </div>
        </div>
    </div>

    <script>
        const contentInput = document.getElementById('content-input');
        const fileInput = document.getElementById('file-input');
        const lineInput = document.getElementById('line-num');
        const colInput = document.getElementById('col-num');
        const resultSection = document.getElementById('result-section');
        const indicator = document.getElementById('type-indicator');
        
        // Handle File Upload
        fileInput.addEventListener('change', (e) => {
            const file = e.target.files[0];
            if (!file) return;
            
            const reader = new FileReader();
            reader.onload = (e) => {
                contentInput.value = e.target.result;
                detectContent();
            };
            reader.readAsText(file);
        });

        // Detect content type (JSON, XML, or Text)
        function detectContent() {
            const text = contentInput.value.trim();
            
            if (text.length === 0) {
                indicator.textContent = "Empty";
                indicator.className = "indicator";
                return;
            }
            
            // Check JSON
            if ((text.startsWith('{') || text.startsWith('[')) && isValidJSON(text)) {
                indicator.textContent = "Valid JSON";
                indicator.className = "indicator valid-json";
                return;
            }
            
            // Check XML
            if (text.startsWith('<') && isValidXML(text)) {
                indicator.textContent = "Valid XML";
                indicator.className = "indicator valid-xml";
                return;
            }
            
            indicator.textContent = "Plain Text";
            indicator.className = "indicator";
        }

        function isValidJSON(text) {
            try {
                JSON.parse(text);
                return true;
            } catch (e) {
                return false;
            }
        }

        function isValidXML(text) {
            try {
                const parser = new DOMParser();
                const doc = parser.parseFromString(text, "application/xml");
                // DOMParser returns a document with a parsererror tag if invalid
                return !doc.querySelector("parsererror");
            } catch (e) {
                return false;
            }
        }
        
        function toggleWrap() {
            if (document.getElementById('wrap-toggle').checked) {
                contentInput.style.whiteSpace = 'pre-wrap';
            } else {
                contentInput.style.whiteSpace = 'pre';
            }
        }
        
        function formatJSON() {
            try {
                const val = contentInput.value;
                if (!val) return;
                const json = JSON.parse(val);
                contentInput.value = JSON.stringify(json, null, 4);
                detectContent();
            } catch (e) {
                alert("Invalid JSON: " + e.message);
            }
        }

        function mapOutsideSqlLiterals(sql, mapper) {
            let output = '';
            let outside = '';
            let mode = null; // "'", '"' or '`'

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

                if (ch === mode && sql[i - 1] !== '\\') {
                    mode = null;
                }
            }

            output += mapper(outside);
            return output;
        }

        function formatSQL() {
            const val = contentInput.value;
            if (!val || !val.trim()) return;

            const normalizeOutside = (segment) => segment.replace(/\s+/g, ' ');
            let formatted = mapOutsideSqlLiterals(val, normalizeOutside).trim();

            const clauseBreaks = [
                'WITH', 'SELECT', 'FROM', 'WHERE', 'GROUP BY', 'ORDER BY',
                'HAVING', 'LIMIT', 'OFFSET', 'INSERT INTO', 'VALUES',
                'UPDATE', 'SET', 'DELETE FROM', 'UNION ALL', 'UNION'
            ];

            for (const clause of clauseBreaks) {
                const escaped = clause.replace(/\s+/g, '\\s+');
                const re = new RegExp(`\\b${escaped}\\b`, 'gi');
                formatted = mapOutsideSqlLiterals(formatted, (segment) =>
                    segment.replace(re, `\n${clause}`)
                );
            }

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

            formatted = formatted
                .replace(/\n{2,}/g, '\n')
                .replace(/[ \t]+\n/g, '\n')
                .trim();

            contentInput.value = formatted;
            detectContent();
        }
        
        function inspectLocation() {
            const text = contentInput.value;
            let lineNum = parseInt(lineInput.value);
            let colNum = parseInt(colInput.value);
            
            if (!text) {
                alert("Please provide some content.");
                return;
            }
            
            // Standardize line splits (handle Windows \r\n, Mac \r, Linux \n)
            const lines = text.split(/\r\n|\r|\n/);
            
            if (lineNum < 1 || lineNum > lines.length) {
                alert(`Line number must be between 1 and ${lines.length}`);
                return;
            }
            
            const line = lines[lineNum - 1];
            
            if (colNum < 1 || colNum > line.length + 1) { // +1 allows for End of Line detection
                alert(`Column number must be between 1 and ${line.length + 1} for this line.`);
                return;
            }
            
            // Logic to display context
            const charIndex = colNum - 1;
            const targetChar = line[charIndex] !== undefined ? line[charIndex] : '(EOL)';
            
            // Generate Context HTML
            // We'll show ~40 chars around the target
            const start = Math.max(0, charIndex - 30);
            const end = Math.min(line.length, charIndex + 30);
            
            let contextHtml = '';
            
            // Pre-text
            if (start > 0) contextHtml += '<span style="color:#777">...</span>';
            contextHtml += escapeHtml(line.substring(start, charIndex));
            
            // Target Char
            let displayChar = targetChar;
            if (displayChar === ' ') displayChar = '&nbsp;';
            if (displayChar === '\t') displayChar = '\\t'; 
            if (displayChar === '(EOL)') displayChar = '⏎';
            
            contextHtml += `<span class="highlight-char">${displayChar}</span>`;
            
            // Post-text
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
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;")
                .replace(/"/g, "&quot;")
                .replace(/'/g, "&#039;");
        }
    </script>
    "#;

    render_base_page("Inspector", content, current_theme, saved_themes)
}
