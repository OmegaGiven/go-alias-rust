use actix_web::{get, post, web::{Data, Json}, HttpResponse, Responder};
use std::{fs, io, sync::Arc, time::{SystemTime, UNIX_EPOCH}, collections::HashMap};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

const BOARD_FILE: &str = "board.json";

// --- Data Structures ---

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Task {
    id: String,
    column_id: String,
    title: String,
    description: String,
    tags: Vec<String>,
    custom_fields: HashMap<String, String>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Column {
    id: String,
    title: String,
    order: usize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct BoardData {
    columns: Vec<Column>,
    tasks: Vec<Task>,
}

// --- Payload Structs ---

#[derive(Deserialize)]
struct CreateColumnPayload {
    title: String,
}

#[derive(Deserialize)]
struct SaveTaskPayload {
    id: Option<String>, // None for new, Some for edit
    column_id: String,
    title: String,
    description: String,
    tags: String, // Comma separated
    custom_fields: HashMap<String, String>,
}

#[derive(Deserialize)]
struct MoveTaskPayload {
    task_id: String,
    new_column_id: String,
}

#[derive(Deserialize)]
struct ReorderColumnsPayload {
    column_ids: Vec<String>,
}

#[derive(Deserialize)]
struct DeletePayload {
    id: String,
}

// --- Logic ---

fn load_board() -> BoardData {
    if let Ok(data) = fs::read_to_string(BOARD_FILE) {
        let mut board: BoardData = serde_json::from_str(&data).unwrap_or_else(|_| init_default_board());
        // Ensure columns are sorted by order field on load
        board.columns.sort_by_key(|c| c.order);
        board
    } else {
        init_default_board()
    }
}

fn init_default_board() -> BoardData {
    BoardData {
        columns: vec![
            Column { id: "todo".into(), title: "To Do".into(), order: 0 },
            Column { id: "progress".into(), title: "In Progress".into(), order: 1 },
            Column { id: "done".into(), title: "Done".into(), order: 2 },
        ],
        tasks: vec![],
    }
}

fn save_board(data: &BoardData) -> io::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(BOARD_FILE, json)
}

fn current_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn generate_id() -> String {
    format!("{:x}", current_ts()) // Simple timestamp based ID
}

// --- Handlers ---

#[get("/board")]
pub async fn board_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_board_page(&current_theme))
}

#[get("/board/data")]
pub async fn board_data_get() -> impl Responder {
    let data = load_board();
    HttpResponse::Ok().json(data)
}

