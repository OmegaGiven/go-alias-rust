use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

// Handler for GET /paint
#[get("/paint")]
pub async fn paint_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_paint_page(&current_theme))
}

fn render_paint_page(current_theme: &Theme) -> String {
    let style = r#"
<style>
    .paint-app {
        display: flex;
        flex-direction: column;
        gap: 0; /* Removed gap */
        height: calc(100vh - 80px); /* Fill remaining screen height */
        padding: 0; /* Removed padding */
    }
    
    .toolbar {
        display: flex;
        gap: 10px;
        align-items: center;
        padding: 8px;
        background-color: var(--secondary-bg);
        border-bottom: 1px solid var(--border-color); /* Only bottom border */
        border-radius: 0; /* Removed rounded corners */
        flex-wrap: wrap;
        flex-shrink: 0;
    }
    
    .tool-group {
        display: flex;
        align-items: center;
        gap: 5px;
        border-right: 1px solid var(--border-color);
        padding-right: 10px;
    }
    .tool-group:last-child {
        border-right: none;
        padding-right: 0;
    }
    
    .canvas-container {
        flex-grow: 1;
        background-color: #1a1a1a; 
        border: none; /* Removed border */
        border-radius: 0; /* Removed rounded corners */
        overflow: auto; /* Allow scrolling for large canvases */
        position: relative;
        display: grid;
        place-items: center; /* Center the canvas */
    }
    
    /* Scope styles to avoid affecting global nav bar */
    .paint-app canvas {
        display: block;
        touch-action: none;
        background-color: #000; /* Actual canvas bg */
        box-shadow: 0 0 10px rgba(0,0,0,0.5);
    }

    .paint-app label {
        font-weight: bold;
        margin-right: 3px;
        color: var(--text-color);
        font-size: 0.9rem;
    }

    .paint-app input[type="color"] {
        border: none;
        width: 30px;
        height: 30px;
        cursor: pointer;
        background: none;
        padding: 0;
    }

    .paint-app input[type="range"] {
        cursor: pointer;
        height: 10px;
    }
    
    .paint-app input[type="number"] {
        background: var(--primary-bg);
        border: 1px solid var(--border-color);
        color: var(--text-color);
        border-radius: 4px;
        padding: 4px;
        width: 60px;
    }

    /* Scope button styles specifically to paint-app to fix nav bar issue */
    .paint-app button {
        padding: 4px 8px;
        font-size: 0.9rem;
        cursor: pointer;
        background-color: var(--tertiary-bg);
        color: var(--text-color);
        border: 1px solid var(--border-color);
        border-radius: 4px;
        transition: background-color 0.2s;
    }
    .paint-app button:hover {
        background-color: var(--link-hover);
        color: white;
        border-color: var(--link-hover);
    }
    
    .size-display {
        min-width: 20px;
        display: inline-block;
        text-align: center;
        font-size: 0.9rem;
    }
</style>
    "#;

    let html_content = r##"
    <div class="paint-app">
        <div class="toolbar">
            <!-- Hidden file input for image loading -->
            <input type="file" id="image-loader" accept="image/png, image/jpeg, image/jpg" style="display: none;">

            <div class="tool-group">
                <label>Dimensions:</label>
                <input type="number" id="canvas-width" value="800" title="Width">
                <span style="color:var(--text-color);">x</span>
                <input type="number" id="canvas-height" value="600" title="Height">
                <button id="resize-btn" title="Update canvas size (clears if confirmed)">Set</button>
            </div>

            <div class="tool-group">
                <label for="color">Brush:</label>
                <input type="color" id="color" value="#ffffff">
            </div>

            <div class="tool-group">
                <label for="bg-color">BG:</label>
                <input type="color" id="bg-color" value="#000000">
                <button id="fill-btn" title="Fill entire canvas">Fill</button>
            </div>
            
            <div class="tool-group">
                <label for="size">Size:</label>
                <input type="range" id="size" min="1" max="50" value="5">
                <span id="size-val" class="size-display">5</span>
            </div>
            
            <div class="tool-group">
                <button id="eraser-btn">Eraser</button>
                <button id="brush-btn">Brush</button>
            </div>

            <div class="tool-group">
                <button id="clear-btn">Clear</button>
            </div>

            <div class="tool-group" style="margin-left: auto;">
                <button id="open-img-btn">📂 Open</button>
                <button id="download-btn">💾 Save</button>
            </div>
        </div>

        <div class="canvas-container" id="canvas-container">
            <canvas id="drawing-board"></canvas>
        </div>
    </div>

    <script>
        const canvas = document.getElementById('drawing-board');
        const container = document.getElementById('canvas-container');
        const ctx = canvas.getContext('2d');
        
        const imageLoader = document.getElementById('image-loader');
        const openImgBtn = document.getElementById('open-img-btn');

        const colorPicker = document.getElementById('color');
        const bgColorPicker = document.getElementById('bg-color');
        
        const sizePicker = document.getElementById('size');
        const sizeVal = document.getElementById('size-val');
        
        const widthInput = document.getElementById('canvas-width');
        const heightInput = document.getElementById('canvas-height');
        const resizeBtn = document.getElementById('resize-btn');

        const fillBtn = document.getElementById('fill-btn');
        const clearBtn = document.getElementById('clear-btn');
        const eraserBtn = document.getElementById('eraser-btn');
        const brushBtn = document.getElementById('brush-btn');
        const downloadBtn = document.getElementById('download-btn');

        let isDrawing = false;
        let lastX = 0;
        let lastY = 0;
        let isEraser = false;
        let brushColor = '#ffffff';
        let backgroundColor = '#000000';

        // --- Initialization ---
        function initCanvas(w, h) {
            // Set canvas dimensions
            canvas.width = w;
            canvas.height = h;
            
            // Update inputs to match
            widthInput.value = w;
            heightInput.value = h;
            
            // Fill with default background
            ctx.fillStyle = backgroundColor;
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            
            updateContextStyles();
        }

        function updateContextStyles() {
            ctx.lineCap = 'round';
            ctx.lineJoin = 'round';
            ctx.lineWidth = sizePicker.value;
            // If erasing, paint with the background color
            ctx.strokeStyle = isEraser ? backgroundColor : colorPicker.value;
        }

        // --- Initial Setup ---
        // Start with 800x600 default
        initCanvas(800, 600);

        // --- Resize Logic ---
        resizeBtn.addEventListener('click', () => {
            const w = parseInt(widthInput.value);
            const h = parseInt(heightInput.value);
            
            if (w && h) {
                if (confirm("Resizing will clear the current drawing. Continue?")) {
                    // Create temp canvas to try and preserve content
                    const tempCanvas = document.createElement('canvas');
                    tempCanvas.width = canvas.width;
                    tempCanvas.height = canvas.height;
                    const tempCtx = tempCanvas.getContext('2d');
                    tempCtx.drawImage(canvas, 0, 0);
                    
                    // Resize
                    canvas.width = w;
                    canvas.height = h;
                    
                    // Refill background
                    ctx.fillStyle = backgroundColor;
                    ctx.fillRect(0, 0, canvas.width, canvas.height);
                    
                    // Draw old content back (top-left aligned)
                    ctx.drawImage(tempCanvas, 0, 0);
                    
                    updateContextStyles();
                }
            } else {
                alert("Invalid dimensions");
            }
        });

        // Fill the canvas with the current background color
        function fillCanvas() {
            ctx.fillStyle = backgroundColor;
            ctx.fillRect(0, 0, canvas.width, canvas.height);
        }

        // --- Drawing Logic ---
        function getPos(e) {
            const rect = canvas.getBoundingClientRect();
            // Handle scale if CSS width != internal width
            const scaleX = canvas.width / rect.width;
            const scaleY = canvas.height / rect.height;
            
            return {
                x: (e.clientX - rect.left) * scaleX,
                y: (e.clientY - rect.top) * scaleY
            };
        }

        function draw(e) {
            if (!isDrawing) return;
            
            const pos = getPos(e);

            ctx.beginPath();
            ctx.moveTo(lastX, lastY);
            ctx.lineTo(pos.x, pos.y);
            ctx.stroke();

            [lastX, lastY] = [pos.x, pos.y];
        }

        canvas.addEventListener('mousedown', (e) => {
            isDrawing = true;
            const pos = getPos(e);
            [lastX, lastY] = [pos.x, pos.y];
            draw(e); 
        });

        canvas.addEventListener('mousemove', draw);
        canvas.addEventListener('mouseup', () => isDrawing = false);
        canvas.addEventListener('mouseout', () => isDrawing = false);

        // --- Image Opening Logic ---
        openImgBtn.addEventListener('click', () => {
            imageLoader.click();
        });

        imageLoader.addEventListener('change', (e) => {
            const file = e.target.files[0];
            if (!file) return;
            
            const reader = new FileReader();
            reader.onload = (event) => {
                const img = new Image();
                img.onload = () => {
                    if(confirm('Load image? This will replace the current canvas.')) {
                        // Resize canvas to match image?
                        if(confirm('Resize canvas to match image dimensions?')) {
                            canvas.width = img.width;
                            canvas.height = img.height;
                            widthInput.value = img.width;
                            heightInput.value = img.height;
                        }
                        
                        // Clear with background color first
                        fillCanvas();
                        
                        // Draw image
                        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
                        updateContextStyles();
                    }
                };
                img.src = event.target.result;
            };
            reader.readAsDataURL(file);
            imageLoader.value = '';
        });


        // --- Controls ---
        colorPicker.addEventListener('change', (e) => {
            isEraser = false;
            brushColor = e.target.value;
            updateContextStyles();
        });

        bgColorPicker.addEventListener('change', (e) => {
            backgroundColor = e.target.value;
            updateContextStyles(); 
        });

        fillBtn.addEventListener('click', () => {
            if(confirm('Fill entire canvas?')) {
                fillCanvas();
            }
        });

        sizePicker.addEventListener('input', (e) => {
            sizeVal.textContent = e.target.value;
            updateContextStyles();
        });

        eraserBtn.addEventListener('click', () => {
            isEraser = true;
            updateContextStyles();
        });

        brushBtn.addEventListener('click', () => {
            isEraser = false;
            colorPicker.value = brushColor; 
            updateContextStyles();
        });

        clearBtn.addEventListener('click', () => {
            if(confirm('Clear canvas?')) {
                fillCanvas(); // Clear by refilling with background
            }
        });

        downloadBtn.addEventListener('click', () => {
            const link = document.createElement('a');
            link.download = 'my-painting.png';
            link.href = canvas.toDataURL();
            link.click();
        });

    </script>
    "##;

    render_base_page("Paint Tool", &format!("{}{}", style, html_content), current_theme)
}