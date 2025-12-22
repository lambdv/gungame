//! GunGame Server - Multiplayer Game Server
//!
//! This server provides both HTTP REST API for lobby management and UDP for real-time gameplay.
//! Architecture combines Axum (web framework) for REST endpoints and Tokio UDP for game sync.
//!
//! Key features:
//! - HTTP API: Create/join lobbies, get player lists
//! - UDP sync: Real-time position updates, player join/leave events
//! - Dummy bot: Server-controlled AI player for testing
//! - Thread-safe: Uses RwLock for concurrent state access

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
// Renet imports (currently unused but kept for future use)
// use renet::{RenetServer, ServerEvent, ConnectionConfig, ChannelConfig, SendType, ClientId};
use serde::{Deserialize, Serialize};  // JSON serialization/deserialization
use std::collections::HashMap;  // For storing lobbies and players
use std::sync::Arc;  // Atomic reference counting for shared state
use std::time::SystemTime;  // For player activity timestamps
use tokio::net::UdpSocket;  // Async UDP networking
use tokio::sync::RwLock;  // Thread-safe read/write access to game state
use tokio::time::{interval, Duration};  // For periodic dummy bot updates
use tower_http::cors::CorsLayer;  // CORS middleware for web client access

// Configuration constants
const PLAYER_INACTIVITY_TIMEOUT_SECS: u64 = 15; // Remove players inactive for 15 seconds

// HTTP API Types - Data structures for REST API requests/responses

/// Request to create a new multiplayer lobby
#[derive(Serialize, Deserialize, Debug)]
struct CreateLobbyRequest {
    code: String,  // Unique lobby identifier
    max_players: Option<u32>,  // Optional player limit (defaults to 4)
    scene: Option<String>,  // Game scene/world for this lobby
}

/// Request to join an existing lobby
#[derive(Serialize, Deserialize, Debug)]
struct JoinLobbyRequest {
    player_name: String,  // Display name for the joining player
}

#[derive(Serialize, Deserialize, Debug)]
struct LobbyInfo {
    code: String,
    player_count: usize,
    max_players: u32,
    players: Vec<PlayerInfo>,
    server_ip: String,
    udp_port: u16,
    scene: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PlayerInfo {
    id: u32,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JoinLobbyResponse {
    lobby: LobbyInfo,
    player_id: u32,
}

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
    id: u32,
    health: u32,
    max_health: u32,
    current_weapon_id: u32,
    current_ammo: u32,
    max_ammo: u32,
    is_reloading: bool,
}

type LobbyCode = String;

// Weapon data structure matching client weapon.json
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

#[derive(Debug, Clone)]
pub struct Player {
    id: u32,
    name: String,
    position: (f32, f32, f32),
    rotation: (f32, f32, f32),
    last_update: SystemTime,

    // Health state
    current_health: u32,
    max_health: u32,

    // Weapon and ammo state
    current_weapon_id: u32,
    current_ammo: u32,
    max_ammo: u32,

    // Reload state
    is_reloading: bool,
    reload_end_time: Option<SystemTime>,

    // Combat timing
    last_shot_time: SystemTime,
}

#[derive(Debug)]
pub struct Lobby {
    code: LobbyCode,
    players: HashMap<u32, Player>,
    max_players: u32,
    dummy_player: Option<Player>,
    client_addresses: HashMap<u32, std::net::SocketAddr>, // player_id -> client address
    scene: String,
}

/// GameServer - Central server state container
/// Thread-safe wrapper around all lobbies and global server state
#[derive(Debug)]
pub struct GameServer {
    lobbies: HashMap<LobbyCode, Lobby>,  // All active lobbies by code
    next_player_id: u32,  // Counter for assigning unique player IDs
    weapons: HashMap<u32, WeaponData>,  // Weapon database
}

impl GameServer {
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

async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<LobbyInfo>> {
    println!("HTTP: List lobbies request received");
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

    println!("HTTP: Returning {} lobbies", lobbies.len());
    Json(lobbies)
}

/// Main server entry point
/// Sets up both HTTP REST API and UDP real-time server
/// Creates a default "test" lobby for development/testing
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize shared server state with thread-safe access
    let game_server = Arc::new(RwLock::new(GameServer::new()));

