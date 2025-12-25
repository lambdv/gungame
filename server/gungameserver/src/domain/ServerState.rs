use std::collections::HashMap;
use crate::domain::Lobby;
use crate::domain::WeaponData;
use crate::domain::LobbyCode;
use crate::domain::Player;
use crate::domain::PlayerSyncState;
use crate::domain::SystemTime;
use crate::domain::SystemDuration;
use crate::domain::SystemError;
use crate::domain::SystemResult;
use crate::domain::SystemVec;
use crate::domain::SystemHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ServerState - Central server state container
/// Thread-safe wrapper around all lobbies and global server state
#[derive(Debug)]
pub struct ServerState {
    lobbies: HashMap<LobbyCode, Lobby>,  // All active lobbies by code
    next_player_id: u32,  // Counter for assigning unique player IDs
    weapons: HashMap<u32, WeaponData>,  // Weapon database
}


impl ServerState {
    pub fn new() -> Self {
        let weapons = Self::load_weapon_data();
        Self {
            lobbies: HashMap::new(),
            next_player_id: 1,
            weapons,
        }
    }

    fn load_weapon_data() -> HashMap<u32, WeaponData> {
        let mut weapons = HashMap::new();

        // Hardcoded weapon data - in production, this would be loaded from a config file
        weapons.insert(1, WeaponData {
            id: 1,
            name: "Golden Friend".to_string(),
            damage: 20,
            fire_rate: 4.0,
            range: 100.0,
            reload_time: 1.0,
            ammo: 20,
        });

        weapons.insert(2, WeaponData {
            id: 2,
            name: "Prototype".to_string(),
            damage: 30,
            fire_rate: 2.0,
            range: 150.0,
            reload_time: 1.5,
            ammo: 8,
        });

        weapons.insert(3, WeaponData {
            id: 3,
            name: "Combat Knife".to_string(),
            damage: 50,
            fire_rate: 1.5,
            range: 3.0,
            reload_time: 0.0,
            ammo: 0, // Melee weapon, no ammo limit
        });

        weapons
    }

    pub fn create_lobby(&mut self, code: String, max_players: u32, scene: String) -> Result<&Lobby, &'static str> {
        if self.lobbies.contains_key(&code) {
            return Err("Lobby code already exists");
        }

        let lobby = Lobby {
            code: code.clone(),
            players: HashMap::new(),
            max_players,
            dummy_player: Some(Player {
                id: 999,
                name: "DummyBot".to_string(),
                position: (5.0, 1.0, 0.0),
                rotation: (0.0, 0.0, 0.0),
                last_update: SystemTime::now(),

                // Health
                current_health: 100,
                max_health: 100,

                // Weapon and ammo (dummy uses knife)
                current_weapon_id: 3,
                current_ammo: 0,
                max_ammo: 0,

                // Reload state
                is_reloading: false,
                reload_end_time: None,

                // Combat timing
                last_shot_time: SystemTime::now(),
            }),
            client_addresses: HashMap::new(),
            scene: scene,
        };

