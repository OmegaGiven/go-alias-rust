use actix_web::{HttpResponse, Responder, get, web::Data};
use std::{collections::HashMap, sync::Arc};

use crate::app_state::{AppState, Theme};
use crate::base_page::{render_base_page, static_asset};

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
    <div class="inspector-page">
        <div class="inspector-container">
            <div class="input-section">
            <div class="inspector-toolbar">
                <label class="inspector-file-btn" for="file-input">Open File</label>
                <input type="file" id="file-input">
                <div id="type-indicator" class="indicator">Text</div>
                <label class="inspector-wrap-toggle" for="wrap-toggle">
                    <input type="checkbox" id="wrap-toggle" onchange="toggleWrap()">
                    <span>Wrap</span>
                </label>
                <button id="prettify-btn" class="toolbar-action-btn" onclick="prettifyContent()" disabled>Prettify</button>
                <div class="inspector-location-strip">
                    <label for="line-num">Line</label>
                    <input type="number" id="line-num" min="1" value="1" placeholder="50">
                    <label for="col-num">Column</label>
                    <input type="number" id="col-num" min="1" value="1" placeholder="12">
                    <button class="control-action-btn" onclick="inspectLocation()">Inspect Location</button>
                </div>
            </div>
            <div id="inspector-source-meta" class="inspector-source-meta" hidden></div>
            
            <textarea id="content-input" placeholder="Paste JSON, XML, or Text content here..." oninput="detectContent()"></textarea>
            <div id="inspector-input-resizer" class="inspector-splitter inspector-horizontal-splitter" title="Drag to resize source input"></div>

            <section id="json-tools" class="json-tools" hidden>
                <div class="json-tools-header">
                    <div>
                        <h2>JSON Inspector</h2>
                        <div id="json-summary-line" class="json-summary-line">Paste valid JSON to inspect structure.</div>
                    </div>
                    <div class="json-tools-actions">
                        <button type="button" id="json-view-raw-btn" class="json-tool-btn active" data-json-view="raw">Raw</button>
                        <button type="button" id="json-view-tree-btn" class="json-tool-btn" data-json-view="tree">Tree</button>
                        <button type="button" id="json-view-summary-btn" class="json-tool-btn" data-json-view="summary">Summary</button>
                        <button type="button" id="json-view-table-btn" class="json-tool-btn" data-json-view="table">Table</button>
                    </div>
                </div>
                <div class="json-tool-panel json-tool-panel-active" id="json-panel-raw">
                    <pre id="json-raw-preview"></pre>
                </div>
                <div class="json-tool-panel" id="json-panel-tree">
                    <div class="json-tree-toolbar">
                        <button type="button" id="json-expand-one-btn">Expand One Level</button>
                        <button type="button" id="json-expand-all-btn">Expand All</button>
                        <button type="button" id="json-collapse-all-btn">Collapse All</button>
                        <input type="search" id="json-search-input" placeholder="Search keys or values">
                        <button type="button" id="json-search-prev-btn">Prev</button>
                        <button type="button" id="json-search-next-btn">Next</button>
                        <span id="json-search-count">0 matches</span>
                    </div>
                    <div class="json-tree-layout">
                        <div id="json-tree-view" class="json-tree-view"></div>
                        <div id="json-detail-resizer" class="inspector-splitter inspector-vertical-splitter" title="Drag to resize selected node details"></div>
                        <aside class="json-node-details">
                            <h3>Selected Node</h3>
                            <dl>
                                <dt>Path</dt><dd id="json-detail-path">-</dd>
                                <dt>Type</dt><dd id="json-detail-type">-</dd>
                                <dt>Size</dt><dd id="json-detail-size">-</dd>
                            </dl>
                            <pre id="json-detail-preview"></pre>
                            <div class="json-detail-actions">
                                <button type="button" id="json-copy-path-btn">Copy Path</button>
                                <button type="button" id="json-copy-value-btn">Copy Value</button>
                                <button type="button" id="json-copy-pretty-btn">Copy Pretty</button>
                            </div>
                        </aside>
                    </div>
                </div>
                <div class="json-tool-panel" id="json-panel-summary">
                    <div id="json-summary-view" class="json-summary-view"></div>
                </div>
                <div class="json-tool-panel" id="json-panel-table">
                    <div class="json-table-toolbar">
                        <button type="button" id="json-copy-csv-btn" disabled>Copy CSV</button>
                        <span id="json-table-status">Select an array of objects in the tree.</span>
                    </div>
                    <div id="json-table-view" class="json-table-view"></div>
                </div>
            </section>
        </div>
        
            <div id="result-section" class="result-section">
                <div style="display:flex; justify-content:space-between; align-items:center; margin: calc(var(--element-margin) / 2) var(--element-margin);">
                    <h2 style="margin: calc(var(--element-margin) / 2) var(--element-margin);">Inspection Result</h2>
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
    </div>
    <script src="{inspector_js}" defer></script>
    "#;
    let content = content.replace("{inspector_js}", &static_asset("inspector.js"));

    render_base_page(
        "Inspector",
        &format!(
            r#"<link rel="stylesheet" href="{}">{}"#,
            static_asset("inspector.css"),
            content
        ),
        current_theme,
        saved_themes,
    )
}
