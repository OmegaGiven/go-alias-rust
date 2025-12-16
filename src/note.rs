use actix_web::{get, post, web::{self, Data}, HttpResponse, Responder};
use htmlescape::encode_minimal;
use serde::Deserialize;
use std::{fs, io::{self, Write}, sync::Arc};
use serde_json;

use crate::app_state::{AppState, Theme, Note};
use crate::base_page::render_base_page;

static NOTES_FILE: &str = "notes.json";

#[derive(Deserialize)]
pub struct NoteForm {
    pub subject: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct _DeleteForm {
    pub note_index: usize,
}

pub fn save_notes(notes: &[Note]) -> io::Result<()> {
    let json = serde_json::to_string(notes)?;
    let mut f = fs::File::create(NOTES_FILE)?;
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
    
    save_notes(&notes).ok();
    
    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}

pub async fn note_delete(
    state: Data<Arc<AppState>>,
    form: web::Form<_DeleteForm>,
) -> impl Responder {
    let mut notes = state.notes.lock().unwrap();
    let index = form.note_index;

    if index < notes.len() {
        notes.remove(index);
        save_notes(&notes).ok();
        println!("Note deleted at index: {}", index);
    } else {
        eprintln!("Attempted to delete note with out-of-bounds index: {}", index);
    }

    HttpResponse::SeeOther()
        .append_header(("Location", "/note"))
        .finish()
}


fn render_note_page(notes: &[Note], current_theme: &Theme) -> String {
    let saved_notes_list = notes
        .iter()
        .enumerate()
        .map(|(index, n)| {
            format!(
                r#"
                <li class="saved-note-item">
                    <form method="POST" action="/note/delete" class="delete-form">
                        <input type="hidden" name="note_index" value="{index}">
                        <button type="submit" class="delete-button" title="Delete this note">x</button>
                    </form>
                    <span class="saved-note" data-index="{index}" data-subject="{subject}" data-content="{content}">
                        {subject_escaped}
                    </span>
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

    let page_styles = r#"
<style>
    .note-view-container { display: flex; height: calc(100vh - 60px); position: relative; overflow: hidden; }
    
    /* Sidebar Styles */
    #sidebar { 
        width: 250px; 
        min-width: 0; 
        background: var(--secondary-bg); 
        color: var(--text-color); 
        padding: 5px; 
        overflow-y: auto; 
        flex-shrink: 0; 
        font-size: 0.9em; 
        border-right: 1px solid var(--border-color);
    }
    #sidebar.collapsed { width: 0 !important; padding: 0 !important; overflow: hidden; }
    
    #sidebar h2 { margin: 5px 0 5px 0; padding-bottom: 5px; border-bottom: 1px solid var(--border-color); font-size: 1.1em; white-space: nowrap; overflow: hidden; }
    #sidebar ul { list-style: none; padding: 0; margin: 0; }
    
    .sidebar-search input { width: 100%; padding: 4px; margin-bottom: 5px; box-sizing: border-box; border: 1px solid var(--border-color); background: var(--primary-bg); color: var(--text-color); border-radius: 4px; font-size: 0.9em; }

    .saved-note-item { display: flex; align-items: center; margin-bottom: 1px; background-color: var(--tertiary-bg); border-radius: 4px; padding: 2px; }
    .saved-note-item:hover { background-color: var(--border-color); }
    .saved-note { cursor: pointer; font-weight: bold; transition: color 0.2s; flex-grow: 1; display: block; margin-left: 5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .saved-note:hover { color: var(--link-hover); }
    .delete-form { margin: 0; display: inline-flex; align-items: center; }
    .delete-button { background: none; border: none; cursor: pointer; color: var(--text-color); padding: 0 5px; font-size: 0.9em; opacity: 0.6; }
    .delete-button:hover { color: #ff6b6b; opacity: 1; }

    /* Resizer */
    #sidebar-resizer {
        width: 6px;
        background-color: var(--tertiary-bg);
        border-right: 1px solid var(--border-color);
        cursor: col-resize;
        flex-shrink: 0;
        z-index: 100;
        transition: background-color 0.2s;
    }
    #sidebar-resizer:hover, #sidebar-resizer.resizing { background-color: var(--link-hover); }

    /* Main Content */
    #main { flex: 1; display: flex; flex-direction: column; padding: 10px; overflow: hidden; background-color: var(--primary-bg); }
    
    /* Editor Toolbar */
    .toolbar { display: flex; gap: 5px; margin-bottom: 5px; align-items: center; flex-wrap: wrap; flex-shrink: 0; }
    .subject-input { flex-grow: 1; padding: 5px 8px; border: 1px solid var(--border-color); background-color: var(--secondary-bg); color: var(--text-color); border-radius: 4px; height: 28px; }
    .utility-btn { background-color: var(--tertiary-bg); border: 1px solid var(--border-color); padding: 5px 10px; font-size: 0.9em; border-radius: 4px; height: 28px; cursor: pointer; color: var(--text-color); }
    .utility-btn:hover { background-color: var(--link-hover); color: white; border-color: var(--link-hover); }
    
    .editor-wrapper { display: flex; flex-direction: column; flex-grow: 1; border: 1px solid var(--border-color); border-radius: 4px; overflow: hidden; min-height: 0; }
    .editor-container { display: flex; flex-grow: 1; overflow: hidden; min-height: 0; }
    
    .line-numbers { background-color: var(--tertiary-bg); color: #777; padding: 10px 5px; text-align: right; user-select: none; overflow: hidden; border-right: 1px solid var(--border-color); flex-shrink: 0; min-width: 35px; box-sizing: border-box; font-family: 'Consolas', monospace; font-size: 12px; line-height: 20px; }
    #editor { flex-grow: 1; border: none; outline: none; padding: 10px; white-space: pre; overflow: auto; resize: none; background-color: var(--secondary-bg); color: var(--text-color); box-sizing: border-box; font-family: 'Consolas', monospace; font-size: 12px; line-height: 20px; }
    
    #markdown-preview { display: none; flex-grow: 1; border: 1px solid var(--border-color); border-radius: 4px; padding: 20px; background-color: var(--secondary-bg); color: var(--text-color); overflow-y: auto; }
    
    .save-db-btn { 
        background-color: var(--tertiary-bg); 
        border: 1px solid var(--border-color); 
        padding: 5px 10px; 
        font-size: 0.9em; 
        border-radius: 4px; 
        height: 28px; 
        cursor: pointer; 
        color: var(--text-color); 
        font-weight: bold;
        white-space: nowrap;
    }
    .save-db-btn:hover { background-color: var(--link-hover); color: white; border-color: var(--link-hover); }

    #markdown-preview h1, #markdown-preview h2 { border-bottom: 1px solid var(--border-color); padding-bottom: 5px; }
    #markdown-preview code { background: #444; padding: 2px 5px; border-radius: 3px; }
    #markdown-preview pre { background: #333; padding: 10px; border-radius: 5px; overflow-x: auto; }
    #markdown-preview blockquote { border-left: 3px solid var(--link-color); margin-left: 0; padding-left: 10px; color: #aaa; }
</style>
"#;

    let body_content = format!(r#"
    <div class="note-view-container">
        <div id="sidebar">
            <h2>Saved Notes</h2>
            <div class="sidebar-search"><input type="text" id="note-search-input" placeholder="Search notes..."></div>
            <ul id="saved-notes-list">
                {saved_notes_list}
            </ul>
        </div>
        
        <div id="sidebar-resizer" title="Drag to resize, Click to toggle"></div>

        <div id="main">
            <form method="POST" action="/note" style="display: flex; flex-direction: column; height: 100%;">
                <div class="toolbar">
                    <input type="file" id="file-input" style="display: none;" accept=".txt,.md,.json,.rs,.js,.html">
                    <input type="text" id="subject" name="subject" placeholder="Subject / Filename" value="" class="subject-input" autocomplete="off" />
                    
                    <!-- Save button moved here -->
                    <button type="submit" id="save-btn" class="save-db-btn">Save Note</button>

                    <button type="button" id="toggle-preview-btn" class="utility-btn">Preview</button>
                    <button type="button" id="download-btn" class="utility-btn">Export</button>
                    <button type="button" id="email-btn" class="utility-btn">Email</button>
                    <button type="button" id="open-file-btn" class="utility-btn">Open</button>
                </div>

                <div class="editor-wrapper" id="editor-wrapper">
                    <div class="editor-container">
                        <div class="line-numbers" id="line-numbers"></div>
                        <textarea id="editor" name="content" placeholder="Type here..." spellcheck="false"></textarea>
                    </div>
                    <div id="markdown-preview"></div>
                </div>
            </form>
        </div>
    </div>

    <script>
        // Sidebar Resizing Logic
        const sidebar = document.getElementById('sidebar');
        const resizer = document.getElementById('sidebar-resizer');
        let isResizing = false;
        let lastDownX = 0;
        let savedSidebarWidth = 250;

        resizer.addEventListener('mousedown', (e) => {{
            isResizing = true;
            lastDownX = e.clientX;
            resizer.classList.add('resizing');
            document.body.style.cursor = 'col-resize';
            document.body.style.userSelect = 'none';
        }});

        document.addEventListener('mousemove', (e) => {{
            if (!isResizing) return;
            let newWidth = e.clientX;
            if (newWidth < 0) newWidth = 0;
            if (newWidth > 600) newWidth = 600;
            
            sidebar.style.width = newWidth + 'px';
            
            if (newWidth === 0) {{
                sidebar.classList.add('collapsed');
            }} else {{
                sidebar.classList.remove('collapsed');
            }}
        }});

        document.addEventListener('mouseup', (e) => {{
            if (!isResizing) return;
            isResizing = false;
            resizer.classList.remove('resizing');
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
            
            // Click detection (moved less than 5px)
            if (Math.abs(e.clientX - lastDownX) < 5) {{
                toggleSidebar();
            }} else {{
                if (sidebar.offsetWidth > 0) {{
                    savedSidebarWidth = sidebar.offsetWidth;
                }}
            }}
        }});

        function toggleSidebar() {{
            if (sidebar.offsetWidth === 0 || sidebar.classList.contains('collapsed')) {{
                sidebar.classList.remove('collapsed');
                sidebar.style.width = (savedSidebarWidth || 250) + 'px';
            }} else {{
                savedSidebarWidth = sidebar.offsetWidth;
                sidebar.classList.add('collapsed');
                sidebar.style.width = '0px';
            }}
        }}
        
        // Search Filter
        const noteSearchInput = document.getElementById('note-search-input');
        const savedNotesList = document.getElementById('saved-notes-list');
        if(noteSearchInput) {{
            noteSearchInput.addEventListener('keyup', () => {{
                const filter = noteSearchInput.value.toUpperCase();
                const items = savedNotesList.getElementsByTagName('li');
                for (let i = 0; i < items.length; i++) {{
                    const span = items[i].querySelector('.saved-note');
                    if (span && span.textContent.toUpperCase().indexOf(filter) > -1) {{
                        items[i].style.display = 'flex';
                    }} else {{
                        items[i].style.display = 'none';
                    }}
                }}
            }});
        }}

        // Editor Logic
        const subjectInput = document.getElementById("subject");
        const textarea = document.getElementById("editor");
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

        textarea.addEventListener("scroll", () => {{ lineNumbers.scrollTop = textarea.scrollTop; }});
        textarea.addEventListener("input", () => {{ updateLineNumbers(); resetSaveButton(); }});
        subjectInput.addEventListener("input", resetSaveButton);

        function resetSaveButton() {{
            if (saveButton.textContent.startsWith("Update Note:")) {{
                saveButton.textContent = "Save / Update Note";
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
                    }}
                }} catch (err) {{}}
            }}, 10);
        }});