    // Create the default "test" lobby that clients can connect to immediately
    {
        let mut server = game_server.write().await;
        match server.create_lobby("test".to_string(), 8, "test_world".to_string()) {
            Ok(lobby) => {
                println!("✅ Test server 'test' created successfully");
                println!("   - Code: {}", lobby.code);
                println!("   - Max players: {}", lobby.max_players);
                println!("   - Scene: {}", lobby.scene);
            }
            Err(e) => {
                eprintln!("❌ Failed to create test server: {}", e);
            }
        }
    }

    // HTTP Server
    let app_state = game_server.clone();
    let app = Router::new()
        .route("/lobbies", post(create_lobby))
        .route("/lobbies", get(list_lobbies))
        .route("/lobbies/:code/join", post(join_lobby))
        .route("/lobbies/:code", get(get_lobby))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Start HTTP server
    let http_addr = "0.0.0.0:8080";
    println!("Starting HTTP server on {}", http_addr);
    let http_server = tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(http_addr).await {
            Ok(listener) => {
                println!("HTTP server successfully bound to {}", http_addr);
                listener
            }
            Err(e) => {
                eprintln!("Failed to bind HTTP server to {}: {}", http_addr, e);
                eprintln!("Make sure no other process is using port 8080");
                return;
            }
        };

        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    // UDP Server for real-time communication
    let udp_addr = "0.0.0.0:8081";
    println!("Starting UDP server on {}", udp_addr);

    let udp_socket = Arc::new(match UdpSocket::bind(udp_addr).await {
        Ok(socket) => {
            println!("UDP server successfully bound to {}", udp_addr);
            socket
        }
        Err(e) => {
            eprintln!("Failed to bind UDP server to {}: {}", udp_addr, e);
            eprintln!("Make sure no other process is using port 8081");
            return Err(e.into());
        }
    });
    let udp_socket_clone = udp_socket.clone();

    let game_server_clone = game_server.clone();
    let udp_server = tokio::spawn(async move {
        let _ = udp_server_loop(udp_socket, game_server_clone).await;
    });

    // Dummy player movement task
    let dummy_mover_game_server = game_server.clone();
    let dummy_mover_socket = udp_socket_clone.clone();
    let dummy_mover = tokio::spawn(async move {
        let _ = dummy_player_mover(dummy_mover_socket, dummy_mover_game_server).await;
    });

    // Player cleanup task - runs every 5 seconds
    let cleanup_game_server = game_server.clone();
    let cleanup_socket = udp_socket_clone.clone();
    let cleanup_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut server = cleanup_game_server.write().await;
            let completed_reloads = server.update_reload_states();

            // Broadcast reload finished events
            for (lobby_code, player_id) in completed_reloads {
                if let Some(lobby) = server.lobbies.get(&lobby_code) {
                    let reload_finished_packet = serde_json::json!({
                        "type": "reload_finished",
                        "player_id": player_id
                    });
                    let packet_data = serde_json::to_vec(&reload_finished_packet).unwrap();

                    // Send to all clients in the lobby
                    for (client_id, client_addr) in &lobby.client_addresses {
                        if let Err(e) = cleanup_socket.send_to(&packet_data, *client_addr).await {
                            println!("Failed to send reload finished to player {}: {:?}", client_id, e);
                        }
                    }
                }
            }

