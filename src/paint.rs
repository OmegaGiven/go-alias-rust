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
    // CSS uses single braces { } in raw string, so we format! with {{ }} escaping if needed, 
    // or just use a raw string if we aren't injecting variables. 
    // Since we aren't injecting into CSS here, a plain raw string is fine, but format! expects escaping.
    // We'll use a simple raw string for CSS since no variables are injected.
    let style = r#"
<style>
    .paint-app {
        display: flex;
        flex-direction: column;
        gap: 10px;
        height: calc(100vh - 80px); /* Fill remaining screen height */
    }
    
    .toolbar {
        display: flex;
        gap: 10px;
        align-items: center;
        padding: 8px;
        background-color: var(--secondary-bg);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        flex-wrap: wrap;
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
        background-color: #000000; /* Default to black for dark theme preference */
        border: 1px solid var(--border-color);
        border-radius: 8px;
        overflow: hidden;
        position: relative;
        cursor: crosshair;
    }
    
    canvas {
        display: block;
        touch-action: none; /* Prevent scrolling while drawing on touch devices */
    }

    label {
        font-weight: bold;
        margin-right: 3px;
        color: var(--text-color);
        font-size: 0.9rem;
    }

    input[type="color"] {
        border: none;
        width: 30px;
        height: 30px;
        cursor: pointer;
        background: none;
        padding: 0;
    }

    input[type="range"] {
        cursor: pointer;
        height: 10px;
    }
    
    button {
        padding: 4px 8px;
        font-size: 0.9rem;
        cursor: pointer;
        background-color: var(--tertiary-bg);
        color: var(--text-color);
        border: 1px solid var(--border-color);
        border-radius: 4px;
        transition: background-color 0.2s;
    }
    button:hover {
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

    // FIX: Use r##" delimiters so that "#" inside color strings doesn't end the string early.
    let html_content = r##"
    <div class="paint-app">
        <div class="toolbar">
            <!-- Hidden file input for image loading -->
            <input type="file" id="image-loader" accept="image/png, image/jpeg, image/jpg" style="display: none;">

            <div class="tool-group">
                <label for="color">Brush:</label>
                <input type="color" id="color" value="#ffffff">
            </div>

            <div class="tool-group">
                <label for="bg-color">Background:</label>
                <input type="color" id="bg-color" value="#000000">
                <button id="fill-btn" title="Fill entire canvas with background color">Fill</button>
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
        function resizeCanvas() {
            // Save current content
            const tempCanvas = document.createElement('canvas');
            const tempCtx = tempCanvas.getContext('2d');
            tempCanvas.width = canvas.width;
            tempCanvas.height = canvas.height;
            tempCtx.drawImage(canvas, 0, 0);

            // Resize
            canvas.width = container.clientWidth;
            canvas.height = container.clientHeight;

            // Restore content
            ctx.drawImage(tempCanvas, 0, 0);
            
            // Update styles after resize
            updateContextStyles();
        }

        // Fill the canvas with the current background color
        function fillCanvas() {
            ctx.fillStyle = backgroundColor;
            ctx.fillRect(0, 0, canvas.width, canvas.height);
        }

        function updateContextStyles() {
            ctx.lineCap = 'round';
            ctx.lineJoin = 'round';
            ctx.lineWidth = sizePicker.value;
            // If erasing, paint with the background color
            ctx.strokeStyle = isEraser ? backgroundColor : colorPicker.value;
        }

        window.addEventListener('resize', resizeCanvas);
        
        // Initial setup
        setTimeout(() => {
            resizeCanvas();
            fillCanvas(); // Start with black background
        }, 10); 

        // --- Drawing Logic ---
        function draw(e) {
            if (!isDrawing) return;
            
            // Calculate mouse position relative to canvas
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;

            ctx.beginPath();
            ctx.moveTo(lastX, lastY);
            ctx.lineTo(x, y);
            ctx.stroke();

            [lastX, lastY] = [x, y];
        }

        canvas.addEventListener('mousedown', (e) => {
            isDrawing = true;
            const rect = canvas.getBoundingClientRect();
            [lastX, lastY] = [e.clientX - rect.left, e.clientY - rect.top];
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
                    if(confirm('Load image? This will clear the current canvas.')) {
                        // Clear with background color first
                        fillCanvas();
                        
                        // Calculate scaling to fit image within canvas if it's too large
                        let dw = img.width;
                        let dh = img.height;
                        
                        // Scale down if larger than canvas
                        const hRatio = canvas.width / img.width;
                        const vRatio = canvas.height / img.height;
                        const ratio  = Math.min(hRatio, vRatio);
                        
                        // Only scale down, don't scale up small images (unless desired)
                        if (ratio < 1) {
                            dw = img.width * ratio;
                            dh = img.height * ratio;
                        }
                        
                        // Center the image
                        const dx = (canvas.width - dw) / 2;
                        const dy = (canvas.height - dh) / 2;
                        
                        ctx.drawImage(img, dx, dy, dw, dh);
                    }
                };
                img.src = event.target.result;
            };
            reader.readAsDataURL(file);
            // Reset input value so the same file can be selected again if needed
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
            // Update container color for consistency
            container.style.backgroundColor = backgroundColor;
            updateContextStyles(); // Update eraser color
        });

        fillBtn.addEventListener('click', () => {
            if(confirm('This will fill the entire canvas with the background color, erasing your drawing. Continue?')) {
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