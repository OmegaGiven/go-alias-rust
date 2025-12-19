use actix_web::{get, web::Data, HttpResponse, Responder};
use std::sync::Arc;
use crate::app_state::{AppState, Theme};
use crate::base_page::render_base_page;
use std::collections::HashMap; // Added this line

#[get("/connection")]
pub async fn connection_page(state: Data<Arc<AppState>>) -> impl Responder {
    let current_theme = state.current_theme.lock().unwrap();
    let saved_themes = state.saved_themes.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(render_connection_page(&current_theme, &saved_themes))
}

fn render_connection_page(current_theme: &Theme, saved_themes: &HashMap<String, Theme>) -> String {
    let style = r#"
<style>
    .conn-container { max-width: 800px; margin: 0 auto; padding: 20px; }
    .panel { background: var(--secondary-bg); border: 1px solid var(--border-color); border-radius: 8px; padding: 20px; margin-bottom: 20px; }
    .panel h2 { margin-top: 0; border-bottom: 1px solid var(--border-color); padding-bottom: 10px; }
    
    .room-display { font-family: monospace; font-size: 1.5em; background: var(--tertiary-bg); padding: 15px; text-align: center; border-radius: 4px; margin: 15px 0; letter-spacing: 2px; border: 1px dashed var(--link-color); word-break: break-all;}
    
    /* btn removed - now using shared .btn class if defined, or btn-secondary from static/style.css */
    .btn { padding: 10px 20px; background: var(--link-color); color: #fff; border: none; border-radius: 4px; cursor: pointer; font-weight: bold; font-size: 1em; }
    .btn:hover { opacity: 0.9; }
    /* btn-secondary removed - now in static/style.css */
    .btn-action { font-size: 0.9em; padding: 5px 10px; }
    
    /* input-group removed - now using .form-group from static/style.css */
    
    .perm-table { width: 100%; border-collapse: collapse; margin-top: 10px; }
    .perm-table th, .perm-table td { text-align: left; padding: 10px; border-bottom: 1px solid var(--border-color); }
    .perm-table th { color: #888; }
    
    .status-indicator { display: inline-block; width: 10px; height: 10px; border-radius: 50%; background: #555; margin-right: 5px; }
    .status-indicator.active { background: #49cc90; box-shadow: 0 0 5px #49cc90; }
    
    .hidden { display: none; }

    .security-note { font-size: 0.8em; color: #888; margin-top: 5px; font-style: italic; }

    /* File Transfer Styles */
    .file-list { list-style: none; padding: 0; margin-top: 15px; }
    .file-item { 
        background: var(--primary-bg); 
        border: 1px solid var(--border-color); 
        padding: 10px; 
        margin-bottom: 8px; 
        border-radius: 4px; 
        display: flex; 
        justify-content: space-between; 
        align-items: center;
    }
    .file-name { font-weight: bold; }
    .file-meta { font-size: 0.8em; color: #888; margin-left: 10px; }
    .encrypted-badge { background: #49cc90; color: #000; font-size: 0.7em; padding: 2px 5px; border-radius: 4px; margin-left: 5px; font-weight: bold; }
</style>
    "#;

    let content = r#"
    <div class="conn-container">
        <h1>Connection Manager</h1>
        <p>Establish a Peer-to-Peer connection to collaborate on tools.</p>
        
        <!-- Host Section -->
        <div class="panel" id="host-panel">
            <h2>Host a Session</h2>
            <p>Click below to generate an Invite Code and Encryption Key.</p>
            
            <div id="host-info-area" class="hidden">
                <div style="margin-bottom: 15px;">
                    <label style="display:block; color:#888; margin-bottom:5px;">Room ID:</label>
                    <div id="host-room-display" class="room-display"></div>
                </div>
                
                <div style="margin-bottom: 15px;">
                    <label style="display:block; color:#888; margin-bottom:5px;">Encryption Key (Share this secretly):</label>
                    <div id="host-key-display" class="room-display" style="border-color: #ff4444; color: #ff8888; font-size: 1.2em;"></div>
                </div>
            </div>

            <button id="create-btn" class="btn" onclick="createRoom()">Generate Secure Invite</button>
            <div id="host-status" style="margin-top: 10px; color: #888;">Status: Not started</div>
            
            <div id="permissions-area" class="hidden">
                <h3>Peer Permissions</h3>
                <p style="font-size: 0.9em; color: #888;">Control what the connected peer can modify.</p>
                <table class="perm-table">
                    <tr>
                        <th>Tool</th>
                        <th>Access Level</th>
                    </tr>
                    <tr>
                        <td>Paint</td>
                        <td>
                            <select onchange="updatePerm('paint', this.value)" class="perm-select">
                                <option value="rw">Read & Write</option>
                                <option value="r">Read Only</option>
                                <option value="none">None</option>
                            </select>
                        </td>
                    </tr>
                    <tr>
                        <td>Task Board</td>
                        <td>
                            <select onchange="updatePerm('board', this.value)" class="perm-select">
                                <option value="rw">Read & Write</option>
                                <option value="r">Read Only</option>
                                <option value="none">None</option>
                            </select>
                        </td>
                    </tr>
                    <tr>
                        <td>SQL Manager</td>
                        <td>
                            <select onchange="updatePerm('sql', this.value)" class="perm-select">
                                <option value="none" selected>None (Restricted)</option>
                                <option value="r">Read Only</option>
                            </select>
                        </td>
                    </tr>
                </table>
            </div>
        </div>

        <!-- Join Section -->
        <div class="panel" id="join-panel">
            <h2>Join a Session</h2>
            <p>Enter the Room ID and Encryption Key provided by the host.</p>
            <div class="form-group">
                <label>Room ID</label>
                <input type="text" id="join-code" placeholder="Enter Invite Code">
            </div>
            <div class="form-group">
                <label>Encryption Key</label>
                <input type="password" id="join-key" placeholder="Enter Secret Key">
                <div class="security-note">This key is used to encrypt signaling traffic so the server/ISP cannot read it.</div>
            </div>
            <button class="btn" onclick="joinRoom()">Connect</button>
            <div id="join-status"></div>
        </div>

        <!-- Secure File Transfer Section -->
        <div class="panel hidden" id="transfer-panel">
            <h2>Secure File Transfer</h2>
            <p>Stage files here to send them over the encrypted P2P channel.</p>
            
            <div style="border: 2px dashed var(--border-color); padding: 20px; text-align: center; border-radius: 8px;">
                <label for="file-input" class="btn btn-secondary">Select Files or Folder</label>
                <input type="file" id="file-input" multiple webkitdirectory style="display: none;">
                <div style="margin-top: 10px; color: #888;">Selected files are encrypted immediately in the browser.</div>
            </div>

            <h3>Staged Files (Ready to Send)</h3>
            <ul id="staged-list" class="file-list">
                <li style="color: #888; font-style: italic;">No files staged.</li>
            </ul>

            <h3>Received Files</h3>
            <ul id="received-list" class="file-list">
                 <!-- Populated by JS when data received -->
            </ul>
        </div>
    </div>

    <script>
        let currentRoomId = null;
        let isHost = false;

        // --- CRYPTO HELPERS ---
        async function getCryptoKey() {
            const hex = localStorage.getItem('p2p_key');
            if (!hex) throw new Error("No encryption key found. Please Host or Join a room.");
            
            // Convert Hex to Bytes
            const bytes = new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
            
            return await window.crypto.subtle.importKey(
                "raw", bytes, { name: "AES-GCM" }, false, ["encrypt", "decrypt"]
            );
        }

        async function encryptFile(file) {
            const key = await getCryptoKey();
            const iv = window.crypto.getRandomValues(new Uint8Array(12));
            const buffer = await file.arrayBuffer();
            
            const encrypted = await window.crypto.subtle.encrypt(
                { name: "AES-GCM", iv: iv }, key, buffer
            );

            // Return IV + Encrypted Data combined
            const combined = new Uint8Array(iv.length + encrypted.byteLength);
            combined.set(iv);
            combined.set(new Uint8Array(encrypted), iv.length);
            return combined;
        }

        async function decryptFile(encryptedData) {
            const key = await getCryptoKey();
            const iv = encryptedData.slice(0, 12);
            const data = encryptedData.slice(12);

            return await window.crypto.subtle.decrypt(
                { name: "AES-GCM", iv: iv }, key, data
            );
        }
        
        // --- UI LOGIC ---

        function generateKey() {
            const array = new Uint8Array(16);
            window.crypto.getRandomValues(array);
            return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
        }

        async function createRoom() {
            const btn = document.getElementById('create-btn');
            btn.disabled = true;
            btn.innerText = "Creating...";
            
            try {
                const res = await fetch('/signal/create', { method: 'POST' });
                const data = await res.json();
                
                currentRoomId = data.room_id;
                isHost = true;
                const encryptionKey = generateKey();
                
                document.getElementById('host-room-display').innerText = currentRoomId;
                document.getElementById('host-key-display').innerText = encryptionKey;
                
                document.getElementById('host-info-area').classList.remove('hidden');
                document.getElementById('permissions-area').classList.remove('hidden');
                document.getElementById('host-status').innerHTML = '<span class="status-indicator active"></span> Room created. Waiting for peer...';
                
                // Save room ID and Key to local storage for tools to pick up
                localStorage.setItem('p2p_room_id', currentRoomId);
                localStorage.setItem('p2p_key', encryptionKey);
                localStorage.setItem('p2p_role', 'host');
                
                document.getElementById('transfer-panel').classList.remove('hidden'); // Show transfer panel
                btn.style.display = 'none';
                
            } catch (e) {
                alert("Error creating room: " + e.message);
                btn.disabled = false;
            }
        }

        async function joinRoom() {
            const code = document.getElementById('join-code').value.trim();
            const key = document.getElementById('join-key').value.trim();
            
            if (!code) return alert("Please enter a Room ID");
            if (!key) return alert("Please enter the Encryption Key");
            
            currentRoomId = code;
            isHost = false;
            
            localStorage.setItem('p2p_room_id', currentRoomId);
            localStorage.setItem('p2p_key', key);
            localStorage.setItem('p2p_role', 'guest');
            
            document.getElementById('join-status').innerHTML = '<span class="status-indicator active"></span> Credentials saved. Go to Paint/Board tool to connect.';
            document.getElementById('transfer-panel').classList.remove('hidden'); // Show transfer panel
        }

        async function updatePerm(tool, level) {
            if (!currentRoomId || !isHost) return;
            
            await fetch('/signal/permissions', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ room_id: currentRoomId, tool: tool, level: level })
            });
        }

        // --- FILE HANDLING ---
        const fileInput = document.getElementById('file-input');
        const stagedList = document.getElementById('staged-list');
        
        fileInput.addEventListener('change', async (e) => {
            const files = e.target.files;
            if (files.length > 0) {
                // Clear the "No files" message if it exists
                if(stagedList.querySelector('li').innerText === "No files staged.") {
                    stagedList.innerHTML = '';
                }
            }

            for (let file of files) {
                try {
                    // 1. Create UI Item
                    const li = document.createElement('li');
                    li.className = 'file-item';
                    li.innerHTML = `
                        <div>
                            <span class="file-name">${file.name}</span>
                            <span class="file-meta">(${(file.size / 1024).toFixed(1)} KB)</span>
                            <span id="status-${file.name}" style="color: orange; font-size: 0.8em; margin-left:10px;">Encrypting...</span>
                        </div>
                    `;
                    stagedList.appendChild(li);

                    // 2. Encrypt Content
                    const encryptedBytes = await encryptFile(file);
                    
                    // 3. Update UI
                    const statusSpan = document.getElementById(`status-${file.name}`);
                    statusSpan.innerHTML = `<span class="encrypted-badge">ENCRYPTED</span>`;
                    
                    // 4. Add "Send" Button (Placeholder for WebRTC trigger)
                    const sendBtn = document.createElement('button');
                    sendBtn.className = 'btn btn-action';
                    sendBtn.innerText = 'Stage for P2P';
                    sendBtn.onclick = () => {
                        // In a real app, this would push 'encryptedBytes' to the RTCDataChannel
                        alert(`File "${file.name}" (${encryptedBytes.length} bytes) is encrypted and ready for the data channel.`);
                        console.log("Encrypted Blob Ready:", encryptedBytes);
                    };
                    li.appendChild(sendBtn);

                } catch (err) {
                    console.error(err);
                    alert("Encryption failed for " + file.name + ": " + err.message);
                }
            }
        });
        
        // Helper to allow receiving manually (for testing/demo)
        // In full app, this is called by the data channel 'onmessage' event
        window.receiveFile = async (encryptedData, fileName) => {
            try {
                const decryptedBuffer = await decryptFile(encryptedData);
                const blob = new Blob([decryptedBuffer]);
                const url = URL.createObjectURL(blob);
                
                const ul = document.getElementById('received-list');
                const li = document.createElement('li');
                li.className = 'file-item';
                li.innerHTML = `
                    <div>
                        <span class="file-name">${fileName}</span>
                        <span class="encrypted-badge" style="background:var(--link-color); color:white;">RECEIVED</span>
                    </div>
                    <a href="${url}" download="${fileName}">
                        <button class="btn btn-action">Download</button>
                    </a>
                `;
                ul.appendChild(li);
            } catch(e) {
                console.error(e);
                alert("Failed to decrypt received file.");
            }
        };
    </script>
    "#;

    render_base_page("Connection Manager", &format!("{}{}", style, content), current_theme, saved_themes)
}