        let code_clone = code.clone();
        self.lobbies.insert(code, lobby);
        Ok(self.lobbies.get(&code_clone).unwrap())
    }

    pub fn join_lobby(&mut self, code: &str, player_name: String) -> Result<(u32, &Lobby), &'static str> {
        println!("HTTP JOIN REQUEST: player '{}' joining lobby '{}'", player_name, code);

        let lobby = self.lobbies.get_mut(code).ok_or("Lobby not found")?;

        if lobby.players.len() >= lobby.max_players as usize {
            println!("HTTP JOIN FAILED: Lobby '{}' is full ({} players)", code, lobby.players.len());
            return Err("Lobby is full");
        }

        // Always create a new player - no automatic reconnection logic
        // (Reconnection should be handled explicitly by the client if needed)
        let player_id = self.next_player_id;
        self.next_player_id += 1;
        println!("HTTP JOIN SUCCESS: Assigned player ID {} to '{}'", player_id, player_name);

        // New player - create fresh player entry
        // Start with default weapon (Golden Friend - id 1)
        let default_weapon_id = 1;
        let default_weapon = self.weapons.get(&default_weapon_id).unwrap();

        let player = Player {
            id: player_id,
            name: player_name.clone(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: SystemTime::now(),

            // Health
            current_health: 100,
            max_health: 100,

            // Weapon and ammo (start with default weapon)
            current_weapon_id: default_weapon_id,
            current_ammo: default_weapon.ammo,
            max_ammo: default_weapon.ammo,

            // Reload state
            is_reloading: false,
            reload_end_time: None,

            // Combat timing
            last_shot_time: SystemTime::now(),
        };

        lobby.players.insert(player_id, player);
        println!("Created new player {} with ID {}", player_name, player_id);

        Ok((player_id, self.lobbies.get(code).unwrap()))
    }

    pub fn set_player_address(&mut self, lobby_code: &str, player_id: u32, address: std::net::SocketAddr) {
        if let Some(lobby) = self.lobbies.get_mut(lobby_code) {
            lobby.client_addresses.insert(player_id, address);
        }
    }

    pub fn get_lobby(&self, code: &str) -> Option<&Lobby> {
        self.lobbies.get(code)
    }

    /// Apply damage to a player (server-authoritative)
    pub fn player_take_damage(&mut self, lobby_code: &str, player_id: u32, damage: u32) -> Result<(), &'static str> {
        let lobby = self.lobbies.get_mut(lobby_code).ok_or("Lobby not found")?;
        let player = lobby.players.get_mut(&player_id).ok_or("Player not found")?;

        // Cheat prevention: validate damage is reasonable (0 < damage <= 100)
        if damage == 0 || damage > 100 {
            return Err("Invalid damage amount");
        }

        // Apply damage with underflow protection
        player.current_health = player.current_health.saturating_sub(damage);

        Ok(())
    }

    /// Handle player shooting (server-authoritative ammo management)
    pub fn player_shoot(&mut self, lobby_code: &str, player_id: u32) -> Result<bool, &'static str> {
        let lobby = self.lobbies.get_mut(lobby_code).ok_or("Lobby not found")?;
        let player = lobby.players.get_mut(&player_id).ok_or("Player not found")?;

        // Cheat prevention: validate weapon exists
        if !self.weapons.contains_key(&player.current_weapon_id) {
            return Err("Invalid weapon");
        }

        // Check if player is reloading
        if player.is_reloading {
            return Ok(false);
        }

        // Check ammo
        if player.current_ammo == 0 {
            return Ok(false);
        }

        // Check fire rate
        let weapon = self.weapons.get(&player.current_weapon_id).unwrap();
        let now = SystemTime::now();
        let time_since_last_shot = now.duration_since(player.last_shot_time)
            .map_err(|_| "Time error")?;

        if time_since_last_shot.as_secs_f32() < (1.0 / weapon.fire_rate) {
            return Ok(false); // Too soon to shoot again
        }

        // Consume ammo (with underflow protection)
        player.current_ammo = player.current_ammo.saturating_sub(1);
        player.last_shot_time = now;

        Ok(true)
    }

    /// Start player reload (server-authoritative)
    pub fn player_start_reload(&mut self, lobby_code: &str, player_id: u32) -> Result<(), &'static str> {
        let lobby = self.lobbies.get_mut(lobby_code).ok_or("Lobby not found")?;
        let player = lobby.players.get_mut(&player_id).ok_or("Player not found")?;

        // Can't reload if already reloading or at max ammo
        if player.is_reloading || player.current_ammo == player.max_ammo {
            return Err("Cannot reload");
        }

        let weapon = self.weapons.get(&player.current_weapon_id).ok_or("Weapon not found")?;
        player.is_reloading = true;
        player.reload_end_time = Some(SystemTime::now() + std::time::Duration::from_secs_f32(weapon.reload_time));

        Ok(())
    }

    /// Check and complete finished reloads
    pub fn update_reload_states(&mut self) -> Vec<(String, u32)> {
        let now = SystemTime::now();
        let mut completed_reloads = Vec::new();

        for (lobby_code, lobby) in &mut self.lobbies {
            for player in lobby.players.values_mut() {
                if player.is_reloading {
                    if let Some(end_time) = player.reload_end_time {
                        if now >= end_time {
                            // Reload complete
                            player.current_ammo = player.max_ammo;
                            player.is_reloading = false;
                            player.reload_end_time = None;
                            completed_reloads.push((lobby_code.clone(), player.id));
                        }
                    }
                }
            }
        }

        completed_reloads
    }

    /// Get player's current state for syncing to client
    pub fn get_player_state(&self, lobby_code: &str, player_id: u32) -> Result<&Player, &'static str> {
        let lobby = self.lobbies.get(lobby_code).ok_or("Lobby not found")?;
        lobby.players.get(&player_id).ok_or("Player not found")
    }

    /// Handle player weapon switching (server-authoritative)
    pub fn player_switch_weapon(&mut self, lobby_code: &str, player_id: u32, weapon_id: u32) -> Result<(), &'static str> {
        let lobby = self.lobbies.get_mut(lobby_code).ok_or("Lobby not found")?;
        let player = lobby.players.get_mut(&player_id).ok_or("Player not found")?;

        // Validate weapon exists
        if !self.weapons.contains_key(&weapon_id) {
            return Err("Invalid weapon");
        }

        // Update player's weapon and reset ammo to max for new weapon
        let weapon = self.weapons.get(&weapon_id).unwrap();
        player.current_weapon_id = weapon_id;
        player.current_ammo = weapon.ammo;
        player.max_ammo = weapon.ammo;

        // Cancel any ongoing reload
        player.is_reloading = false;
        player.reload_end_time = None;

        Ok(())
    }

    /// Get full state sync data for all players in a lobby
    pub fn get_lobby_state_sync(&self, lobby_code: &str) -> Result<Vec<PlayerSyncState>, &'static str> {
        let lobby = self.lobbies.get(lobby_code).ok_or("Lobby not found")?;

        let mut player_states = Vec::new();
        for player in lobby.players.values() {
            player_states.push(PlayerSyncState {
                id: player.id,
                health: player.current_health,
                max_health: player.max_health,
                current_weapon_id: player.current_weapon_id,
                current_ammo: player.current_ammo,
                max_ammo: player.max_ammo,
                is_reloading: player.is_reloading,
            });
        }

        Ok(player_states)
    }

    /// Clean up inactive players and empty lobbies
    /// Returns the number of players removed and lobbies deleted
    pub fn cleanup_inactive_players_and_empty_lobbies(&mut self) -> (usize, usize) {
        let now = SystemTime::now();
        let mut players_removed = 0;
        let mut lobbies_to_remove = Vec::new();

        // First pass: remove inactive players from lobbies
        for (lobby_code, lobby) in self.lobbies.iter_mut() {
            let mut inactive_players = Vec::new();

            for (player_id, player) in &lobby.players {
                // Skip the dummy bot (ID 999)
                if *player_id == 999 {
                    continue;
                }

                if let Ok(duration) = now.duration_since(player.last_update) {
                    if duration.as_secs() > PLAYER_INACTIVITY_TIMEOUT_SECS {
                        inactive_players.push(*player_id);
                    }
                }
            }

            // Remove inactive players
            for player_id in inactive_players {
                lobby.players.remove(&player_id);
                lobby.client_addresses.remove(&player_id);
                players_removed += 1;
                println!("Removed inactive player {} from lobby {}", player_id, lobby_code);
            }

            // Check if lobby is now empty (no real players)
            let real_player_count = lobby.players.values()
                .filter(|p| p.id != 999)
                .count();

            if real_player_count == 0 {
                lobbies_to_remove.push(lobby_code.clone());
            }
        }

        // Second pass: remove empty lobbies
        let lobbies_deleted = lobbies_to_remove.len();
        for lobby_code in lobbies_to_remove {
            self.lobbies.remove(&lobby_code);
            println!("Deleted empty lobby: {}", lobby_code);
        }

        (players_removed, lobbies_deleted)
    }
}

type AppState = Arc<RwLock<GameServer>>;

// HTTP API Handlers
async fn create_lobby(
    State(state): State<AppState>,
    Json(request): Json<CreateLobbyRequest>,
) -> Result<Json<LobbyInfo>, StatusCode> {
    let mut server = state.write().await;

    let max_players = request.max_players.unwrap_or(4);
    let scene = request.scene.unwrap_or_else(|| "world".to_string());
    match server.create_lobby(request.code, max_players, scene) {
        Ok(lobby) => {
            let lobby_info = LobbyInfo {
                code: lobby.code.clone(),
                player_count: lobby.players.len(),
                max_players: lobby.max_players,
                players: lobby.players.values().map(|p| PlayerInfo {
                    id: p.id,
                    name: p.name.clone(),
                }).collect(),
                server_ip: "127.0.0.1".to_string(), // In production, get actual server IP
                udp_port: 8081,
                scene: lobby.scene.clone(),
            };
            Ok(Json(lobby_info))
        }
        Err(_) => Err(StatusCode::CONFLICT),
    }
}

async fn join_lobby(
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

async fn get_lobby(
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
