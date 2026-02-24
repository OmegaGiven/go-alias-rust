use actix_web::{post, get, web::{Data, Json}, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::app_state::RoomState;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct RoomResponse {
    room_id: String,
    status: String,
}

#[derive(Serialize)]
pub struct RoomLookupResponse {
    exists: bool,
    has_offer: bool,
    has_answer: bool,
}

#[derive(Deserialize)]
pub struct SignalPayload {
    room_id: String,
    data: String, // SDP or ICE candidate JSON string (Encrypted on client side)
    role: String, // "host" or "guest"
}

#[derive(Deserialize)]
pub struct PermissionPayload {
    room_id: String,
    tool: String,
    level: String, // "rw", "r", "none"
}

#[derive(Deserialize)]
pub struct DisconnectPayload {
    room_id: String,
}

#[post("/signal/create")]
pub async fn signal_create(state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    // Generate a unique room id. Collisions are unlikely but guarded.
    let mut id = format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
    while rooms.contains_key(&id) {
        id = format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos());
    }
    let room = RoomState::new(id.clone());
    rooms.insert(id.clone(), room);
    
    HttpResponse::Ok().json(RoomResponse { room_id: id, status: "created".to_string() })
}

#[post("/signal/offer")]
pub async fn signal_offer(payload: Json<SignalPayload>, state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get_mut(&payload.room_id) {
        // New offer implies a fresh negotiation cycle for this room.
        room.host_offer = Some(payload.data.clone());
        room.guest_answer = None;
        room.host_ice.clear();
        room.guest_ice.clear();
        return HttpResponse::Ok().body("Offer received");
    }
    HttpResponse::NotFound().body("Room not found")
}

#[get("/signal/offer/{room_id}")]
pub async fn signal_get_offer(path: actix_web::web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get(&path.into_inner()) {
        if let Some(offer) = &room.host_offer {
            return HttpResponse::Ok().body(offer.clone());
        }
    }
    HttpResponse::NotFound().body("Offer not found")
}

#[post("/signal/answer")]
pub async fn signal_answer(payload: Json<SignalPayload>, state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get_mut(&payload.room_id) {
        room.guest_answer = Some(payload.data.clone());
        return HttpResponse::Ok().body("Answer received");
    }
    HttpResponse::NotFound().body("Room not found")
}

#[get("/signal/answer/{room_id}")]
pub async fn signal_get_answer(path: actix_web::web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get(&path.into_inner()) {
        if let Some(answer) = &room.guest_answer {
            return HttpResponse::Ok().body(answer.clone());
        }
    }
    HttpResponse::NotFound().body("Answer not found")
}

#[post("/signal/ice")]
pub async fn signal_ice(payload: Json<SignalPayload>, state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get_mut(&payload.room_id) {
        if payload.role == "host" {
            room.host_ice.push(payload.data.clone());
        } else {
            room.guest_ice.push(payload.data.clone());
        }
        return HttpResponse::Ok().body("ICE candidate received");
    }
    HttpResponse::NotFound().body("Room not found")
}

#[get("/signal/ice/{room_id}/{role}")]
pub async fn signal_get_ice(path: actix_web::web::Path<(String, String)>, state: Data<Arc<AppState>>) -> impl Responder {
    let (room_id, role) = path.into_inner();
    let mut rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get_mut(&room_id) {
        // If I am host, return guest ICE; if guest, return host ICE.
        // Drain after read to avoid re-sending the same candidates forever.
        let candidates = if role == "host" {
            std::mem::take(&mut room.guest_ice)
        } else {
            std::mem::take(&mut room.host_ice)
        };
        return HttpResponse::Ok().json(candidates);
    }
    HttpResponse::NotFound().body("Room not found")
}

#[post("/signal/permissions")]
pub async fn signal_permissions(payload: Json<PermissionPayload>, state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get_mut(&payload.room_id) {
        room.permissions.insert(payload.tool.clone(), payload.level.clone());
        return HttpResponse::Ok().json(&room.permissions);
    }
    HttpResponse::NotFound().body("Room not found")
}

#[get("/signal/permissions/{room_id}")]
pub async fn signal_get_permissions(path: actix_web::web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get(&path.into_inner()) {
        return HttpResponse::Ok().json(&room.permissions);
    }
    HttpResponse::NotFound().body("Room not found")
}

#[get("/signal/room/{room_id}")]
pub async fn signal_room_lookup(path: actix_web::web::Path<String>, state: Data<Arc<AppState>>) -> impl Responder {
    let room_id = path.into_inner();
    let rooms = state.rooms.lock().unwrap();
    if let Some(room) = rooms.get(&room_id) {
        return HttpResponse::Ok().json(RoomLookupResponse {
            exists: true,
            has_offer: room.host_offer.is_some(),
            has_answer: room.guest_answer.is_some(),
        });
    }
    HttpResponse::Ok().json(RoomLookupResponse {
        exists: false,
        has_offer: false,
        has_answer: false,
    })
}

#[post("/signal/disconnect")]
pub async fn signal_disconnect(payload: Json<DisconnectPayload>, state: Data<Arc<AppState>>) -> impl Responder {
    let mut rooms = state.rooms.lock().unwrap();
    rooms.remove(&payload.room_id);
    HttpResponse::Ok().body("Room disconnected")
}