            let (players_removed, lobbies_deleted) = server.cleanup_inactive_players_and_empty_lobbies();
            if players_removed > 0 || lobbies_deleted > 0 {
                println!("Cleanup: removed {} inactive players, deleted {} empty lobbies", players_removed, lobbies_deleted);
            }
        }
    });

    // State synchronization task - runs every 500ms with change detection
    let sync_game_server = game_server.clone();
    let sync_socket = udp_socket_clone.clone();
    let sync_task = tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(500)); // 2 updates per second
        let mut last_sync_states: HashMap<String, Vec<PlayerSyncState>> = HashMap::new();

        loop {
            interval.tick().await;
            let server = sync_game_server.read().await;

            // Send state sync to all lobbies only if state changed
            for (lobby_code, lobby) in &server.lobbies {
                if let Ok(current_states) = server.get_lobby_state_sync(lobby_code) {
                    // Check if state has actually changed
                    let should_sync = if let Some(last_states) = last_sync_states.get(lobby_code) {
                        // Compare relevant state (health, ammo, weapon, reload) - exclude position/rotation
                        !current_states.iter().zip(last_states.iter()).all(|(curr, last)| {
                            curr.id == last.id &&
                            curr.health == last.health &&
                            curr.max_health == last.max_health &&
                            curr.current_weapon_id == last.current_weapon_id &&
                            curr.current_ammo == last.current_ammo &&
                            curr.max_ammo == last.max_ammo &&
                            curr.is_reloading == last.is_reloading
                        })
                    } else {
                        // First time syncing this lobby
                        true
                    };

                    if should_sync {
                        let state_sync_packet = serde_json::json!({
                            "type": "state_sync",
                            "players": current_states
                        });
                        let packet_data = serde_json::to_vec(&state_sync_packet).unwrap();

                        // Send to all clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if let Err(e) = sync_socket.send_to(&packet_data, *client_addr).await {
                                println!("Failed to send state sync to player {}: {:?}", client_id, e);
                            }
                        }

                        // Update last sync state
                        last_sync_states.insert(lobby_code.clone(), current_states);
                    }
                }
            }
        }
    });

    // Wait for all tasks
    tokio::try_join!(http_server, udp_server, dummy_mover, cleanup_task, sync_task)?;

    Ok(())
}

async fn udp_server_loop(
    socket: Arc<UdpSocket>,
    game_server: Arc<RwLock<GameServer>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        if let Ok(packet) = serde_json::from_slice::<serde_json::Value>(data) {
            handle_udp_packet(packet, addr, &socket, &game_server).await;
        }
    }
}

