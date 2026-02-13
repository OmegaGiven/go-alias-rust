use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::base_page::render_base_page;

#[get("/connection")]
pub async fn connection_page(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();

    let body = r#"
    <div class="container connection-container">
        <h1>P2P Connection Manager</h1>
        <p class="subtitle">Establish a direct browser-to-browser link for real-time collaboration.</p>

        <div class="connection-grid">
            <!-- HOST SECTION -->
            <div class="connection-card" id="host-card">
                <h2>Host a Room</h2>
                <p>Create a room and share the ID with a friend.</p>
                <button class="form-submit-btn" onclick="p2p.createRoom()">Create New Room</button>
                
                <div id="host-controls" style="display:none; margin-top: 20px;">
                    <div class="info-box">
                        <label>Room ID:</label>
                        <div class="copy-box">
                            <code id="room-id-display">----</code>
                            <button onclick="p2p.copyRoomId()">Copy</button>
                        </div>
                    </div>
                    <div class="status-indicator">
                        <span id="host-status">Waiting for guest...</span>
                    </div>
                </div>
            </div>

            <!-- JOIN SECTION -->
            <div class="connection-card" id="join-card">
                <h2>Join a Room</h2>
                <p>Enter a Room ID provided by a host.</p>
                <div class="input-group">
                    <input type="text" id="join-room-id" placeholder="Paste Room ID here...">
                    <button class="form-submit-btn" onclick="p2p.joinRoom()">Join Room</button>
                </div>
                <div id="join-status-container" style="display:none; margin-top: 20px;">
                    <div class="status-indicator">
                        <span id="join-status">Connecting...</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- PERMISSIONS / ACTIVE CONNECTION -->
        <div id="active-connection-area" style="display:none; margin-top: 30px;">
            <div class="connection-card wide">
                <h2>Active P2P Data Channel</h2>
                <div id="p2p-chat" class="chat-area">
                    <div id="chat-messages"></div>
                    <div class="chat-input">
                        <input type="text" id="chat-msg-input" placeholder="Send a message over P2P...">
                        <button onclick="p2p.sendMessage()">Send</button>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <style>
        .connection-container { max-width: 900px; margin: 40px auto; padding: 20px; }
        .connection-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-top: 30px; }
        .connection-card { 
            background: var(--secondary-bg); 
            border: 1px solid var(--border-color); 
            padding: 25px; 
            border-radius: 12px;
            display: flex;
            flex-direction: column;
            gap: 15px;
        }
        .connection-card.wide { grid-column: span 2; }
        .info-box { background: var(--tertiary-bg); padding: 15px; border-radius: 8px; }
        .copy-box { display: flex; justify-content: space-between; align-items: center; margin-top: 5px; }
        code { font-family: monospace; color: var(--link-color); font-size: 1.2rem; }
        .status-indicator { font-style: italic; opacity: 0.8; text-align: center; }
        .chat-area { display: flex; flex-direction: column; height: 300px; }
        #chat-messages { flex: 1; overflow-y: auto; border: 1px solid var(--border-color); margin-bottom: 10px; padding: 10px; border-radius: 4px; }
        .chat-input { display: flex; gap: 10px; }
        .chat-input input { flex: 1; }
    </style>
    "#;

    let html = render_base_page("Manage Connections", body, &current_theme, &saved_themes);
    
    // Inject the P2P JavaScript logic
    let final_html = html.replace("</body>", &format!("<script>{}</script></body>", get_p2p_js()));

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(final_html)
}

fn get_p2p_js() -> String {
    include_str!("../../static/p2p_logic.js").to_string()
}