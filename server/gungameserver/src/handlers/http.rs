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

#[derive(serde::Serialize)]
pub struct LeaderboardEntry {
    pub player_id: u32,
    pub name: String,
    pub score: u32,
    pub kills: u32,
    pub deaths: u32,
    pub killstreak: u32,
}

#[derive(serde::Serialize)]
pub struct LeaderboardResponse {
    pub lobby_code: String,
    pub entries: Vec<LeaderboardEntry>,
}

/// Thin HTTP handler: Get lobby leaderboard
pub async fn get_lobby_leaderboard(
    State(app_state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<LeaderboardResponse>, StatusCode> {
    let lobby_arc = app_state.state.get_lobby(&code)
        .ok_or(StatusCode::NOT_FOUND)?;

    let lobby = lobby_arc.read().await;

    let mut entries: Vec<LeaderboardEntry> = lobby.players.values()
        .filter(|p| p.id != 999) // Exclude dummy bot
        .map(|p| LeaderboardEntry {
            player_id: p.id,
            name: p.name.clone(),
            score: p.score,
            kills: p.kills,
            deaths: p.deaths,
            killstreak: p.killstreak,
        })
        .collect();

    entries.sort_by(|a, b| b.score.cmp(&a.score));

    Ok(Json(LeaderboardResponse {
        lobby_code: code,
        entries,
    }))
}

#[derive(serde::Serialize)]
pub struct PlayerStats {
    pub player_id: u32,
    pub name: String,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub total_score: u32,
    pub kdratio: f32,
}

/// Thin HTTP handler: Get player stats
pub async fn get_player_stats(
    State(app_state): State<AppState>,
    Path((_code, player_id)): Path<(String, u32)>,
) -> Result<Json<PlayerStats>, StatusCode> {
    let lobby_arc = app_state.state.get_lobby(&_code)
        .ok_or(StatusCode::NOT_FOUND)?;

    let lobby = lobby_arc.read().await;

    let player = lobby.players.get(&player_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let kdratio = if player.deaths > 0 {
        player.kills as f32 / player.deaths as f32
    } else {
        player.kills as f32
    };

    Ok(Json(PlayerStats {
        player_id: player.id,
        name: player.name.clone(),
        total_kills: player.kills,
        total_deaths: player.deaths,
        total_score: player.score,
        kdratio,
    }))
}

#[derive(serde::Serialize)]
pub struct GlobalLeaderboardEntry {
    pub player_id: u32,
    pub name: String,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub total_score: u32,
    pub games_played: u32,
    pub kdratio: f32,
}

/// Thin HTTP handler: Get global leaderboard (across all sessions)
pub async fn get_global_leaderboard(
    State(app_state): State<AppState>,
) -> Json<Vec<GlobalLeaderboardEntry>> {
    let top_players = app_state.state.global_stats.get_top_players(20);

    let entries: Vec<GlobalLeaderboardEntry> = top_players.iter()
        .map(|stats| {
            let kdratio = if stats.total_deaths > 0 {
                stats.total_kills as f32 / stats.total_deaths as f32
            } else {
                stats.total_kills as f32
            };

            GlobalLeaderboardEntry {
                player_id: stats.player_id,
                name: stats.name.clone(),
                total_kills: stats.total_kills,
                total_deaths: stats.total_deaths,
                total_score: stats.total_score,
                games_played: stats.games_played,
                kdratio,
            }
        })
        .collect();

    Json(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::lobby::Lobby;

    // Note: HTTP handler tests would require full AppState setup
    // Integration tests are better suited for HTTP handlers
}