#[post("/board/column/add")]
pub async fn board_add_column(payload: Json<CreateColumnPayload>) -> impl Responder {
    let mut data = load_board();
    let new_col = Column {
        id: format!("col_{}", generate_id()),
        title: payload.title.clone(),
        order: data.columns.len(),
    };
    data.columns.push(new_col);
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

#[post("/board/column/delete")]
pub async fn board_delete_column(payload: Json<DeletePayload>) -> impl Responder {
    let mut data = load_board();
    data.columns.retain(|c| c.id != payload.id);
    // Also delete tasks in that column
    data.tasks.retain(|t| t.column_id != payload.id);
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

// NEW: Handler to reorder columns
#[post("/board/column/reorder")]
pub async fn board_reorder_columns(payload: Json<ReorderColumnsPayload>) -> impl Responder {
    let mut data = load_board();
    
    // Map existing columns for easy lookup
    let mut col_map: HashMap<String, Column> = data.columns.drain(..)
        .map(|c| (c.id.clone(), c))
        .collect();
        
    let mut new_cols = Vec::new();
    
    // Rebuild list based on new ID order
    for (idx, id) in payload.column_ids.iter().enumerate() {
        if let Some(mut col) = col_map.remove(id) {
            col.order = idx;
            new_cols.push(col);
        }
    }
    
    // Append any remaining columns (fallback safety)
    let mut remaining: Vec<Column> = col_map.into_values().collect();
    remaining.sort_by_key(|c| c.order);
    for mut col in remaining {
        col.order = new_cols.len();
        new_cols.push(col);
    }
    
    data.columns = new_cols;
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

#[post("/board/task/save")]
pub async fn board_save_task(payload: Json<SaveTaskPayload>) -> impl Responder {
    let mut data = load_board();
    let ts = current_ts();

    let tags_vec: Vec<String> = payload.tags.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if let Some(id) = &payload.id {
        // Update
        if let Some(task) = data.tasks.iter_mut().find(|t| &t.id == id) {
            task.title = payload.title.clone();
            task.description = payload.description.clone();
            task.tags = tags_vec;
            task.custom_fields = payload.custom_fields.clone();
            task.updated_at = ts;
        }
    } else {
        // Create
        let new_task = Task {
            id: generate_id(),
            column_id: payload.column_id.clone(),
            title: payload.title.clone(),
            description: payload.description.clone(),
            tags: tags_vec,
            custom_fields: payload.custom_fields.clone(),
            created_at: ts,
            updated_at: ts,
        };
        data.tasks.push(new_task);
    }
    
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

#[post("/board/task/move")]
pub async fn board_move_task(payload: Json<MoveTaskPayload>) -> impl Responder {
    let mut data = load_board();
    if let Some(task) = data.tasks.iter_mut().find(|t| t.id == payload.task_id) {
        task.column_id = payload.new_column_id.clone();
        task.updated_at = current_ts();
    }
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

#[post("/board/task/delete")]
pub async fn board_delete_task(payload: Json<DeletePayload>) -> impl Responder {
    let mut data = load_board();
    data.tasks.retain(|t| t.id != payload.id);
    let _ = save_board(&data);
    HttpResponse::Ok().json(data)
}

// --- Rendering ---

fn render_board_page(current_theme: &Theme) -> String {
    let style = r#"
<style>
    .board-app {
        height: calc(100vh - 90px);
        display: flex;
        flex-direction: column;
    }
    .board-toolbar {
        padding: 10px;
        background: var(--secondary-bg);
        border-bottom: 1px solid var(--border-color);
        display: flex;
        gap: 10px;
    }
    .board-container {
        flex-grow: 1;
        overflow-x: auto;
        display: flex;
        padding: 20px;
        gap: 20px;
        align-items: flex-start; /* Important for column height */
    }
    
    /* Column Styles */
    .column {
        min-width: 300px;
        max-width: 300px;
        background: var(--secondary-bg);
        border-radius: 8px;
        border: 1px solid var(--border-color);
        display: flex;
        flex-direction: column;
        max-height: 100%;
        cursor: default; /* Default cursor inside, grab on header */
        transition: transform 0.2s;
    }
    .column.dragging {
        opacity: 0.4;
        border: 2px dashed var(--link-color);
    }
    .column-header {
        padding: 10px 15px;
        border-bottom: 1px solid var(--border-color);
        font-weight: bold;
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: var(--tertiary-bg);
        border-radius: 8px 8px 0 0;
        cursor: grab; /* Explicit grab cursor for header */
    }
    .column-header:active {
        cursor: grabbing;
    }
    .column-body {
        padding: 10px;
        overflow-y: auto;
        flex-grow: 1;
        min-height: 100px; /* Ensure drop target has height */
    }

    /* Task Card Styles */
    .task-card {
        background: var(--primary-bg);
        border: 1px solid var(--border-color);
        border-radius: 4px;
        padding: 10px;
        margin-bottom: 10px;
        cursor: grab;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        transition: transform 0.2s, box-shadow 0.2s;
    }
    .task-card:active { cursor: grabbing; }
    .task-card:hover { box-shadow: 0 4px 8px rgba(0,0,0,0.2); border-color: var(--link-color); }
    .task-card.dragging {
        opacity: 0.5;
    }
    
    .task-title { font-weight: bold; margin-bottom: 5px; }
    .task-meta { font-size: 0.8em; color: #888; margin-top: 5px; display: flex; flex-wrap: wrap; gap: 5px; }
    .tag { background: var(--link-color); color: var(--primary-bg); padding: 2px 6px; border-radius: 10px; font-size: 0.75em; }
    
    /* Modal */
    .modal { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.7); z-index: 1000; justify-content: center; align-items: center; }
    .modal.active { display: flex; }
    .modal-content { background: var(--secondary-bg); padding: 20px; border-radius: 8px; width: 500px; max-width: 90%; border: 1px solid var(--border-color); max-height: 90vh; overflow-y: auto; }
    .modal-header { font-size: 1.2em; font-weight: bold; margin-bottom: 15px; border-bottom: 1px solid var(--border-color); padding-bottom: 10px; }
    .form-group { margin-bottom: 15px; }
    .form-group label { display: block; margin-bottom: 5px; font-weight: bold; }
    .form-group input, .form-group textarea, .form-group select { width: 100%; padding: 8px; box-sizing: border-box; background: var(--primary-bg); color: var(--text-color); border: 1px solid var(--border-color); border-radius: 4px; }
    .kv-row { display: flex; gap: 5px; margin-bottom: 5px; }
    
    .btn { padding: 8px 12px; background: var(--link-color); color: #fff; border: none; border-radius: 4px; cursor: pointer; }
    .btn:hover { opacity: 0.9; }
    .btn-danger { background: #f93e3e; }
    .btn-secondary { background: var(--tertiary-bg); color: var(--text-color); border: 1px solid var(--border-color); }
    .icon-btn { background: none; border: none; color: var(--text-color); cursor: pointer; padding: 2px; font-size: 1.1em; }
    .icon-btn:hover { color: var(--link-hover); }

    /* Dragging Visuals */
    .drag-over-column { background: rgba(255,255,255,0.05); border: 2px dashed var(--link-color); }
</style>
"#;

    let html = r#"
    <div class="board-app">
        <div class="board-toolbar">
            <button class="btn btn-secondary" onclick="openColumnModal()">+ Add Column</button>
            <button class="btn" onclick="openTaskModal()">+ New Task</button>
            <div style="margin-left: auto; font-size: 0.9em; color: #888; align-self: center;">Drag columns by header. Drag tasks by card.</div>
        </div>
        <div class="board-container" id="board-container" ondragover="handleContainerDragOver(event)" ondrop="handleContainerDrop(event)">
            <!-- Columns will be injected here -->
        </div>
    </div>

    <!-- Task Modal -->
    <div id="task-modal" class="modal">
        <div class="modal-content">
            <div class="modal-header"><span id="modal-title">New Task</span></div>
            <input type="hidden" id="task-id">
            
            <div class="form-group">
                <label>Title</label>
                <input type="text" id="task-title" placeholder="Task summary">
            </div>
            
            <div class="form-group">
                <label>Description</label>
                <textarea id="task-desc" rows="4" placeholder="Detailed description..."></textarea>
            </div>
            
            <div class="form-group">
                <label>Column</label>
                <select id="task-column"></select>
            </div>
            
            <div class="form-group">
                <label>Tags (comma separated)</label>
                <input type="text" id="task-tags" placeholder="bug, urgent, frontend">
            </div>

            <div class="form-group">
                <label>Custom Fields <button type="button" class="icon-btn" onclick="addCustomFieldRow()">+</button></label>
                <div id="custom-fields-container"></div>
            </div>
            
            <div style="display: flex; justify-content: space-between; margin-top: 20px;">
                <button class="btn btn-danger" id="btn-delete-task" style="display:none;" onclick="deleteTask()">Delete</button>
                <div>
                    <button class="btn btn-secondary" onclick="closeModal('task-modal')">Cancel</button>
                    <button class="btn" onclick="saveTask()">Save</button>
                </div>
            </div>
            <div id="task-meta" style="margin-top: 15px; font-size: 0.8em; color: #888; border-top: 1px solid #444; padding-top: 10px; display: none;"></div>
        </div>
    </div>

    <!-- Column Modal -->
    <div id="col-modal" class="modal">
        <div class="modal-content">
            <div class="modal-header">Add Column</div>
            <div class="form-group">
                <label>Column Title</label>
                <input type="text" id="col-title">
            </div>
            <div style="text-align: right; margin-top: 20px;">
                <button class="btn btn-secondary" onclick="closeModal('col-modal')">Cancel</button>
                <button class="btn" onclick="saveColumn()">Create</button>
            </div>
        </div>
    </div>

    <script>
        let boardData = { columns: [], tasks: [] };
        let draggedType = null; // 'task' or 'column'
        
        // --- Init ---
        async function loadBoard() {
            const res = await fetch('/board/data');
            boardData = await res.json();
            renderBoard();
        }
        
        loadBoard();

        // --- Rendering ---
        function renderBoard() {
            const container = document.getElementById('board-container');
            container.innerHTML = '';
            
            // Render Columns
            boardData.columns.forEach(col => {
                const colDiv = document.createElement('div');
                colDiv.className = 'column';
                colDiv.id = col.id;
                colDiv.draggable = true;
                
                // Listeners for Column Dragging
                colDiv.ondragstart = (ev) => dragColumnStart(ev, col.id);
                colDiv.ondragend = (ev) => dragColumnEnd(ev);

                colDiv.innerHTML = `
                    <div class="column-header">
                        ${col.title}
                        <button class="icon-btn" onclick="deleteColumn('${col.id}')" title="Delete Column">x</button>
                    </div>
                    <div class="column-body" id="body_${col.id}" 
                         ondrop="dropTask(event, '${col.id}')" 
                         ondragover="allowDropTask(event)" 
                         ondragleave="dragLeaveTask(event)">
                    </div>
                `;
                container.appendChild(colDiv);
            });

            // Render Tasks
            boardData.tasks.forEach(task => {
                const colBody = document.getElementById('body_' + task.column_id);
                if (colBody) {
                    const card = document.createElement('div');
                    card.className = 'task-card';
                    card.draggable = true;
                    card.id = task.id;
                    
                    // Listeners for Task Dragging
                    card.ondragstart = (ev) => dragTaskStart(ev, task.id);
                    card.ondragend = (ev) => dragTaskEnd(ev);
                    
                    card.onclick = (ev) => {
                        ev.stopPropagation(); // Prevent bubbling
                        openTaskModal(task.id);
                    };
                    
                    let tagsHtml = task.tags.map(t => `<span class="tag">${t}</span>`).join('');
                    let customHtml = '';
                    let cfKeys = Object.keys(task.custom_fields);
                    if (cfKeys.length > 0) {
                        customHtml += `<div style="width:100%; margin-top:5px; font-size:0.9em;">`;
                        cfKeys.slice(0, 2).forEach(k => {
                             customHtml += `<div><b>${k}:</b> ${task.custom_fields[k]}</div>`;
                        });
                        customHtml += `</div>`;
                    }

                    card.innerHTML = `
                        <div class="task-title">${task.title}</div>
                        ${customHtml}
                        <div class="task-meta">
                            ${tagsHtml}
                            <span style="margin-left: auto;">${new Date(task.updated_at * 1000).toLocaleDateString()}</span>
                        </div>
                    `;
                    colBody.appendChild(card);
                }
            });
            
            // Update Select in Modal
            const select = document.getElementById('task-column');
            select.innerHTML = boardData.columns.map(c => `<option value="${c.id}">${c.title}</option>`).join('');
        }

        // --- TASK Drag & Drop ---
        function dragTaskStart(ev, id) {
            ev.stopPropagation(); // Stop bubbling so we don't trigger column drag
            draggedType = 'task';
            ev.dataTransfer.setData("text/plain", id);
            ev.dataTransfer.setData("type", "task");
            ev.target.classList.add('dragging');
        }

        function dragTaskEnd(ev) {
            ev.target.classList.remove('dragging');
            draggedType = null;
        }

        function allowDropTask(ev) {
            if (draggedType === 'task') {
                ev.preventDefault();
                ev.currentTarget.classList.add('drag-over-column');
            }
        }
        
        function dragLeaveTask(ev) {
            ev.currentTarget.classList.remove('drag-over-column');
        }

        async function dropTask(ev, colId) {
            if (draggedType !== 'task') return;
            ev.preventDefault();
            const colBody = ev.currentTarget;
            colBody.classList.remove('drag-over-column');
            const taskId = ev.dataTransfer.getData("text/plain");
            const card = document.getElementById(taskId);
            
            // Move visually
            colBody.appendChild(card);
            
            // Persist
            await fetch('/board/task/move', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({ task_id: taskId, new_column_id: colId })
            });
            
            // Reload to ensure data consistency
            const res = await fetch('/board/data');
            boardData = await res.json();
        }

        // --- COLUMN Drag & Drop ---
        function dragColumnStart(ev, id) {
            draggedType = 'column';
            ev.dataTransfer.setData("text/plain", id);
            ev.dataTransfer.setData("type", "column");
            ev.target.classList.add('dragging');
        }

        function dragColumnEnd(ev) {
            ev.target.classList.remove('dragging');
            draggedType = null;
        }

        function handleContainerDragOver(ev) {
            if (draggedType === 'column') {
                ev.preventDefault();
                const container = document.getElementById('board-container');
                const afterElement = getDragAfterElement(container, ev.clientX);
                const dragging = document.querySelector('.column.dragging');
                if (afterElement == null) {
                    container.appendChild(dragging);
                } else {
                    container.insertBefore(dragging, afterElement);
                }
            }
        }

        async function handleContainerDrop(ev) {
            if (draggedType !== 'column') return;
            ev.preventDefault();
            
            // Calculate new order based on DOM
            const container = document.getElementById('board-container');
            const newOrderIds = Array.from(container.children).map(child => child.id);
            
            // Persist new order
            await fetch('/board/column/reorder', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({ column_ids: newOrderIds })
            });

            // Reload
            const res = await fetch('/board/data');
            boardData = await res.json();
        }

        // Helper to determine where to drop column in horizontal list
        function getDragAfterElement(container, x) {
            // Get all columns EXCEPT the one being dragged
            const draggableElements = [...container.querySelectorAll('.column:not(.dragging)')];

            return draggableElements.reduce((closest, child) => {
                const box = child.getBoundingClientRect();
                const offset = x - box.left - box.width / 2;
                // We want the element where our cursor is to the LEFT of its center
                // offset < 0 means we are left of center
                if (offset < 0 && offset > closest.offset) {
                    return { offset: offset, element: child };
                } else {
                    return closest;
                }
            }, { offset: Number.NEGATIVE_INFINITY }).element;
        }


        // --- Modals & Logic ---
        function openColumnModal() {
            document.getElementById('col-modal').classList.add('active');
            document.getElementById('col-title').value = '';
            document.getElementById('col-title').focus();
        }

        function openTaskModal(taskId = null) {
            const modal = document.getElementById('task-modal');
            const container = document.getElementById('custom-fields-container');
            container.innerHTML = ''; // Clear custom fields
            
            if (taskId) {
                // Edit Mode
                const task = boardData.tasks.find(t => t.id === taskId);
                if (!task) return;
                
                document.getElementById('modal-title').innerText = 'Edit Task';
                document.getElementById('task-id').value = task.id;
                document.getElementById('task-title').value = task.title;
                document.getElementById('task-desc').value = task.description;
                document.getElementById('task-column').value = task.column_id;
                document.getElementById('task-tags').value = task.tags.join(', ');
                document.getElementById('btn-delete-task').style.display = 'inline-block';
                
                for (const [key, val] of Object.entries(task.custom_fields)) {
                    addCustomFieldRow(key, val);
                }
                
                const metaDiv = document.getElementById('task-meta');
                metaDiv.style.display = 'block';
                metaDiv.innerHTML = `Created: ${new Date(task.created_at * 1000).toLocaleString()}<br>Updated: ${new Date(task.updated_at * 1000).toLocaleString()}`;

            } else {
                // New Mode
                document.getElementById('modal-title').innerText = 'New Task';
                document.getElementById('task-id').value = '';
                document.getElementById('task-title').value = '';
                document.getElementById('task-desc').value = '';
                document.getElementById('task-tags').value = '';
                document.getElementById('btn-delete-task').style.display = 'none';
                document.getElementById('task-meta').style.display = 'none';
                if(boardData.columns.length > 0) {
                     document.getElementById('task-column').value = boardData.columns[0].id;
                }
            }
            modal.classList.add('active');
        }

        function closeModal(id) {
            document.getElementById(id).classList.remove('active');
        }
        
        function addCustomFieldRow(key = '', val = '') {
            const container = document.getElementById('custom-fields-container');
            const row = document.createElement('div');
            row.className = 'kv-row';
            row.innerHTML = `
                <input type="text" class="cf-key" placeholder="Field Name" value="${key}">
                <input type="text" class="cf-val" placeholder="Value" value="${val}">
                <button type="button" class="icon-btn" onclick="this.parentElement.remove()">x</button>
            `;
            container.appendChild(row);
        }

        // --- Actions ---
        async function saveColumn() {
            const title = document.getElementById('col-title').value;
            if (!title) return;
            
            const res = await fetch('/board/column/add', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({ title })
            });
            boardData = await res.json();
            renderBoard();
            closeModal('col-modal');
        }
        
        async function deleteColumn(id) {
            if(!confirm('Delete this column and all its tasks?')) return;
             const res = await fetch('/board/column/delete', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({ id })
            });
            boardData = await res.json();
            renderBoard();
        }

        async function saveTask() {
            const id = document.getElementById('task-id').value || null;
            const title = document.getElementById('task-title').value;
            if (!title) return alert('Title required');
            
            const custom_fields = {};
            document.querySelectorAll('#custom-fields-container .kv-row').forEach(row => {
                const k = row.querySelector('.cf-key').value.trim();
                const v = row.querySelector('.cf-val').value.trim();
                if (k) custom_fields[k] = v;
            });

            const payload = {
                id: id,
                title: title,
                description: document.getElementById('task-desc').value,
                column_id: document.getElementById('task-column').value,
                tags: document.getElementById('task-tags').value,
                custom_fields: custom_fields
            };
            
            const res = await fetch('/board/task/save', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify(payload)
            });
            boardData = await res.json();
            renderBoard();
            closeModal('task-modal');
        }
        
        async function deleteTask() {
            const id = document.getElementById('task-id').value;
            if(!id || !confirm('Delete this task?')) return;
            
            const res = await fetch('/board/task/delete', {
                method: 'POST',
                headers: {'Content-Type': 'application/json'},
                body: JSON.stringify({ id })
            });
            boardData = await res.json();
            renderBoard();
            closeModal('task-modal');
        }
    </script>
    "#;

    render_base_page("Task Board", &format!("{}{}", style, html), current_theme)
}