use serde::{Deserialize, Serialize};

/// HTTP Request/Response DTOs

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateLobbyRequest {
    pub code: String,
    pub max_players: Option<u32>,
    pub scene: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinLobbyRequest {
    pub player_name: String,
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

// Re-export PlayerSyncState for convenience (may be used by external code)
#[allow(unused_imports)]
pub use crate::state::lobby::PlayerSyncState;

