use actix_web::{get, web::Data, HttpResponse, Responder};
use std::{collections::HashMap, sync::Arc};

use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;

#[get("/paint")]
pub async fn paint_get(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_paint_page(&current_theme, &saved_themes))
}

fn render_paint_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let style = r#"
<style>
    .paint-app {
        display: flex;
        flex-direction: column;
        gap: 0;
        height: calc(100vh - 80px);
        padding: 0;
    }

    .toolbar {
        display: flex;
        gap: 10px;
        align-items: center;
        padding: 8px;
        background-color: var(--secondary-bg);
        border-bottom: 1px solid var(--border-color);
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
        overflow: auto;
        position: relative;
        display: grid;
        place-items: center;
    }

    .paint-app canvas {
        display: block;
        touch-action: none;
        background-color: #000;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
        cursor: crosshair;
    }

    .paint-app canvas.select-mode {
        cursor: default;
    }

    .paint-app label {
        font-weight: bold;
        margin-right: 3px;
        color: var(--text-color);
        font-size: var(--font-size-small);
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

    .paint-app button:hover {
        background-color: var(--link-hover);
        color: white;
        border-color: var(--link-hover);
    }

    .size-display {
        min-width: 20px;
        display: inline-block;
        text-align: center;
        font-size: var(--font-size-small);
    }

    .paint-tool.active {
        border-color: var(--link-color) !important;
        box-shadow: inset 0 0 0 1px var(--link-color);
    }
</style>
    "#;

    let html_content = r##"
    <div class="paint-app">
        <div class="toolbar">
            <input type="file" id="image-loader" accept="image/png, image/jpeg, image/jpg" style="display: none;">

            <div class="tool-group">
                <label>Dimensions:</label>
                <input type="number" id="canvas-width" value="1200" title="Width">
                <span style="color:var(--text-color);">x</span>
                <input type="number" id="canvas-height" value="800" title="Height">
                <button id="resize-btn" class="btn-small btn-secondary" title="Update canvas size (clears if confirmed)">Set</button>
            </div>

            <div class="tool-group">
                <label for="color">Brush:</label>
                <input type="color" id="color" value="#ffffff">
            </div>

            <div class="tool-group">
                <label for="bg-color">BG:</label>
                <input type="color" id="bg-color" value="#000000">
                <button id="fill-btn" class="btn-small btn-secondary" title="Fill raster layer">Fill</button>
            </div>

            <div class="tool-group">
                <label for="size">Size:</label>
                <input type="range" id="size" min="1" max="50" value="5">
                <span id="size-val" class="size-display">5</span>
            </div>

            <div class="tool-group">
                <button id="select-btn" class="btn-small btn-secondary paint-tool">Select</button>
                <button id="brush-btn" class="btn-small btn-secondary paint-tool">Brush</button>
                <button id="eraser-btn" class="btn-small btn-secondary paint-tool">Eraser</button>
            </div>

            <div class="tool-group">
                <button id="rect-btn" class="btn-small btn-secondary paint-tool">Rect</button>
                <button id="ellipse-btn" class="btn-small btn-secondary paint-tool">Ellipse</button>
                <button id="line-btn" class="btn-small btn-secondary paint-tool">Line</button>
                <button id="text-btn" class="btn-small btn-secondary paint-tool">Text</button>
            </div>

            <div class="tool-group">
                <button id="undo-btn" class="btn-small btn-secondary" title="Undo (Ctrl+Z)">Undo</button>
                <button id="clear-btn" class="btn-small btn-secondary">Clear</button>
            </div>

            <div class="tool-group" style="margin-left: auto;">
                <button id="open-img-btn" class="btn-small btn-secondary">Open</button>
                <button id="download-btn" class="btn-small btn-secondary">Save</button>
            </div>
            <div id="paint-p2p-status" style="font-size: var(--font-size-small); color: #888; align-self: center;">Local paint mode</div>
        </div>

        <div class="canvas-container">
            <canvas id="drawing-board"></canvas>
        </div>
    </div>

    <script>
        const canvas = document.getElementById('drawing-board');
        const ctx = canvas.getContext('2d');
        const rasterLayer = document.createElement('canvas');
        const rasterCtx = rasterLayer.getContext('2d');

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
        const selectBtn = document.getElementById('select-btn');
        const brushBtn = document.getElementById('brush-btn');
        const eraserBtn = document.getElementById('eraser-btn');
        const rectBtn = document.getElementById('rect-btn');
        const ellipseBtn = document.getElementById('ellipse-btn');
        const lineBtn = document.getElementById('line-btn');
        const textBtn = document.getElementById('text-btn');
        const undoBtn = document.getElementById('undo-btn');
        const downloadBtn = document.getElementById('download-btn');

        let isDrawing = false;
        let lastX = 0;
        let lastY = 0;
        let isEraser = false;
        let brushColor = '#ffffff';
        let backgroundColor = '#000000';
        let paintReadonly = false;
        let syncTimer = null;

        let activeTool = 'brush';
        let shapeStart = null;
        let previewObject = null;
        let dragObject = null;
        let lineStartPoint = null;

        let objects = [];
        let selectedObjectId = null;
        let nextObjectId = 1;

        const undoStack = [];
        const MAX_UNDO_STEPS = 20;

        function setPaintStatus(text) {
            const el = document.getElementById('paint-p2p-status');
            if (el) el.textContent = text;
        }

        function setPaintReadonly(nextReadonly) {
            paintReadonly = !!nextReadonly;
            document.querySelectorAll('.toolbar input, .toolbar button').forEach((el) => {
                if (el.id === 'download-btn') return;
                el.disabled = paintReadonly;
            });
        }

        function scheduleBroadcast() {
            if (paintReadonly) return;
            if (syncTimer) clearTimeout(syncTimer);
            syncTimer = setTimeout(() => broadcastPaintState(), 200);
        }

        function broadcastPaintState() {
            if (!window.p2p || !window.p2p.isConnected() || paintReadonly) return;
            window.p2p.sendToolState('paint', {
                width: canvas.width,
                height: canvas.height,
                backgroundColor,
                imageData: canvas.toDataURL('image/png')
            });
        }

        function applyRemotePaintState(payload) {
            if (!payload || !payload.imageData) return;
            setPaintReadonly(true);
            setPaintStatus('Connected: viewing shared canvas');

            if (payload.backgroundColor) backgroundColor = payload.backgroundColor;
            if (payload.width && payload.height) {
                canvas.width = payload.width;
                canvas.height = payload.height;
                rasterLayer.width = payload.width;
                rasterLayer.height = payload.height;
                widthInput.value = payload.width;
                heightInput.value = payload.height;
            }

            const img = new Image();
            img.onload = () => {
                rasterCtx.clearRect(0, 0, canvas.width, canvas.height);
                rasterCtx.drawImage(img, 0, 0, canvas.width, canvas.height);
                objects = [];
                selectedObjectId = null;
                renderAll();
            };
            img.src = payload.imageData;
        }

        function initPaintP2P() {
            if (!window.p2p) return;
            const info = window.p2p.getSessionInfo();

            if (info.connected && info.role === 'guest') {
                setPaintReadonly(true);
                setPaintStatus('Connected: viewing shared canvas');
                window.p2p.sendToolMessage('paint', 'request_state', {});
            } else if (info.connected) {
                setPaintReadonly(false);
                setPaintStatus('Connected: sharing local canvas');
                setTimeout(() => broadcastPaintState(), 300);
            } else {
                setPaintStatus('Local paint mode');
            }

            window.addEventListener('p2p-status', (event) => {
                const state = event.detail || {};
                if (!state.connected) {
                    setPaintReadonly(false);
                    setPaintStatus('Local paint mode');
                    return;
                }

                if (state.role === 'guest') {
                    setPaintReadonly(true);
                    setPaintStatus('Connected: viewing shared canvas');
                    if (window.p2p) window.p2p.sendToolMessage('paint', 'request_state', {});
                } else {
                    setPaintReadonly(false);
                    setPaintStatus('Connected: sharing local canvas');
                    broadcastPaintState();
                }
            });

            window.addEventListener('p2p-message', (event) => {
                const msg = event.detail || {};
                if (msg.type !== 'tool' || msg.tool !== 'paint') return;
                if (msg.action === 'request_state') {
                    if (!paintReadonly) broadcastPaintState();
                    return;
                }
                if (msg.action === 'state') applyRemotePaintState(msg.payload);
            });
        }

        function deepClone(v) {
            return JSON.parse(JSON.stringify(v));
        }

        function snapshotState() {
            return {
                rasterData: rasterLayer.toDataURL(),
                objects: deepClone(objects),
                nextObjectId
            };
        }

        function saveState() {
            if (undoStack.length >= MAX_UNDO_STEPS) undoStack.shift();
            undoStack.push(snapshotState());
        }

        function restoreSnapshot(snapshot) {
            if (!snapshot) return;
            const img = new Image();
            img.onload = () => {
                rasterCtx.clearRect(0, 0, canvas.width, canvas.height);
                rasterCtx.drawImage(img, 0, 0, canvas.width, canvas.height);
                objects = deepClone(snapshot.objects || []);
                nextObjectId = snapshot.nextObjectId || (objects.length + 1);
                selectedObjectId = null;
                renderAll();
                scheduleBroadcast();
            };
            img.src = snapshot.rasterData;
        }

        function undo() {
            if (paintReadonly || undoStack.length === 0) return;
            restoreSnapshot(undoStack.pop());
        }

        function initCanvas(w, h) {
            canvas.width = w;
            canvas.height = h;
            rasterLayer.width = w;
            rasterLayer.height = h;
            widthInput.value = w;
            heightInput.value = h;
            rasterCtx.fillStyle = backgroundColor;
            rasterCtx.fillRect(0, 0, canvas.width, canvas.height);
            objects = [];
            selectedObjectId = null;
            nextObjectId = 1;
            renderAll();
            updateRasterBrushStyles();
            scheduleBroadcast();
        }

        function updateRasterBrushStyles() {
            rasterCtx.lineCap = 'round';
            rasterCtx.lineJoin = 'round';
            rasterCtx.lineWidth = Number(sizePicker.value);
            rasterCtx.strokeStyle = isEraser ? backgroundColor : colorPicker.value;
        }

        function setActiveTool(tool) {
            activeTool = tool;
            isEraser = tool === 'eraser';
            canvas.classList.toggle('select-mode', tool === 'select');
            if (tool !== 'line') {
                lineStartPoint = null;
                previewObject = null;
                renderAll();
            }
            [selectBtn, brushBtn, eraserBtn, rectBtn, ellipseBtn, lineBtn, textBtn].forEach((btn) => {
                if (!btn) return;
                btn.classList.toggle('active', btn.id === `${tool}-btn`);
            });
            updateRasterBrushStyles();
        }

        function getPos(e) {
            const rect = canvas.getBoundingClientRect();
            const scaleX = canvas.width / rect.width;
            const scaleY = canvas.height / rect.height;
            return {
                x: (e.clientX - rect.left) * scaleX,
                y: (e.clientY - rect.top) * scaleY
            };
        }

        function normalizedRect(a, b) {
            return {
                x: Math.min(a.x, b.x),
                y: Math.min(a.y, b.y),
                w: Math.abs(b.x - a.x),
                h: Math.abs(b.y - a.y)
            };
        }

        function shapeDraft(start, end) {
            const r = normalizedRect(start, end);
            if (r.w < 2 || r.h < 2) return null;
            return {
                id: `obj_${nextObjectId}`,
                type: activeTool,
                x: r.x,
                y: r.y,
                w: r.w,
                h: r.h,
                stroke: colorPicker.value,
                lineWidth: Number(sizePicker.value) || 2,
                text: '',
                fontSize: Math.max(12, Number(sizePicker.value) * 2.5)
            };
        }

        function lineDraft(start, end) {
            const dx = end.x - start.x;
            const dy = end.y - start.y;
            if (Math.hypot(dx, dy) < 2) return null;
            return {
                id: `obj_${nextObjectId}`,
                type: 'line',
                x1: start.x,
                y1: start.y,
                x2: end.x,
                y2: end.y,
                stroke: colorPicker.value,
                lineWidth: Number(sizePicker.value) || 2
            };
        }

        function measureTextBox(text, fontSize) {
            ctx.save();
            ctx.font = `${fontSize}px var(--base-font-family, sans-serif)`;
            const w = Math.max(40, ctx.measureText(text || '').width);
            ctx.restore();
            return { w, h: fontSize * 1.4 };
        }

        function drawObject(o, selected) {
            ctx.save();
            const lineWidth = o.lineWidth || 2;
            const stroke = o.stroke || '#ffffff';
            ctx.lineWidth = lineWidth;
            ctx.strokeStyle = stroke;
            ctx.fillStyle = stroke;

            if (o.type === 'rect') {
                ctx.beginPath();
                ctx.rect(o.x, o.y, o.w, o.h);
                ctx.stroke();
            } else if (o.type === 'ellipse') {
                ctx.beginPath();
                ctx.ellipse(o.x + o.w / 2, o.y + o.h / 2, Math.max(1, o.w / 2), Math.max(1, o.h / 2), 0, 0, Math.PI * 2);
                ctx.stroke();
            } else if (o.type === 'line') {
                ctx.beginPath();
                ctx.moveTo(o.x1, o.y1);
                ctx.lineTo(o.x2, o.y2);
                ctx.stroke();
            } else if (o.type === 'text') {
                ctx.font = `${o.fontSize || 14}px var(--base-font-family, sans-serif)`;
                ctx.textAlign = 'left';
                ctx.textBaseline = 'top';
                ctx.fillText(o.text || '', o.x, o.y);
            }

            if ((o.type === 'rect' || o.type === 'ellipse') && o.text) {
                ctx.font = `${o.fontSize || 14}px var(--base-font-family, sans-serif)`;
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(o.text, o.x + o.w / 2, o.y + o.h / 2);
            }

            if (selected) {
                const pad = 3;
                ctx.setLineDash([5, 3]);
                ctx.lineWidth = 1;
                ctx.strokeStyle = '#4da6ff';
                if (o.type === 'line') {
                    const minX = Math.min(o.x1, o.x2);
                    const minY = Math.min(o.y1, o.y2);
                    const w = Math.abs(o.x2 - o.x1);
                    const h = Math.abs(o.y2 - o.y1);
                    ctx.strokeRect(minX - pad, minY - pad, w + pad * 2, h + pad * 2);
                } else {
                    const w = o.type === 'text' ? (o.w || Math.max(40, (o.text || '').length * ((o.fontSize || 14) * 0.6))) : o.w;
                    const h = o.type === 'text' ? (o.h || (o.fontSize || 14) * 1.4) : o.h;
                    ctx.strokeRect(o.x - pad, o.y - pad, w + pad * 2, h + pad * 2);
                }
            }
            ctx.restore();
        }

        function renderAll(tempObj = null) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            ctx.drawImage(rasterLayer, 0, 0);
            for (const o of objects) drawObject(o, o.id === selectedObjectId);
            if (tempObj) drawObject(tempObj, false);
        }

        function hitObject(pos) {
            function pointToSegmentDistance(px, py, x1, y1, x2, y2) {
                const vx = x2 - x1;
                const vy = y2 - y1;
                const wx = px - x1;
                const wy = py - y1;
                const c1 = vx * wx + vy * wy;
                if (c1 <= 0) return Math.hypot(px - x1, py - y1);
                const c2 = vx * vx + vy * vy;
                if (c2 <= c1) return Math.hypot(px - x2, py - y2);
                const b = c1 / c2;
                const bx = x1 + b * vx;
                const by = y1 + b * vy;
                return Math.hypot(px - bx, py - by);
            }

            for (let i = objects.length - 1; i >= 0; i--) {
                const o = objects[i];
                if (o.type === 'text') {
                    const w = o.w || Math.max(40, (o.text || '').length * ((o.fontSize || 14) * 0.6));
                    const h = o.h || (o.fontSize || 14) * 1.4;
                    if (pos.x >= o.x && pos.x <= o.x + w && pos.y >= o.y && pos.y <= o.y + h) return o;
                } else if (o.type === 'rect') {
                    if (pos.x >= o.x && pos.x <= o.x + o.w && pos.y >= o.y && pos.y <= o.y + o.h) return o;
                } else if (o.type === 'ellipse') {
                    const rx = o.w / 2;
                    const ry = o.h / 2;
                    const cx = o.x + rx;
                    const cy = o.y + ry;
                    const dx = (pos.x - cx) / Math.max(rx, 1);
                    const dy = (pos.y - cy) / Math.max(ry, 1);
                    if ((dx * dx + dy * dy) <= 1) return o;
                } else if (o.type === 'line') {
                    const threshold = Math.max(5, (o.lineWidth || 2) + 2);
                    const d = pointToSegmentDistance(pos.x, pos.y, o.x1, o.y1, o.x2, o.y2);
                    if (d <= threshold) return o;
                }
            }
            return null;
        }

        function fillRaster() {
            rasterCtx.fillStyle = backgroundColor;
            rasterCtx.fillRect(0, 0, canvas.width, canvas.height);
            renderAll();
        }

        function drawBrushStroke(e) {
            const pos = getPos(e);
            rasterCtx.beginPath();
            rasterCtx.moveTo(lastX, lastY);
            rasterCtx.lineTo(pos.x, pos.y);
            rasterCtx.stroke();
            [lastX, lastY] = [pos.x, pos.y];
            renderAll();
        }

        canvas.addEventListener('mousedown', (e) => {
            if (paintReadonly) return;
            const pos = getPos(e);

            if (activeTool === 'select') {
                const hit = hitObject(pos);
                selectedObjectId = hit ? hit.id : null;
                if (hit) {
                    saveState();
                    if (hit.type === 'line') {
                        dragObject = {
                            id: hit.id,
                            anchorX: pos.x,
                            anchorY: pos.y,
                            startX1: hit.x1,
                            startY1: hit.y1,
                            startX2: hit.x2,
                            startY2: hit.y2
                        };
                    } else {
                        dragObject = { id: hit.id, dx: pos.x - hit.x, dy: pos.y - hit.y };
                    }
                    isDrawing = true;
                } else {
                    dragObject = null;
                    isDrawing = false;
                }
                renderAll();
                return;
            }

            if (activeTool === 'line') {
                if (!lineStartPoint) {
                    lineStartPoint = pos;
                    previewObject = {
                        id: `obj_${nextObjectId}`,
                        type: 'line',
                        x1: pos.x,
                        y1: pos.y,
                        x2: pos.x,
                        y2: pos.y,
                        stroke: colorPicker.value,
                        lineWidth: Number(sizePicker.value) || 2
                    };
                    selectedObjectId = null;
                    renderAll(previewObject);
                } else {
                    const line = lineDraft(lineStartPoint, pos);
                    lineStartPoint = null;
                    previewObject = null;
                    if (line) {
                        saveState();
                        line.id = `obj_${nextObjectId++}`;
                        objects.push(line);
                        selectedObjectId = null;
                        renderAll();
                        scheduleBroadcast();
                    } else {
                        renderAll();
                    }
                }
                return;
            }

            if (activeTool === 'text') {
                saveState();
                const text = prompt('Enter text:', '');
                if (text) {
                    const fontSize = Math.max(12, Number(sizePicker.value) * 2.5);
                    const box = measureTextBox(text, fontSize);
                    objects.push({
                        id: `obj_${nextObjectId++}`,
                        type: 'text',
                        x: pos.x,
                        y: pos.y,
                        w: box.w,
                        h: box.h,
                        text,
                        stroke: colorPicker.value,
                        fontSize
                    });
                    selectedObjectId = null;
                    renderAll();
                    scheduleBroadcast();
                }
                return;
            }

            if (activeTool === 'rect' || activeTool === 'ellipse') {
                saveState();
                isDrawing = true;
                shapeStart = pos;
                previewObject = null;
                return;
            }

            saveState();
            isDrawing = true;
            [lastX, lastY] = [pos.x, pos.y];
            drawBrushStroke(e);
        });

        canvas.addEventListener('dblclick', (e) => {
            if (paintReadonly) return;
            const pos = getPos(e);
            const hit = hitObject(pos);
            if (!hit) return;
            if (hit.type === 'line') return;

            const editedText = prompt('Edit text:', hit.text || '');
            if (editedText === null) return;

            saveState();
            selectedObjectId = hit.id;
            hit.text = editedText;

            if (hit.type === 'text') {
                const fontSize = hit.fontSize || Math.max(12, Number(sizePicker.value) * 2.5);
                hit.fontSize = fontSize;
                const box = measureTextBox(editedText, fontSize);
                hit.w = box.w;
                hit.h = box.h;
            }

            renderAll();
            scheduleBroadcast();
        });

        canvas.addEventListener('mousemove', (e) => {
            if (paintReadonly) return;

            if (activeTool === 'line' && lineStartPoint) {
                const pos = getPos(e);
                previewObject = {
                    id: `obj_${nextObjectId}`,
                    type: 'line',
                    x1: lineStartPoint.x,
                    y1: lineStartPoint.y,
                    x2: pos.x,
                    y2: pos.y,
                    stroke: colorPicker.value,
                    lineWidth: Number(sizePicker.value) || 2
                };
                renderAll(previewObject);
                return;
            }

            if (!isDrawing) return;

            if (activeTool === 'select' && dragObject) {
                const pos = getPos(e);
                const obj = objects.find((o) => o.id === dragObject.id);
                if (!obj) return;
                if (obj.type === 'line') {
                    const dx = pos.x - dragObject.anchorX;
                    const dy = pos.y - dragObject.anchorY;
                    obj.x1 = dragObject.startX1 + dx;
                    obj.y1 = dragObject.startY1 + dy;
                    obj.x2 = dragObject.startX2 + dx;
                    obj.y2 = dragObject.startY2 + dy;
                } else {
                    obj.x = pos.x - dragObject.dx;
                    obj.y = pos.y - dragObject.dy;
                }
                renderAll();
                return;
            }

            if (activeTool === 'rect' || activeTool === 'ellipse') {
                previewObject = shapeDraft(shapeStart, getPos(e));
                renderAll(previewObject);
                return;
            }

            drawBrushStroke(e);
        });

        function finishDraw(e) {
            if (paintReadonly || !isDrawing) return;

            if (activeTool === 'select') {
                isDrawing = false;
                dragObject = null;
                renderAll();
                scheduleBroadcast();
                return;
            }

            if (activeTool === 'line') return;

            if (activeTool === 'rect' || activeTool === 'ellipse') {
                const shape = shapeDraft(shapeStart, getPos(e));
                if (shape) {
                    const label = prompt('Optional shape text (leave blank for none):', '');
                    if (label) shape.text = label;
                    shape.id = `obj_${nextObjectId++}`;
                    objects.push(shape);
                }
                shapeStart = null;
                previewObject = null;
                selectedObjectId = null;
                renderAll();
            }

            isDrawing = false;
            scheduleBroadcast();
        }

        canvas.addEventListener('mouseup', finishDraw);
        canvas.addEventListener('mouseout', (e) => {
            if (activeTool === 'brush' || activeTool === 'eraser') finishDraw(e);
            else {
                isDrawing = false;
                dragObject = null;
                if (activeTool !== 'line') {
                    previewObject = null;
                    renderAll();
                }
            }
        });

        resizeBtn.addEventListener('click', () => {
            if (paintReadonly) return;
            const w = parseInt(widthInput.value, 10);
            const h = parseInt(heightInput.value, 10);
            if (!w || !h) {
                alert('Invalid dimensions');
                return;
            }
            if (!confirm('Resizing may clip content. Continue?')) return;

            saveState();
            const old = document.createElement('canvas');
            old.width = rasterLayer.width;
            old.height = rasterLayer.height;
            old.getContext('2d').drawImage(rasterLayer, 0, 0);

            canvas.width = w;
            canvas.height = h;
            rasterLayer.width = w;
            rasterLayer.height = h;
            widthInput.value = w;
            heightInput.value = h;

            rasterCtx.fillStyle = backgroundColor;
            rasterCtx.fillRect(0, 0, w, h);
            rasterCtx.drawImage(old, 0, 0);
            updateRasterBrushStyles();
            renderAll();
            scheduleBroadcast();
        });

        openImgBtn.addEventListener('click', () => {
            if (!paintReadonly) imageLoader.click();
        });

        imageLoader.addEventListener('change', (e) => {
            if (paintReadonly) return;
            const file = e.target.files[0];
            if (!file) return;

            const reader = new FileReader();
            reader.onload = (event) => {
                const img = new Image();
                img.onload = () => {
                    if (!confirm('Load image? This will replace current raster layer.')) return;
                    saveState();
                    if (confirm('Resize canvas to image dimensions?')) {
                        canvas.width = img.width;
                        canvas.height = img.height;
                        rasterLayer.width = img.width;
                        rasterLayer.height = img.height;
                        widthInput.value = img.width;
                        heightInput.value = img.height;
                    }
                    rasterCtx.fillStyle = backgroundColor;
                    rasterCtx.fillRect(0, 0, canvas.width, canvas.height);
                    rasterCtx.drawImage(img, 0, 0, canvas.width, canvas.height);
                    objects = [];
                    selectedObjectId = null;
                    updateRasterBrushStyles();
                    renderAll();
                    scheduleBroadcast();
                };
                img.src = event.target.result;
            };
            reader.readAsDataURL(file);
            imageLoader.value = '';
        });

        fillBtn.addEventListener('click', () => {
            if (paintReadonly) return;
            if (!confirm('Fill raster layer with current BG color?')) return;
            saveState();
            fillRaster();
            scheduleBroadcast();
        });

        clearBtn.addEventListener('click', () => {
            if (paintReadonly) return;
            if (!confirm('Clear canvas and all placed objects?')) return;
            saveState();
            fillRaster();
            objects = [];
            selectedObjectId = null;
            renderAll();
            scheduleBroadcast();
        });

        colorPicker.addEventListener('change', (e) => {
            if (paintReadonly) return;
            brushColor = e.target.value;
            if (activeTool === 'eraser') setActiveTool('brush');
            updateRasterBrushStyles();
        });

        bgColorPicker.addEventListener('change', (e) => {
            if (paintReadonly) return;
            backgroundColor = e.target.value;
            updateRasterBrushStyles();
        });

        sizePicker.addEventListener('input', (e) => {
            if (paintReadonly) return;
            sizeVal.textContent = e.target.value;
            updateRasterBrushStyles();
        });

        selectBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('select'); });
        brushBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('brush'); });
        eraserBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('eraser'); });
        rectBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('rect'); });
        ellipseBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('ellipse'); });
        lineBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('line'); });
        textBtn.addEventListener('click', () => { if (!paintReadonly) setActiveTool('text'); });
        undoBtn.addEventListener('click', undo);

        downloadBtn.addEventListener('click', () => {
            const link = document.createElement('a');
            link.download = 'my-painting.png';
            link.href = canvas.toDataURL();
            link.click();
        });

        window.addEventListener('keydown', (e) => {
            if (paintReadonly) return;
            const tag = (document.activeElement && document.activeElement.tagName || '').toLowerCase();
            if (tag === 'input' || tag === 'textarea') return;

            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
                e.preventDefault();
                undo();
                return;
            }

            if ((e.key === 'Delete' || e.key === 'Backspace') && selectedObjectId) {
                e.preventDefault();
                saveState();
                objects = objects.filter((o) => o.id !== selectedObjectId);
                selectedObjectId = null;
                renderAll();
                scheduleBroadcast();
            }
        });

        initCanvas(1200, 800);
        initPaintP2P();
        setActiveTool('brush');
    </script>
    "##;

    render_base_page("Paint Tool", &format!("{}{}", style, html_content), current_theme, saved_themes)
}
