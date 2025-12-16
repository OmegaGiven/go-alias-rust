use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

#[get("/inspector")]
pub async fn inspector_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_inspector_page(&current_theme))
}

fn render_inspector_page(current_theme: &Theme) -> String {
    let style = r#"
<style>
    .inspector-container {
        display: flex;
        flex-direction: column;
        gap: 20px;
        max-width: 900px;
        margin: 0 auto;
        padding: 20px;
        height: calc(100vh - 100px);
    }
    
    /* SCOPED: Only affects elements INSIDE the inspector container */
    
    .inspector-container .input-section {
        background: var(--secondary-bg);
        padding: 20px;
        border-radius: 0; 
        border: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        flex-grow: 1;
        overflow: hidden;
    }
    
    .inspector-container .toolbar {
        display: flex;
        gap: 15px;
        margin-bottom: 10px;
        align-items: center;
        flex-wrap: wrap;
    }
    
    .inspector-container .control-row {
        display: flex;
        gap: 15px;
        margin-top: 15px;
        align-items: end;
        flex-wrap: wrap;
        border-top: 1px solid var(--border-color);
        padding-top: 15px;
    }
    
    .inspector-container .form-group {
        display: flex;
        flex-direction: column;
        gap: 5px;
    }
    
    .inspector-container label { 
        font-weight: bold; 
        font-size: 0.9em; 
        color: var(--text-color); 
    }
    
    .inspector-container input[type="number"], 
    .inspector-container input[type="file"] {
        padding: 8px;
        background: var(--primary-bg);
        border: 1px solid var(--border-color);
        color: var(--text-color);
        border-radius: 0; 
    }
    
    .inspector-container textarea {
        flex-grow: 1;
        width: 100%;
        background: var(--primary-bg);
        border: 1px solid var(--border-color);
        color: var(--text-color);
        border-radius: 0;
        padding: 10px;
        font-family: monospace;
        resize: none;
        box-sizing: border-box;
        white-space: pre; 
        overflow: auto;
    }
    
    /* This specific selector prevents the button styles from hitting the Nav Bar */
    .inspector-container button {
        padding: 8px 16px;
        background: var(--tertiary-bg);
        color: var(--text-color);
        border: 1px solid var(--border-color);
        border-radius: 0; /* Sharp corners for inspector only */
        cursor: pointer;
        font-weight: bold;
    }
    .inspector-container button:hover { 
        background: var(--link-hover); 
        color: #fff; 
        border-color: var(--link-hover); 
    }
    
    .inspector-container .result-section {
        background: var(--secondary-bg);
        padding: 20px;
        border-radius: 0;
        border: 1px solid var(--border-color);
        display: none;
        flex-shrink: 0;
    }
    
    .inspector-container .context-display {
        font-family: monospace;
        background: var(--primary-bg);
        padding: 15px;
        border-radius: 0;
        overflow-x: auto;
        white-space: pre;
        margin-bottom: 10px;
        border: 1px solid var(--border-color);
        font-size: 1.1em;
        line-height: 1.5;
    }
    
    .inspector-container .highlight-char {
        background-color: #ff4444;
        color: white;
        font-weight: bold;
        padding: 2px 4px;
        border-radius: 0;
        border: 1px solid #cc0000;
    }
    
    .inspector-container .char-info {
        display: grid;
        grid-template-columns: auto 1fr;
        gap: 10px;
        font-size: 0.9em;
        background: var(--tertiary-bg);
        padding: 10px;
        border-radius: 0;
    }
    .inspector-container .info-label { color: #888; font-weight: bold; text-align: right;}
    
    .inspector-container .checkbox-group {
        display: flex;
        align-items: center;
        gap: 5px;
        font-size: 0.9em;
    }
</style>
    "#;

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
                
                <div class="checkbox-group">
                    <input type="checkbox" id="wrap-toggle" onchange="toggleWrap()">
                    <label for="wrap-toggle" style="font-weight: normal; cursor: pointer;">Word Wrap</label>
                </div>

                <button onclick="formatJSON()">Prettify JSON</button>
            </div>
            
            <textarea id="content-input" placeholder="Paste JSON, XML, or Text content here..."></textarea>
            
            <div class="control-row">
                <div class="form-group">
                    <label>Line Number</label>
                    <input type="number" id="line-num" min="1" value="1" placeholder="e.g. 50">
                </div>
                <div class="form-group">
                    <label>Column Number</label>
                    <input type="number" id="col-num" min="1" value="1" placeholder="e.g. 12">
                </div>
                <button onclick="inspectLocation()" style="align-self: flex-end; margin-bottom: 2px;">Inspect Location</button>
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
        
        // Handle File Upload
        fileInput.addEventListener('change', (e) => {
            const file = e.target.files[0];
            if (!file) return;
            
            const reader = new FileReader();
            reader.onload = (e) => {
                contentInput.value = e.target.result;
            };
            reader.readAsText(file);
        });
        
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
            } catch (e) {
                alert("Invalid JSON: " + e.message);
            }
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

    render_base_page("Inspector", &format!("{}{}", style, content), current_theme)
}