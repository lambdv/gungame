use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLobbyRequest {
    pub code: String,
    pub max_players: Option<u32>,
    pub scene: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinLobbyResponse {
    pub lobby: LobbyInfo,
    pub player_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub code: String,
    pub player_count: usize,
    pub max_players: u32,
    pub players: Vec<PlayerInfo>,
    pub server_ip: String,
    pub udp_port: u16,
    pub scene: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u32,
    pub name: String,
}