        // Markdown Logic
        function renderMarkdown(text) {{
            return text
                .replace(/^# (.*$)/gim, '<h1>$1</h1>')
                .replace(/^## (.*$)/gim, '<h2>$1</h2>')
                .replace(/^### (.*$)/gim, '<h3>$1</h3>')
                .replace(/^\> (.*$)/gim, '<blockquote>$1</blockquote>')
                .replace(/\*\*(.*)\*\*/gim, '<b>$1</b>')
                .replace(/\*(.*)\*/gim, '<i>$1</i>')
                .replace(/`([^`]+)`/gim, '<code>$1</code>')
                .replace(/```([^`]+)```/gim, '<pre><code>$1</code></pre>')
                .replace(/\[(.*?)\]\((.*?)\)/gim, "<a href='$2' target='_blank'>$1</a>")
                .replace(/\n/gim, '<br />');
        }}

        togglePreviewBtn.addEventListener('click', (e) => {{
            e.preventDefault(); 
            isPreview = !isPreview;
            if (isPreview) {{
                previewContainer.innerHTML = renderMarkdown(textarea.value);
                editorContainer.style.display = 'none';
                previewContainer.style.display = 'block';
                togglePreviewBtn.textContent = 'Edit';
            }} else {{
                editorContainer.style.display = 'flex';
                previewContainer.style.display = 'none';
                togglePreviewBtn.textContent = 'Preview';
            }}
        }});

        // File IO
        document.getElementById('open-file-btn').addEventListener('click', (e) => {{ e.preventDefault(); fileInput.click(); }});
        fileInput.addEventListener('change', (e) => {{
            const file = e.target.files[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = (e) => {{
                textarea.value = e.target.result;
                subjectInput.value = file.name;
                updateLineNumbers();
                resetSaveButton();
                if(isPreview) previewContainer.innerHTML = renderMarkdown(textarea.value);
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
            const span = event.target.closest('.saved-note');
            if (span) {{
                const subject = span.getAttribute('data-subject');
                const content = span.getAttribute('data-content');
                subjectInput.value = subject;
                textarea.value = content;
                updateLineNumbers();
                saveButton.textContent = "Update Note: " + subject;
                if (isPreview) previewContainer.innerHTML = renderMarkdown(content);
                else subjectInput.focus();
            }}
        }});
        
        updateLineNumbers();
    </script>
    "#, 
    saved_notes_list = saved_notes_list
    );

    render_base_page("Quick Notes", &format!("{}{}", page_styles, body_content), current_theme)
}