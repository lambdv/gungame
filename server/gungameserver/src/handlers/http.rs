use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use crate::handlers::models::{CreateLobbyRequest, JoinLobbyRequest, JoinLobbyResponse, LobbyInfo, PlayerInfo};
use crate::state::server_state::ServerState;
use crate::domain::lobbies;
use crate::utils::weapondb::WeaponDb;
use crate::utils::config::Config;
use std::sync::Arc;
use tokio::net::UdpSocket;

/// App state for HTTP handlers (includes server state and dependencies)
#[derive(Clone)]
pub struct AppState {
    pub state: Arc<ServerState>,
    pub weapons: Arc<WeaponDb>,
    pub config: Arc<Config>,
    pub udp_socket: Arc<UdpSocket>,
}

/// Thin HTTP handler: Create lobby
pub async fn create_lobby(
    State(app_state): State<AppState>,
    Json(request): Json<CreateLobbyRequest>,
) -> Result<Json<LobbyInfo>, StatusCode> {
    if app_state.state.lobby_exists(&request.code) {
        return Err(StatusCode::CONFLICT);
    }

    let max_players = request.max_players.unwrap_or(4);
    let scene = request.scene.unwrap_or_else(|| "world".to_string());

    // Create lobby and spawn tick loop
    if let Err(e) = crate::server::create_lobby_with_tick(
        app_state.state.clone(),
        request.code.clone(),
        max_players,
        scene.clone(),
        app_state.weapons.clone(),
        app_state.config.clone(),
        app_state.udp_socket.clone(),
    ).await {
        log::error!("Failed to create lobby: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Get lobby info
    let lobby_arc = app_state.state.get_lobby(&request.code)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let lobby = lobby_arc.read().await;
    let lobby_info = LobbyInfo {
        code: lobby.code.clone(),
        player_count: lobby.players.len(),
        max_players: lobby.max_players,
        players: lobby.players.values().map(|p| PlayerInfo {
            id: p.id,
            name: p.name.clone(),
        }).collect(),
        server_ip: "127.0.0.1".to_string(),
        udp_port: app_state.config.udp_port,
        scene: lobby.scene.clone(),
    };

    Ok(Json(lobby_info))
}

/// Thin HTTP handler: Join lobby
pub async fn join_lobby(
    State(app_state): State<AppState>,
    Path(code): Path<String>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<JoinLobbyResponse>, StatusCode> {
    let lobby_arc = app_state.state.get_lobby(&code)
        .ok_or(StatusCode::NOT_FOUND)?;

    let player_id = app_state.state.next_player_id();
    
    // Acquire lock, add player
    let mut lobby = lobby_arc.write().await;
    
    let default_weapon = WeaponDb::default_weapon_id();
    
    match lobbies::add_player(&mut lobby, player_id, request.player_name.clone(), default_weapon, &app_state.weapons) {
        Ok(()) => {
            let lobby_info = LobbyInfo {
                code: lobby.code.clone(),
                player_count: lobby.players.len(),
                max_players: lobby.max_players,
                players: lobby.players.values().map(|p| PlayerInfo {
                    id: p.id,
                    name: p.name.clone(),
                }).collect(),
                server_ip: "127.0.0.1".to_string(),
                udp_port: app_state.config.udp_port,
                scene: lobby.scene.clone(),
            };

            Ok(Json(JoinLobbyResponse {
                lobby: lobby_info,
                player_id,
            }))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Thin HTTP handler: Get lobby info
pub async fn get_lobby(
    State(app_state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<LobbyInfo>, StatusCode> {
    let lobby_arc = app_state.state.get_lobby(&code)
        .ok_or(StatusCode::NOT_FOUND)?;

    let lobby = lobby_arc.read().await;
    
    let lobby_info = LobbyInfo {
        code: lobby.code.clone(),
        player_count: lobby.players.len(),
        max_players: lobby.max_players,
        players: lobby.players.values().map(|p| PlayerInfo {
            id: p.id,
            name: p.name.clone(),
        }).collect(),
        server_ip: "127.0.0.1".to_string(),
        udp_port: app_state.config.udp_port,
        scene: lobby.scene.clone(),
    };

    Ok(Json(lobby_info))
}

/// Thin HTTP handler: List all lobbies
pub async fn list_lobbies(
    State(app_state): State<AppState>,
) -> Json<Vec<LobbyInfo>> {
    let mut lobbies_info = Vec::new();

    for entry in app_state.state.iter_lobbies() {
        let lobby = entry.lobby.read().await;
        lobbies_info.push(LobbyInfo {
            code: lobby.code.clone(),
            player_count: lobby.players.len(),
            max_players: lobby.max_players,
            players: lobby.players.values().map(|p| PlayerInfo {
                id: p.id,
                name: p.name.clone(),
            }).collect(),
            server_ip: "127.0.0.1".to_string(),
            udp_port: app_state.config.udp_port,
            scene: lobby.scene.clone(),
        });
    }

    Json(lobbies_info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::lobby::Lobby;

    // Note: HTTP handler tests would require full AppState setup
    // Integration tests are better suited for HTTP handlers
}
