use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::{Deserialize, Serialize};
use std::{fs, io::{self, Write}, sync::Arc, path::Path};
use serde_json;

use crate::app_state::{AppState, Theme, Note};
use crate::base_page::render_base_page;

static NOTES_FILE: &str = "notes.json";
static BOOKMARKS_FILE: &str = "fs_bookmarks.json";

#[derive(Deserialize)]
pub struct NoteForm {
    pub subject: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct _DeleteForm {
    pub note_index: usize,
}

// --- NEW: File System Structs ---
#[derive(Deserialize)]
pub struct LsForm {
    pub path: String,
    pub show_hidden: bool,
}

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
}

#[derive(Serialize)]
pub struct LsResponse {
    pub current_path: String,
    pub entries: Vec<FileEntry>,
}

#[derive(Deserialize)]
pub struct ReadForm {
    pub path: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct SaveFileForm {
    pub path: String,
    pub content: String,
}


#[derive(Deserialize)]
pub struct SearchForm {
    pub path: String,
    pub query: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub name: String,
    pub path: String,
}
// --------------------------------

pub fn save_notes(notes: &[Note]) -> io::Result<()> {
    let json = serde_json::to_string(notes)?;
    let mut f = fs::File::create(NOTES_FILE)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

fn load_bookmarks() -> Vec<Bookmark> {
    if Path::new(BOOKMARKS_FILE).exists() {
        let data = fs::read_to_string(BOOKMARKS_FILE).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_bookmarks(bookmarks: &[Bookmark]) -> io::Result<()> {
    let json = serde_json::to_string(bookmarks)?;
    let mut f = fs::File::create(BOOKMARKS_FILE)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

#[get("/note")]
pub async fn note_get(state: Data<Arc<AppState>>) -> impl Responder {
    let notes = state.notes.lock().unwrap().clone();
    let current_theme = state.current_theme.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_note_page(&notes, &current_theme))
}

#[post("/note")]
pub async fn note_post(
    state: Data<Arc<AppState>>,
    form: web::Form<NoteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    
    let subject = form.subject.trim();
    let content = form.content.trim();
    
    if subject.is_empty() && content.is_empty() {
        return HttpResponse::SeeOther()
            .append_header(("Location", "/note"))
            .finish();
    }
    
    let final_subject = if subject.is_empty() {
        content.chars().take(30).collect::<String>().trim().to_string()
    } else {
        subject.to_string()
    };

    let new_note = Note {
        subject: final_subject.clone(),
        content: content.to_string(),
    };

    let existing_index = notes.iter().position(|n| n.subject == final_subject);

    match existing_index {
        Some(index) => {
            notes.remove(index);
            notes.insert(index, new_note);
            println!("Note updated: {}", final_subject);
        }
        None => {
            notes.push(new_note);
            println!("New note saved: {}", final_subject);
        }
    }
    
    let notes_to_save = notes.clone();
    web::block(move || save_notes(&notes_to_save)).await.ok();
    
    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}

#[post("/note/delete")]
pub async fn note_delete(
    state: Data<Arc<AppState>>,
    form: web::Form<_DeleteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    let index = form.note_index;

    if index < notes.len() {
        notes.remove(index);
        let notes_to_save = notes.clone();
        web::block(move || save_notes(&notes_to_save)).await.ok();
        println!("Note deleted at index: {}", index);
    } else {
        eprintln!("Attempted to delete note with out-of-bounds index: {}", index);
    }

    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}

// --- NEW: File System Handlers ---
#[post("/note/ls")]
pub async fn note_ls(form: web::Json<LsForm>) -> impl Responder {
    let res = web::block(move || {
        // Determine the path to read. Default to current working directory if "." or empty.
        let path_to_read = if form.path.is_empty() || form.path == "." {
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
        } else {
            Path::new(&form.path).to_path_buf()
        };

        // Canonicalize to get the absolute path (resolves symlinks, .., etc.)
        let absolute_path = match fs::canonicalize(&path_to_read) {
            Ok(p) => p,
            Err(_) => path_to_read.clone(), // Fallback if canonicalize fails
        };

        let current_path_str = absolute_path.to_string_lossy().to_string();
        let mut entries = Vec::new();
        
        if let Ok(read_dir) = fs::read_dir(&absolute_path) {
            for entry in read_dir.flatten() {
                if let Ok(meta) = entry.metadata() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    
                    if !form.show_hidden && file_name.starts_with('.') {
                        continue;
                    }

                    let is_dir = meta.is_dir();
                    // Construct the full absolute path for this entry
                    let full_path = absolute_path.join(&file_name).to_string_lossy().to_string();
                    
                    entries.push(FileEntry {
                        name: file_name,
                        is_dir,
                        path: full_path,
                    });
                }
            }
        }
        
        // Sort: Directories first, then alphabetical
        entries.sort_by(|a, b| {
            if a.is_dir == b.is_dir {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            } else {
                b.is_dir.cmp(&a.is_dir)
            }
        });

        Ok::<LsResponse, io::Error>(LsResponse {
            current_path: current_path_str,
            entries,
        })
    }).await;

    match res {
        Ok(Ok(data)) => HttpResponse::Ok().json(data),
        _ => HttpResponse::InternalServerError().body("Error listing directory"),
    }
}


#[post("/note/read")]
pub async fn note_read(form: web::Json<ReadForm>) -> impl Responder {
    let path = form.path.clone();
    let res = web::block(move || fs::read_to_string(&path)).await;
    match res {
        Ok(Ok(content)) => HttpResponse::Ok().body(content),
        Ok(Err(e)) => HttpResponse::BadRequest().body(format!("Error reading file: {}", e)),
        _ => HttpResponse::InternalServerError().body("Blocked error"),
    }
}


#[post("/note/save_file")]
pub async fn note_save_file(form: web::Json<SaveFileForm>) -> impl Responder {
    let path = form.path.clone();
    let content = form.content.clone();
    let res = web::block(move || fs::write(&path, &content)).await;
    match res {
        Ok(Ok(_)) => HttpResponse::Ok().body("File saved successfully"),
        Ok(Err(e)) => HttpResponse::BadRequest().body(format!("Error saving file: {}", e)),
        _ => HttpResponse::InternalServerError().finish(),
    }
}


// Helper for recursive search
fn recursive_search(dir: &Path, query: &str, results: &mut Vec<FileEntry>, count: &mut usize) {
    if *count >= 50 { return; } // Hard limit to prevent hanging
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            // Skip hidden files/dirs for search optimization
            if file_name.starts_with('.') { continue; }

            let is_dir = path.is_dir();
            
            if file_name.to_lowercase().contains(&query.to_lowercase()) {
                results.push(FileEntry {
                    name: file_name.to_string(),
                    is_dir,
                    path: path.to_string_lossy().to_string(),
                });
                *count += 1;
            }
            
            if *count >= 50 { return; }

            if is_dir {
                recursive_search(&path, query, results, count);
            }
        }
    }
}

#[post("/note/search")]
pub async fn note_search(form: web::Json<SearchForm>) -> impl Responder {
    let res = web::block(move || {
        let start_path = if form.path.is_empty() || form.path == "." {
            std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf())
        } else {
            Path::new(&form.path).to_path_buf()
        };
        
        let mut results = Vec::new();
        let mut count = 0;
        
        recursive_search(&start_path, &form.query, &mut results, &mut count);
        
        Ok::<LsResponse, io::Error>(LsResponse {
            current_path: start_path.to_string_lossy().to_string(),
            entries: results,
        })
    }).await;

