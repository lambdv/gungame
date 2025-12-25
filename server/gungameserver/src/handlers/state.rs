use std::collections::HashMap;

pub type LobbyCode = String;

pub struct ServerState{
    pub lobbies: HashMap<LobbyCode, Lobby>,
    next_player_id: u32,
}

impl ServerState{
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
            next_player_id: 1,
        }
    }

    pub fn create_lobby(&mut self, code: String, max_players: u32, scene: Scene) -> Result<&Lobby, &'static str> {
}

#[derive(Debug)]
pub struct Lobby {
    pub code: LobbyCode,
    pub players: HashMap<u32, Player>,
    pub max_players: u32,
    pub dummy_player: Option<Player>,
    pub client_addresses: HashMap<u32, std::net::SocketAddr>, // player_id -> client address
    pub scene: String,
}

pub enum Scene{
    World
}

#[derive(Debug, Clone)]
pub struct ClientConnection{
    pub address: std::net::SocketAddr,
    pub player_id: u32,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub position: (f32, f32, f32),
    pub rotation: (f32, f32, f32),
    pub last_update: SystemTime,

    // Health state
    pub current_health: u32,
    pub max_health: u32,

    // Weapon and ammo state
    pub current_weapon_id: u32,
    pub current_ammo: u32,
    pub max_ammo: u32,

    // Reload state
    pub is_reloading: bool,
    pub reload_end_time: Option<SystemTime>,

    // Combat timing
    pub last_shot_time: SystemTime,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponData {
    pub id: u32,
    pub name: String,
    pub damage: u32,
    pub fire_rate: f32,
    pub range: f32,
    pub reload_time: f32,
    pub ammo: u32,
}
