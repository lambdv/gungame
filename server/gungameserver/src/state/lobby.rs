use std::collections::HashMap;
use std::time::SystemTime;
use std::net::SocketAddr;
use crate::utils::buffers::SmallPlayerVec;

pub type LobbyCode = String;

/// Player state in a lobby
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

/// Player sync state for delta tracking
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSyncState {
    pub id: u32,
    pub health: u32,
    pub max_health: u32,
    pub current_weapon_id: u32,
    pub current_ammo: u32,
    pub max_ammo: u32,
    pub is_reloading: bool,
}

impl Player {
    pub fn to_sync_state(&self) -> PlayerSyncState {
        PlayerSyncState {
            id: self.id,
            health: self.current_health,
            max_health: self.max_health,
            current_weapon_id: self.current_weapon_id,
            current_ammo: self.current_ammo,
            max_ammo: self.max_ammo,
            is_reloading: self.is_reloading,
        }
    }
}

/// Lobby state - per-lobby partitioned state
#[derive(Debug)]
pub struct Lobby {
    pub code: LobbyCode,
    pub players: HashMap<u32, Player>,
    pub client_addresses: HashMap<u32, SocketAddr>,
    pub max_players: u32,
    pub scene: String,
    
    // Delta tracking for efficient state sync
    pub dirty_players: SmallPlayerVec,  // Players with state changes
    pub last_sync_state: HashMap<u32, PlayerSyncState>,
}

impl Lobby {
    pub fn new(code: LobbyCode, max_players: u32, scene: String) -> Self {
        Self {
            code,
            players: HashMap::new(),
            client_addresses: HashMap::new(),
            max_players,
            scene,
            dirty_players: SmallPlayerVec::new(),
            last_sync_state: HashMap::new(),
        }
    }

    /// Mark a player as dirty (state changed)
    pub fn mark_dirty(&mut self, player_id: u32) {
        if !self.dirty_players.contains(&player_id) {
            self.dirty_players.push(player_id);
        }
    }

    /// Clear all dirty flags
    pub fn clear_dirty(&mut self) {
        self.dirty_players.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lobby_creation() {
        let lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        assert_eq!(lobby.code, "TEST");
        assert_eq!(lobby.max_players, 4);
        assert_eq!(lobby.players.len(), 0);
    }

    #[test]
    fn test_player_to_sync_state() {
        let player = Player {
            id: 1,
            name: "Test".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 20,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: SystemTime::now(),
        };

        let sync = player.to_sync_state();
        assert_eq!(sync.id, 1);
        assert_eq!(sync.health, 100);
        assert_eq!(sync.current_ammo, 20);
    }

    #[test]
    fn test_dirty_tracking() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        lobby.mark_dirty(1);
        lobby.mark_dirty(2);
        assert_eq!(lobby.dirty_players.len(), 2);
        assert!(lobby.dirty_players.contains(&1));
        assert!(lobby.dirty_players.contains(&2));
        
        // Duplicate should not add again
        lobby.mark_dirty(1);
        assert_eq!(lobby.dirty_players.len(), 2);
        
        lobby.clear_dirty();
        assert_eq!(lobby.dirty_players.len(), 0);
    }
}