async fn handle_udp_packet(
    packet: serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<GameServer>>,
) {
    let packet_type = packet.get("type").and_then(|v| v.as_str());

    match packet_type {
        Some("join") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid)) = (lobby_code, player_id) {
                println!("UDP JOIN: Player {} attempting to join lobby {} from {:?}", pid, code, addr);

                let mut server = game_server.write().await;

                // Check if the player exists in the lobby first
                let player_exists = server.lobbies.get(code)
                    .and_then(|lobby| lobby.players.get(&(pid as u32)))
                    .is_some();

                println!("UDP JOIN: Processing join for player {} in lobby {}", pid, code);
                println!("UDP JOIN: Player {} existence check - exists: {}", pid, player_exists);

                if player_exists {
                    // Update client address (handles reconnection from different IP/port)
                    server.set_player_address(code, pid as u32, addr);

                    // Get player name for notifications (now that we can safely borrow again)
                    let player_name = server.lobbies.get(code)
                        .and_then(|lobby| lobby.players.get(&(pid as u32)))
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    // Send welcome message
                    let response = serde_json::json!({
                        "type": "welcome",
                        "message": "Connected to lobby"
                    });
                    println!("UDP JOIN: Sending welcome packet to player {}", pid);
                    let _ = socket.send_to(&serde_json::to_vec(&response).unwrap(), addr).await;

                        // Notify other players in the lobby about the new/reconnecting player
                        let player_joined_packet = serde_json::json!({
                            "type": "player_joined",
                            "player": {
                                "id": pid,
                                "name": player_name
                            }
                        });
                    let packet_data = serde_json::to_vec(&player_joined_packet).unwrap();

                    // Get the current lobby state for notifications
                    if let Some(lobby) = server.lobbies.get(code) {
                        println!("UDP JOIN: Notifying {} existing players about new player {}", lobby.client_addresses.len().saturating_sub(1), pid);
                        // Send to all other clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if *client_id != pid as u32 {
                                println!("UDP JOIN: Notifying player {} about new player {}", client_id, pid);
                                if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                    println!("Failed to notify player {} about player {} join: {:?}", client_id, pid, e);
                                }
                            }
                        }
                    }

                    println!("Player {} successfully joined lobby {} via UDP", pid, code);
                } else {
                    println!("Warning: Player {} not found in lobby {} during UDP join", pid, code);
                    let error_response = serde_json::json!({
                        "type": "error",
                        "message": "Player not found in lobby. Please rejoin via HTTP first."
                    });
                    let _ = socket.send_to(&serde_json::to_vec(&error_response).unwrap(), addr).await;
                }
            }
        }
        Some("leave") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid)) = (lobby_code, player_id) {
                println!("Player {} left lobby {}", pid, code);

                let mut server = game_server.write().await;

                // Remove player from lobby immediately
                if let Some(lobby) = server.lobbies.get_mut(code) {
                    let player_was_present = lobby.players.contains_key(&(pid as u32));

                    if player_was_present {
                        lobby.players.remove(&(pid as u32));
                        lobby.client_addresses.remove(&(pid as u32));

                        // Notify other players in the lobby about the leaving player
                        let player_left_packet = serde_json::json!({
                            "type": "player_left",
                            "player_id": pid
                        });
                        let packet_data = serde_json::to_vec(&player_left_packet).unwrap();

                        // Send to all remaining clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                println!("Failed to notify player {} about leaving player: {:?}", client_id, e);
                            }
                        }

                        println!("Successfully removed player {} from lobby {}", pid, code);
                    } else {
                        println!("Warning: Player {} was not found in lobby {} during leave", pid, code);
                    }
                } else {
                    println!("Warning: Lobby {} not found during leave", code);
                }
            }
        }
        Some("position_update") => {
            // Handle position and rotation updates
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());
            let pos_data = packet.get("position");
            let rot_data = packet.get("rotation");

            if let (Some(pid), Some(pos)) = (player_id, pos_data) {
                let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let z = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                // Extract rotation data
                let (rx, ry, rz) = if let Some(rot) = rot_data {
                    let rx = rot.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let ry = rot.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let rz = rot.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    (rx, ry, rz)
                } else {
                    (0.0, 0.0, 0.0)
                };

                // Update player position and rotation in server state and find lobby code
                let mut lobby_code_opt: Option<String> = None;
                {
                    let mut server = game_server.write().await;
                    for (code, lobby) in server.lobbies.iter_mut() {
                        if let Some(player) = lobby.players.get_mut(&(pid as u32)) {
                            player.position = (x, y, z);
                            player.rotation = (rx, ry, rz);
                            player.last_update = SystemTime::now();
                            lobby_code_opt = Some(code.clone());
                            break;
                        }
                    }
                }

                // Broadcast to all other players in same lobby
                if let Some(lobby_code) = lobby_code_opt {
                    let server = game_server.read().await;
                    if let Some(lobby) = server.lobbies.get(&lobby_code) {
                        let broadcast_packet = serde_json::json!({
                            "type": "position_update",
                            "player_id": pid,
                            "position": {
                                "x": x,
                                "y": y,
                                "z": z
                            },
                            "rotation": {
                                "x": rx,
                                "y": ry,
                                "z": rz
                            }
                        });

                        let packet_data = serde_json::to_vec(&broadcast_packet).unwrap();

                        // Send to all clients in the lobby except the sender
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if *client_id != pid as u32 {
                                if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                    println!("Failed to send position update to player {}: {:?}", client_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Some("shoot") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());
            let target_id = packet.get("target_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid), Some(tid)) = (lobby_code, player_id, target_id) {
                let mut server = game_server.write().await;

                // Get weapon info before doing mutable operations
                let weapon_damage = if let Some(lobby) = server.lobbies.get(code) {
                    if let Some(player) = lobby.players.get(&(pid as u32)) {
                        server.weapons.get(&player.current_weapon_id)
                            .map(|w| w.damage)
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };

                // Try to shoot (server validates ammo/fire rate)
                if let Ok(can_shoot) = server.player_shoot(code, pid as u32) {
                    if can_shoot && weapon_damage > 0 {
                        println!("Player {} shot at target {}", pid, tid);

                        // Apply damage to target
                        let _ = server.player_take_damage(code, tid as u32, weapon_damage);

                        // Broadcast shot event
                        if let Some(lobby) = server.lobbies.get(code) {
                            let shot_packet = serde_json::json!({
                                "type": "player_shot",
                                "player_id": pid,
                                "target_id": tid
                            });
                            let packet_data = serde_json::to_vec(&shot_packet).unwrap();

                            // Send to all clients in the lobby
                            for (client_id, client_addr) in &lobby.client_addresses {
                                if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                    println!("Failed to send shot event to player {}: {:?}", client_id, e);
                                }
                            }

                            // Send damage notification to target
                            if let Some(target_addr) = lobby.client_addresses.get(&(tid as u32)) {
                                let damage_packet = serde_json::json!({
                                    "type": "player_damaged",
                                    "damage": weapon_damage,
                                    "attacker_id": pid
                                });
                                let _ = socket.send_to(&serde_json::to_vec(&damage_packet).unwrap(), target_addr).await;
                            }
                        }
                    }
                }
            }
        }
        Some("reload") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid)) = (lobby_code, player_id) {
                let mut server = game_server.write().await;

                if let Ok(()) = server.player_start_reload(code, pid as u32) {
                    println!("Player {} started reload", pid);

                    // Broadcast reload started
                    if let Some(lobby) = server.lobbies.get(code) {
                        let reload_packet = serde_json::json!({
                            "type": "reload_started",
                            "player_id": pid
                        });
                        let packet_data = serde_json::to_vec(&reload_packet).unwrap();

                        // Send to all clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                println!("Failed to send reload event to player {}: {:?}", client_id, e);
                            }
                        }
                    }
                }
            }
        }
        Some("request_state") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid)) = (lobby_code, player_id) {
                let server = game_server.read().await;

                if let Ok(player) = server.get_player_state(code, pid as u32) {
                    let state_packet = serde_json::json!({
                        "type": "player_state_update",
                        "player_id": pid,
                        "health": player.current_health,
                        "max_health": player.max_health,
                        "ammo": player.current_ammo,
                        "max_ammo": player.max_ammo,
                        "is_reloading": player.is_reloading,
                        "weapon_id": player.current_weapon_id
                    });

                    let _ = socket.send_to(&serde_json::to_vec(&state_packet).unwrap(), addr).await;
                }
            }
        }
        Some("weapon_switch") => {
            let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());
            let weapon_id = packet.get("weapon_id").and_then(|v| v.as_u64());

            if let (Some(code), Some(pid), Some(wid)) = (lobby_code, player_id, weapon_id) {
                let mut server = game_server.write().await;

                if let Ok(()) = server.player_switch_weapon(code, pid as u32, wid as u32) {
                    println!("Player {} switched to weapon {}", pid, wid);

                    // Broadcast weapon switch to all clients in lobby
                    if let Some(lobby) = server.lobbies.get(code) {
                        let weapon_switch_packet = serde_json::json!({
                            "type": "weapon_switched",
                            "player_id": pid,
                            "weapon_id": wid
                        });
                        let packet_data = serde_json::to_vec(&weapon_switch_packet).unwrap();

                        // Send to all clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                println!("Failed to send weapon switch to player {}: {:?}", client_id, e);
                            }
                        }
                    }
                }
            }
        }
        _ => {
            println!("Unknown packet type: {:?}", packet_type);
        }
    }
}

async fn dummy_player_mover(socket: Arc<UdpSocket>, game_server: Arc<RwLock<GameServer>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = interval(Duration::from_millis(200)); // Update every 200ms

    loop {
        interval.tick().await;

        let mut server = game_server.write().await;

        for lobby in server.lobbies.values_mut() {
            if let Some(ref mut dummy) = lobby.dummy_player {
                // Simple AI: move in a circle around the center
                let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs_f32();
                let radius = 3.0;
                let speed = 0.5;

                dummy.position.0 = radius * (time * speed).cos(); // x
                dummy.position.2 = radius * (time * speed).sin(); // z
                dummy.position.1 = 1.0; // y (keep at ground level)

                // Send dummy position updates to all clients in this lobby
                let dummy_update = serde_json::json!({
                    "type": "server_dummy_update",
                    "position": {
                        "x": dummy.position.0,
                        "y": dummy.position.1,
                        "z": dummy.position.2
                    }
                });

                let packet_data = serde_json::to_vec(&dummy_update).unwrap();

                // Send to all clients in this lobby
                for (player_id, client_addr) in &lobby.client_addresses {
                    if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                        println!("Failed to send dummy update to player {}: {:?}", player_id, e);
                    }
                }
            }
        }
    }
}