use serde::{Deserialize, Serialize};

// UDP Game Types
#[derive(Serialize, Deserialize, Debug, Clone)]
enum ClientMessage {
    Position { x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32 },
    Shoot { target_id: u32 },
    TakeDamage { damage: u32, attacker_id: u32 }, // For server validation
    Reload {},
    RequestState {}, // Client requesting their current state
    WeaponSwitch { weapon_id: u32 }, // Client switching weapons
}

#[derive(Serialize, Deserialize, Debug)]
enum ServerMessage {
    UpdatePosition { client_id: u32, x: f32, y: f32, z: f32, rx: f32, ry: f32, rz: f32 },
    PlayerShot { client_id: u32, target_id: u32 },
    PlayerStateUpdate {
        client_id: u32,
        health: u32,
        max_health: u32,
        ammo: u32,
        max_ammo: u32,
        is_reloading: bool,
        weapon_id: u32,
    },
    PlayerDamaged { client_id: u32, damage: u32, attacker_id: u32 },
    ReloadStarted { client_id: u32 },
    ReloadFinished { client_id: u32 },
    StateSync {  // Periodic full state synchronization for all players
        players: Vec<PlayerSyncState>,
    },
    WeaponSwitched { client_id: u32, weapon_id: u32 }, // Player switched weapons
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSyncState {
    pub id: u32,
    pub health: u32,
    pub max_health: u32,
    pub current_weapon_id: u32,
    pub current_ammo: u32,
    pub max_ammo: u32,
    pub is_reloading: bool,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct CreateLobbyRequest {
    pub code: String,  // Unique lobby identifier
    pub max_players: Option<u32>,  // Optional player limit (defaults to 4)
    pub scene: Option<String>,  // Game scene/world for this lobby
}

/// Request to join an existing lobby
#[derive(Serialize, Deserialize, Debug)]
pub struct JoinLobbyRequest {
    pub player_name: String,  // Display name for the joining player
}


#[derive(Serialize, Deserialize, Debug)]
pub struct JoinLobbyResponse {
    pub lobby: LobbyInfo,
    pub player_id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LobbyInfo {
    pub code: String,
    pub player_count: usize,
    pub max_players: u32,
    pub players: Vec<PlayerInfo>,
    pub server_ip: String,
    pub udp_port: u16,
    pub scene: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
}

// HTTP API Handlers
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use super::server::AppState;
use super::state::ServerState;

pub async fn create_lobby(
    State(state): State<AppState>,
    Json(request): Json<CreateLobbyRequest>,
) -> Result<Json<LobbyInfo>, StatusCode> {
    let mut server = state.read().await;
    let lobby_code = request.code.clone();
    let max_players = request.max_players.unwrap_or(4);
    let scene = request.scene.unwrap_or_else(|| "world".to_string());
    match server.lobbies.get_mut(&lobby_code) {
        Some(lobby) => {
            let lobby_info = LobbyInfo {
                code: lobby.code.clone(),
                player_count: lobby.players.len(),
                max_players: lobby.max_players,
                players: lobby.players.values().map(|p| PlayerInfo {
                    id: p.id,
                    name: p.name.clone(),
                }).collect(),
                server_ip: "127.0.0.1".to_string(),
                udp_port: 8081,
                scene: lobby.scene.clone(),
            };
            Ok(Json(lobby_info))
        }
        Err(_) => Err(StatusCode::CONFLICT),
    }
}

pub async fn join_lobby(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<JoinLobbyResponse>, StatusCode> {
    let mut server = state.write().await;

    match server.join_lobby(&code, request.player_name) {
        Ok((player_id, lobby)) => {
            let lobby_info = LobbyInfo {
                code: lobby.code.clone(),
                player_count: lobby.players.len(),
                max_players: lobby.max_players,
                players: lobby.players.values().map(|p| PlayerInfo {
                    id: p.id,
                    name: p.name.clone(),
                }).collect(),
                server_ip: "127.0.0.1".to_string(),
                udp_port: 8081,
                scene: lobby.scene.clone(),
            };

            let response = JoinLobbyResponse {
                lobby: lobby_info,
                player_id,
            };
            Ok(Json(response))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn get_lobby(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<LobbyInfo>, StatusCode> {
    let server = state.read().await;

    match server.get_lobby(&code) {
        Some(lobby) => {
            let lobby_info = LobbyInfo {
                code: lobby.code.clone(),
                player_count: lobby.players.len(),
                max_players: lobby.max_players,
                players: lobby.players.values().map(|p| PlayerInfo {
                    id: p.id,
                    name: p.name.clone(),
                }).collect(),
                server_ip: "127.0.0.1".to_string(),
                udp_port: 8081,
                scene: lobby.scene.clone(),
            };
            Ok(Json(lobby_info))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<LobbyInfo>> {
    let server = state.read().await;

    let lobbies: Vec<LobbyInfo> = server.lobbies.values().map(|lobby| {
        LobbyInfo {
            code: lobby.code.clone(),
            player_count: lobby.players.len(),
            max_players: lobby.max_players,
            players: lobby.players.values().map(|p| PlayerInfo {
                id: p.id,
                name: p.name.clone(),
            }).collect(),
            server_ip: "127.0.0.1".to_string(),
            udp_port: 8081,
            scene: lobby.scene.clone(),
        }
    }).collect();

    Json(lobbies)
}