    match res {
        Ok(Ok(data)) => HttpResponse::Ok().json(data),
        _ => HttpResponse::InternalServerError().body("Search error"),
    }
}


#[get("/note/bookmarks")]
pub async fn note_bookmarks_get() -> impl Responder {
    let bookmarks = load_bookmarks();
    HttpResponse::Ok().json(bookmarks)
}

#[post("/note/bookmarks/add")]
pub async fn note_bookmark_add(form: web::Json<Bookmark>) -> impl Responder {
    let mut bookmarks = load_bookmarks();
    // Check if exists
    if !bookmarks.iter().any(|b| b.path == form.path) {
        bookmarks.push(form.into_inner());
        let _ = save_bookmarks(&bookmarks);
    }
    HttpResponse::Ok().finish()
}

#[post("/note/bookmarks/delete")]
pub async fn note_bookmark_delete(form: web::Json<Bookmark>) -> impl Responder {
    let mut bookmarks = load_bookmarks();
    if let Some(pos) = bookmarks.iter().position(|b| b.path == form.path) {
        bookmarks.remove(pos);
        let _ = save_bookmarks(&bookmarks);
    }
    HttpResponse::Ok().finish()
}
// ---------------------------------

fn render_note_page(notes: &[Note], current_theme: &Theme) -> String {
    let saved_notes_list = notes
        .iter()
        .enumerate()
        .map(|(index, n)| {
            format!(
                r#"
                <li class="saved-note-item">
                    <span class="saved-note" data-index="{index}" data-subject="{subject}" data-content="{content}">
                        {subject_escaped}
                    </span>
                    <form method="POST" action="/note/delete" class="delete-form">
                        <input type="hidden" name="note_index" value="{index}">
                        <button type="submit" class="delete-button" title="Delete this note">×</button>
                    </form>
                </li>
                "#,
                index = index,
                subject = encode_minimal(&n.subject),
                content = encode_minimal(&n.content),
                subject_escaped = encode_minimal(&n.subject),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sidebar_content = format!(r#"
        <div class="tabs">
            <div class="tab active" onclick="switchTab('db')">Saved Notes</div>
            <div class="tab" onclick="switchTab('fs')">Files</div>
        </div>
        
        <!-- Database Tab -->
        <div id="tab-db" class="tab-content active">
            <div class="sidebar-search"><input type="text" id="note-search-input" placeholder="Search notes..."></div>
            <ul id="saved-notes-list" style="list-style: none; padding: 0;">
                {saved_notes_list}
            </ul>
        </div>

        <!-- File System Tab -->
        <div id="tab-fs" class="tab-content">
            <div class="fs-controls">
                <button class="utility-btn" onclick="loadDir('/')" title="Go to System Root">/</button>
                <button class="utility-btn" onclick="goUp()" title="Up Directory">Up</button> 
                <label><input type="checkbox" id="show-hidden-check" onchange="reloadDir()"> Hidden</label>
                <button class="utility-btn" onclick="addBookmark()" title="Bookmark Current Path" style="margin-left: auto;">★</button>
            </div>
            
            <input type="text" id="fs-path-input" class="fs-path-input" value="." onkeypress="if(event.key === 'Enter') loadDir(this.value)">
            <input type="text" id="fs-search-input" class="fs-path-input" placeholder="Search files in current dir..." onkeypress="if(event.key === 'Enter') searchFs(this.value)">

            <!-- Bookmarks Section -->
            <div id="bookmarks-section" style="border-bottom: 1px solid var(--border-color); padding-bottom: 5px; margin-bottom: 5px; display:none;">
                <div style="font-size:0.8em; font-weight:bold; color:#888; margin-bottom:2px;">BOOKMARKS</div>
                <div id="bookmarks-list"></div>
            </div>

            <div id="file-list" style="margin-top: 5px;"></div>
        </div>
    "#, saved_notes_list = saved_notes_list);
    
    let sidebar_html = crate::elements::sidebar::render(&sidebar_content);
    let sidebar_js = crate::elements::sidebar::get_js();

    let page_styles = r#"
<style>
    .note-view-container { display: flex; height: calc(100vh - 60px); position: relative; overflow: hidden; }
    
    /* Sidebar Tabs removed - now using shared .tabs class */
    /* Sidebar Tabs removed - now using shared .tabs class */
    .tab.active { border-bottom: 2px solid var(--link-color); opacity: 1; background: var(--tertiary-bg); }

    .tab-content { flex: 1; overflow-y: auto; padding: 5px; display: none; }
    .tab-content.active { display: flex; }
    
    /* Sidebar search style removed - now in static/style.css */

    /* Saved Notes Styles */
    .saved-note-item { display: flex; align-items: center; justify-content: space-between; background-color: transparent; padding: 2px 5px; cursor: pointer; transition: background 0.2s; }
    .saved-note-item:hover { background-color: var(--tertiary-bg); }
    .saved-note-item.selected { background-color: var(--tertiary-bg); border-left: 3px solid var(--link-color); padding-left: 2px; }
    
    .saved-note { flex-grow: 1; min-width: 0; display: block; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--text-color); font-size: 0.95em; }
    .saved-note:hover { color: var(--link-hover); }
    
    .delete-form { margin: 0; display: flex; align-items: center; margin-left: 5px; }
    .delete-button { background: none; border: none; cursor: pointer; color: var(--text-color); padding: 0 4px; font-size: 1.2em; opacity: 0.3; line-height: 1; }
    .delete-button:hover { color: #ff6b6b; opacity: 1; }

    /* File System Styles */
    .fs-controls { padding: 5px; border-bottom: 1px solid var(--border-color); margin-bottom: 5px; display: flex; gap: 5px; align-items: center;}
    .fs-controls input[type="checkbox"] { margin: 0; }
    .fs-controls label { font-size: 0.8em; user-select: none; cursor: pointer; white-space: nowrap;}
    .fs-controls .utility-btn { font-size: 0.85em; padding: 2px 6px; height: auto; }
    
    .fs-path-input { 
        width: 100%; 
        padding: 4px; 
        font-size: 0.8em; 
        color: var(--text-color); 
        background: var(--primary-bg); 
        border: 1px solid var(--border-color); 
        border-radius: 3px;
        margin-bottom: 5px;
        font-family: monospace;
        box-sizing: border-box;
    }
    
    .file-item { cursor: pointer; padding: 2px 5px; font-size: 0.9em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; border-radius: 3px; display: flex; align-items: center; gap: 5px; }
    .file-item:hover { background-color: var(--tertiary-bg); color: var(--link-hover); }
    .file-icon { width: 16px; text-align: center; display: inline-block; opacity: 0.7; }
    .dir-item { font-weight: bold; color: var(--link-color); }

    /* Bookmarks */
    .bookmark-item { display: flex; align-items: center; justify-content: space-between; font-size: 0.85em; padding: 2px 5px; cursor: pointer; color: var(--text-color); }
    .bookmark-item:hover { background: var(--tertiary-bg); border-radius: 3px; }
    .bookmark-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex-grow: 1; }
    .bookmark-del { margin-left: 5px; opacity: 0.5; font-weight: bold; }
    .bookmark-del:hover { opacity: 1; color: #ff6b6b; }

    /* Main Content */
    #main { flex: 1; display: flex; flex-direction: column; padding: 0; overflow: hidden; background-color: var(--primary-bg); }
    
    /* Editor Toolbar */
    .toolbar { display: flex; gap: 5px; padding: 5px; border-bottom: 1px solid var(--border-color); margin-bottom: 0; align-items: center; flex-wrap: wrap; flex-shrink: 0; background: var(--secondary-bg); }
    .subject-input { flex-grow: 1; padding: 5px 0px; border: 1px solid var(--border-color); background-color: var(--secondary-bg); color: var(--text-color); border-radius: 4px; height: 25px; }
    .utility-btn, .save-db-btn { font-weight: bold; }
    .utility-btn:hover, .save-db-btn:hover { background-color: var(--link-hover); color: white; border-color: var(--link-hover); }
    
    .editor-wrapper { display: flex; flex-direction: column; flex-grow: 1; border: 1px solid var(--border-color); border-radius: 4px; overflow: hidden; min-height: 0; }
    .editor-container { display: flex; flex-grow: 1; overflow: hidden; min-height: 0; }
    
    .line-numbers { 
        background-color: var(--tertiary-bg); 
        color: #777; 
        padding: 10px 5px; 
        text-align: right; 
        user-select: none; 
        overflow: hidden; 
        border-right: 1px solid var(--border-color); 
        flex-shrink: 0; 
        min-width: 35px; 
        box-sizing: border-box; 
        font-family: 'Consolas', 'Monaco', monospace; 
        font-size: 14px; 
        line-height: 20px; 
    }
    
    .input-wrapper { position: relative; flex-grow: 1; height: 100%; overflow: hidden; }
    
    #editor, #highlight-layer {
        position: absolute; top: 0; left: 0; width: 100%; height: 100%;
        padding: 10px; box-sizing: border-box;
        margin: 0; border: none; outline: none;
        font-family: 'Consolas', 'Monaco', monospace; 
        font-size: 14px; 
        line-height: 20px;
        white-space: pre; overflow: auto;
    }
    
    #editor {
        z-index: 2; 
        color: transparent; 
        background: transparent; 
        caret-color: var(--text-color); 
        resize: none;
    }
    
    #highlight-layer {
        z-index: 1; 
        pointer-events: none; 
        color: var(--text-color);
        background: transparent;
    }
    
    #highlight-layer::-webkit-scrollbar { display: none; }
    #highlight-layer { scrollbar-width: none; }
    
    pre code.hljs { background: transparent; padding: 0; }
    
    #markdown-preview { display: none; flex-grow: 1; border: 1px solid var(--border-color); border-radius: 4px; padding: 20px; background-color: var(--secondary-bg); color: var(--text-color); overflow-y: auto; }
    
    #markdown-preview h1, #markdown-preview h2 { border-bottom: 1px solid var(--border-color); padding-bottom: 5px; }
    #markdown-preview code { background: #444; padding: 2px 5px; border-radius: 3px; font-family: 'Consolas', 'Monaco', monospace; }
    #markdown-preview pre { background: #282c34; padding: 10px; border-radius: 5px; overflow-x: auto; }
    #markdown-preview pre code { background: transparent; padding: 0; border-radius: 0; }
    #markdown-preview blockquote { border-left: 3px solid var(--link-color); margin-left: 0; padding-left: 10px; color: #aaa; }
    
    .language-select {
        background-color: var(--secondary-bg);
        color: var(--text-color);
        border: 1px solid var(--border-color);
        border-radius: 4px;
        padding: 0 5px;
        height: 28px;
        font-size: 0.9em;
    }
</style>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/monokai-sublime.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/rust.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/sql.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/javascript.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/json.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/python.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/xml.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/css.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/yaml.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/bash.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/java.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/c.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/cpp.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/go.min.js"></script>
"#;

    let body_content = format!(r#"
    <div class="note-view-container">
        {sidebar_html}
        
        <div id="main">
            <form id="note-form" method="POST" action="/note" style="display: flex; flex-direction: column; height: 100%;">
                <div class="toolbar">
                    <input type="file" id="file-input" style="display: none;" accept=".txt,.md,.json,.rs,.js,.html">
                    <input type="text" id="subject" name="subject" placeholder="Subject / Filename" value="" class="subject-input" autocomplete="off" />
                    
                    <select id="language-select" class="language-select" onchange="updateHighlighting()">
                        <option value="markdown">Markdown</option>
                        <option value="text">Plain Text</option>
                        <option value="rust">Rust</option>
                        <option value="python">Python</option>
                        <option value="javascript">JavaScript</option>
                        <option value="json">JSON</option>
                        <option value="sql">SQL</option>
                        <option value="html">HTML/XML</option>
                        <option value="css">CSS</option>
                        <option value="yaml">YAML</option>
                        <option value="bash">Bash</option>
                        <option value="java">Java</option>
                        <option value="c">C</option>
                        <option value="cpp">C++</option>
                        <option value="go">Go</option>
                    </select>

                    <button type="button" class="btn-small utility-btn" onclick="newNote()" title="Create New Note">New</button>
                    <!-- Save button moved here -->
                    <button type="button" id="save-btn" class="btn-small save-db-btn" onclick="handleSave()">Save Note</button>

                    <button type="button" id="toggle-preview-btn" class="btn-small utility-btn">Preview</button>
                    <button type="button" id="download-btn" class="btn-small utility-btn">Export</button>
                    <button type="button" id="email-btn" class="btn-small utility-btn">Email</button>
                    <button type="button" id="open-file-btn" class="btn-small utility-btn">Open</button>
                </div>

                <div class="editor-wrapper" id="editor-wrapper">
                    <div class="editor-container">
                        <div class="line-numbers" id="line-numbers"></div>
                        <div class="input-wrapper">
                            <pre id="highlight-layer" aria-hidden="true"><code class="hljs" style="background:transparent;padding:0;"></code></pre>
                            <textarea id="editor" name="content" placeholder="Type here..." spellcheck="false"></textarea>
                        </div>
                    </div>
                    <div id="markdown-preview"></div>
                </div>
            </form>
        </div>
    </div>

    <script>

        // Tabs Logic
        function switchTab(tabName) {{
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            
            if (tabName === 'db') {{
                document.querySelector('.tab:nth-child(1)').classList.add('active');
                document.getElementById('tab-db').classList.add('active');
            }} else {{
                document.querySelector('.tab:nth-child(2)').classList.add('active');
                document.getElementById('tab-fs').classList.add('active');
                loadBookmarks();
                if(document.getElementById('file-list').innerHTML === '') loadDir('.');
            }}
        }}

        // File System Logic
        let currentPath = '.';
        let currentFilePath = null; // Track currently open file path

        async function loadDir(path) {{
            // Optimistic update for UI
            document.getElementById('fs-path-input').value = path;
            const showHidden = document.getElementById('show-hidden-check').checked;
            
            const listEl = document.getElementById('file-list');
            listEl.innerHTML = '<div style="padding:10px; text-align:center;">Loading...</div>';

            try {{
                const res = await fetch('/note/ls', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ path: path, show_hidden: showHidden }})
                }});
                const data = await res.json();
                
                // Update with canonical path from server
                currentPath = data.current_path;
                document.getElementById('fs-path-input').value = currentPath;

                listEl.innerHTML = '';
                data.entries.forEach(entry => {{
                    const item = document.createElement('div');
                    item.className = 'file-item' + (entry.is_dir ? ' dir-item' : '');
                    const icon = entry.is_dir ? '📁' : '📄';
                    item.innerHTML = `<span class="file-icon">${{icon}}</span> ${{entry.name}}`;
                    
                    item.onclick = () => {{
                        if (entry.is_dir) {{
                            loadDir(entry.path);
                        }} else {{
                            loadFile(entry.path, entry.name);
                        }}
                    }};
                    listEl.appendChild(item);
                }});
            }} catch (e) {{
                listEl.innerHTML = '<div style="color:red; padding:5px;">Error loading dir</div>';
            }}
        }}

        function reloadDir() {{ loadDir(currentPath); }}

        function goUp() {{
            if (currentPath === '/' || currentPath.match(/^[a-zA-Z]:\\$/)) return;
            const separator = currentPath.includes('/') ? '/' : '\\\\';
            let parts = currentPath.split(separator);
            parts.pop();
            if (parts.length === 1 && parts[0] === '') {{ loadDir('/'); return; }}
            const newPath = parts.join(separator) || '/';
            loadDir(newPath);
        }}

        async function searchFs(query) {{
            if(!query.trim()) return loadDir(currentPath);
            const listEl = document.getElementById('file-list');
            listEl.innerHTML = '<div style="padding:10px; text-align:center;">Searching...</div>';
            try {{
                const res = await fetch('/note/search', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ path: currentPath, query: query }})
                }});
                const data = await res.json();
                listEl.innerHTML = `<div style="padding:2px; font-size:0.8em; color:#888;">Found ${{data.entries.length}} items (limit 50)</div>`;
                data.entries.forEach(entry => {{
                    const item = document.createElement('div');
                    item.className = 'file-item' + (entry.is_dir ? ' dir-item' : '');
                    const icon = entry.is_dir ? '📁' : '📄';
                    item.innerHTML = `<span class="file-icon">${{icon}}</span> ${{entry.name}} <span style="font-size:0.7em; opacity:0.5; margin-left:5px;">(${{entry.path}})</span>`;
                    item.onclick = () => {{ if (entry.is_dir) loadDir(entry.path); else loadFile(entry.path, entry.name); }};
                    listEl.appendChild(item);
                }});
            }} catch (e) {{
                listEl.innerHTML = '<div style="color:red; padding:5px;">Search Error</div>';
            }}
        }}

        // Bookmarks Logic
        async function loadBookmarks() {{
            const div = document.getElementById('bookmarks-list');
            const section = document.getElementById('bookmarks-section');
            try {{
                const res = await fetch('/note/bookmarks');
                const list = await res.json();
                if(list.length > 0) {{
                    section.style.display = 'block';
                    div.innerHTML = '';
                    list.forEach(b => {{
                        const item = document.createElement('div');
                        item.className = 'bookmark-item';
                        item.innerHTML = `<span class="bookmark-path" onclick="loadDir('${{b.path.replace(/\\/g, '\\\\')}}')">📁 ${{b.name}}</span> <span class="bookmark-del" onclick="deleteBookmark('${{b.path.replace(/\\/g, '\\\\')}}')">x</span>`;
                        div.appendChild(item);
                    }});
                }} else {{
                    section.style.display = 'none';
                }}
            }} catch(e) {{}}
        }}

        async function addBookmark() {{
            const parts = currentPath.split(/[/\\]/);
            const name = parts[parts.length-1] || 'Root';
            await fetch('/note/bookmarks/add', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ name: name, path: currentPath }}) }});
            loadBookmarks();
        }}

        async function deleteBookmark(path) {{
            await fetch('/note/bookmarks/delete', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ name: '', path: path }}) }});
            loadBookmarks();
        }}

        // Helper: Detect language from filename
        function detectLanguage(filename) {{
            const ext = filename.split('.').pop().toLowerCase();
            const map = {{
                'rs': 'rust', 'py': 'python', 'js': 'javascript', 'jsx': 'javascript', 'ts': 'javascript', 'tsx': 'javascript',
                'json': 'json', 'sql': 'sql', 'html': 'html', 'xml': 'html', 'svg': 'html', 'css': 'css',
                'yaml': 'yaml', 'yml': 'yaml', 'sh': 'bash', 'bash': 'bash', 'zsh': 'bash',
                'java': 'java', 'c': 'c', 'h': 'c', 'cpp': 'cpp', 'cc': 'cpp', 'hpp': 'cpp', 'go': 'go',
                'md': 'markdown', 'markdown': 'markdown', 'txt': 'text'
            }};
            return map[ext] || 'text';
        }}

        const languageSelect = document.getElementById('language-select');

        async function loadFile(path, name) {{
            try {{
                const res = await fetch('/note/read', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify({{ path: path }}) }});
                if(res.ok) {{
                    const text = await res.text();
                    document.getElementById('editor').value = text;
                    document.getElementById('subject').value = name; 
                    currentFilePath = path; // Set current file path
                    const lang = detectLanguage(name);
                    languageSelect.value = lang;
                    updateLineNumbers();
                    updateHighlighting();
                    document.getElementById('save-btn').textContent = "Save to File"; 
                    if(isPreview) updatePreview(); 
                }} else {{
                    alert("Failed to read file");
                }}
            }} catch (e) {{ alert("Error: " + e.message); }}
        }}
        
        // Search Filter for Notes
        const noteSearchInput = document.getElementById('note-search-input');
        const savedNotesList = document.getElementById('saved-notes-list');
        if(noteSearchInput) {{
            noteSearchInput.addEventListener('keyup', () => {{
                const filter = noteSearchInput.value.toUpperCase();
                const items = savedNotesList.getElementsByTagName('li');
                for (let i = 0; i < items.length; i++) {{
                    const span = items[i].querySelector('.saved-note');
                    if (span && span.textContent.toUpperCase().indexOf(filter) > -1) items[i].style.display = 'flex';
                    else items[i].style.display = 'none';
                }}
            }});
        }}

        // Editor Logic
        const subjectInput = document.getElementById("subject");
        const textarea = document.getElementById("editor");
        const backdrop = document.getElementById("highlight-layer");
        const highlightCodeBlock = backdrop.querySelector('code');
        const lineNumbers = document.getElementById("line-numbers");
        const saveButton = document.getElementById("save-btn");
        const fileInput = document.getElementById('file-input');
        const previewContainer = document.getElementById('markdown-preview');
        const editorContainer = document.querySelector('.editor-container');
        const togglePreviewBtn = document.getElementById('toggle-preview-btn');
        let isPreview = false;

        function updateLineNumbers() {{
            const lines = textarea.value.split("\n").length;
            lineNumbers.innerHTML = "";
            for (let i = 1; i <= lines; i++) {{
                const div = document.createElement("div");
                div.textContent = i;
                lineNumbers.appendChild(div);
            }}
        }}

        function updateHighlighting() {{
            const text = textarea.value;
            const lang = languageSelect.value;
            
            let highlighted = '';
            let renderText = text;
            if (renderText.endsWith('\n')) renderText += ' '; 

            if (lang === 'text' || lang === 'markdown') {{
                highlighted = renderText.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
            }} else {{
                try {{
                    const result = hljs.highlight(renderText, {{ language: lang }});
                    highlighted = result.value;
                }} catch (e) {{
                    highlighted = renderText.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
                }}
            }}
            
            highlightCodeBlock.innerHTML = highlighted;
            highlightCodeBlock.className = 'hljs language-' + lang;
        }}

        // Sync Scrolling
        const syncScroll = () => {{
            backdrop.scrollTop = textarea.scrollTop;
            backdrop.scrollLeft = textarea.scrollLeft;
            lineNumbers.scrollTop = textarea.scrollTop;
        }};

        textarea.addEventListener("scroll", syncScroll);
        
        textarea.addEventListener("input", () => {{ 
            updateLineNumbers(); 
            resetSaveButton();
            updateHighlighting();
        }});
        
        subjectInput.addEventListener("input", resetSaveButton);

        function resetSaveButton() {{
            if (currentFilePath) {{
               saveButton.textContent = "Save to File"; 
            }} else if (saveButton.textContent.startsWith("Update Note:")) {{
                saveButton.textContent = "Save / Update Note";
            }}
        }}
        
        function newNote() {{
            subjectInput.value = "";
            textarea.value = "";
            languageSelect.value = "markdown";
            saveButton.textContent = "Save Note";
            currentFilePath = null; // Clear path
            fileInput.value = "";
            updateLineNumbers();
            updateHighlighting();
            if (isPreview) {{
                // Switch back to edit mode
                togglePreviewBtn.click();
            }}
        }}
        
        async function handleSave() {{
            const content = textarea.value;
            if (currentFilePath) {{
                if (confirm(`Overwrite existing file?\n${{currentFilePath}}`)) {{
                    try {{
                        const res = await fetch('/note/save_file', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{ path: currentFilePath, content: content }})
                        }});
                        if (res.ok) {{
                            alert('File saved.');
                        }} else {{
                            alert('Error saving file: ' + await res.text());
                        }}
                    }} catch (e) {{
                        alert('Error: ' + e.message);
                    }}
                }}
            }} else {{
                document.getElementById('note-form').submit();
            }}
        }}

        // Paste JSON Format
        textarea.addEventListener("paste", function() {{
            setTimeout(() => {{
                try {{
                    let val = textarea.value.trim();
                    if (val.startsWith("{{") || val.startsWith("[")) {{
                        let obj = JSON.parse(val);
                        textarea.value = JSON.stringify(obj, null, 2);
                        updateLineNumbers();
                        updateHighlighting();
                    }}
                }} catch (err) {{}}
            }}, 10);
        }});

        // Markdown Logic
        function renderMarkdown(text) {{
            // 1. Safe extraction of code blocks
            const codeBlocks = [];
            // Regex handles ```lang ... ``` blocks
            let protectedText = text.replace(/```(\w*)\s+([\s\S]*?)```/g, (match, lang, code) => {{
                const placeholder = `__CODE_BLOCK_${{codeBlocks.length}}__`;
                codeBlocks.push({{ lang: lang.trim(), code: code.trim() }});
                return placeholder;
            }});

            // 2. Standard Markdown Parsing
            let html = protectedText
                .replace(/^# (.*$)/gim, '<h1>$1</h1>')
                .replace(/^## (.*$)/gim, '<h2>$1</h2>')
                .replace(/^### (.*$)/gim, '<h3>$1</h3>')
                .replace(/^\> (.*$)/gim, '<blockquote>$1</blockquote>')
                .replace(/\*\*(.*)\*\*/gim, '<b>$1</b>')
                .replace(/\*(.*)\*/gim, '<i>$1</i>')
                .replace(/`([^`]+)`/gim, '<code>$1</code>')
                .replace(/\[(.*?)\]\((.*?)\)/gim, "<a href='$2' target='_blank'>$1</a>")
                .replace(/\n/gim, '<br />');

            // 3. Restore Code Blocks with Syntax Highlighting Prep
            codeBlocks.forEach((block, index) => {{
                const placeholder = `__CODE_BLOCK_${{index}}__`;
                const escapedCode = block.code.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
                const langClass = block.lang ? `language-${{block.lang}}` : '';
                const htmlBlock = `<pre><code class="${{langClass}}">${{escapedCode}}</code></pre>`;
                html = html.replace(placeholder, htmlBlock);
            }});

            return html;
        }}

        function updatePreview() {{
            if (isPreview) {{
                const lang = languageSelect.value;
                const content = textarea.value;
                
                if (lang === 'markdown') {{
                    previewContainer.innerHTML = renderMarkdown(content);
                }} else if (lang === 'text') {{
                    previewContainer.innerHTML = '<pre>' + content.replace(/&/g, "&amp;").replace(/</g, "&lt;") + '</pre>';
                }} else {{
                    // Code highlight for entire file
                    const escaped = content.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
                    previewContainer.innerHTML = `<pre><code class="language-${{lang}}">${{escaped}}</code></pre>`;
                }}
                
                editorContainer.style.display = 'none';
                previewContainer.style.display = 'block';
                togglePreviewBtn.textContent = 'Edit';
                
                if (window.hljs) {{
                    previewContainer.querySelectorAll('pre code').forEach((block) => {{
                        hljs.highlightElement(block);
                    }});
                }}
            }} else {{
                editorContainer.style.display = 'flex';
                previewContainer.style.display = 'none';
                togglePreviewBtn.textContent = 'Preview';
            }}
        }}

        togglePreviewBtn.addEventListener('click', (e) => {{
            e.preventDefault(); 
            isPreview = !isPreview;
            updatePreview();
        }});

        // File IO (Client side open)
        document.getElementById('open-file-btn').addEventListener('click', (e) => {{ e.preventDefault(); fileInput.click(); }});
        fileInput.addEventListener('change', (e) => {{
            const file = e.target.files[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = (e) => {{
                textarea.value = e.target.result;
                subjectInput.value = file.name;
                currentFilePath = null; // Clear path for uploaded files
                const lang = detectLanguage(file.name);
                languageSelect.value = lang;
                updateLineNumbers();
                resetSaveButton();
                updateHighlighting();
                if(isPreview) updatePreview(); 
            }};
            reader.readAsText(file);
            fileInput.value = '';
        }});

        document.getElementById('download-btn').addEventListener('click', (e) => {{
            e.preventDefault();
            const text = textarea.value;
            if (!text) {{ alert("Note is empty!"); return; }}
            let name = subjectInput.value.trim() || 'note.txt';
            if (!name.includes('.')) name += '.txt';
            const blob = new Blob([text], {{ type: 'text/plain' }});
            const anchor = document.createElement('a');
            anchor.download = name;
            anchor.href = window.URL.createObjectURL(blob);
            anchor.click();
        }});

        document.getElementById('email-btn').addEventListener('click', (e) => {{
            e.preventDefault();
            const subject = subjectInput.value.trim();
            const body = textarea.value.trim();
            const gmailUrl = `https://mail.google.com/mail/?view=cm&fs=1&su=${{encodeURIComponent(subject)}}&body=${{encodeURIComponent(body)}}`;
            window.open(gmailUrl, '_blank');
        }});

        // Saved Note Selection
        savedNotesList.addEventListener('click', (event) => {{
            const li = event.target.closest('.saved-note-item');
            if (li) {{
                 // Handle selection visual
                 document.querySelectorAll('.saved-note-item').forEach(el => el.classList.remove('selected'));
                 li.classList.add('selected');

                const span = li.querySelector('.saved-note');
                if (span) {{
                    const subject = span.getAttribute('data-subject');
                    const content = span.getAttribute('data-content');
                    subjectInput.value = subject;
                    textarea.value = content;
                    currentFilePath = null; // Clear path for DB notes
                    languageSelect.value = 'markdown'; 
                    updateLineNumbers();
                    updateHighlighting();
                    saveButton.textContent = "Update Note: " + subject;
                    if (isPreview) updatePreview();
                    else subjectInput.focus();
                }}
            }}
        }});
        
        // Initial setup
        updateLineNumbers();
        updateHighlighting();
        
        // Inject Shared Sidebar JS
        {sidebar_js}
    </script>
    "#, 
    sidebar_html = sidebar_html,
    sidebar_js = sidebar_js
    );

    render_base_page("Quick Notes", &format!("{}{}{}", page_styles, crate::elements::sidebar::get_css(), body_content), current_theme)
}