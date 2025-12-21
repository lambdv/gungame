use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
// Renet imports (currently unused but kept for future use)
// use renet::{RenetServer, ServerEvent, ConnectionConfig, ChannelConfig, SendType, ClientId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tower_http::cors::CorsLayer;

// HTTP API Types
#[derive(Serialize, Deserialize, Debug)]
struct CreateLobbyRequest {
    code: String,
    max_players: Option<u32>,
    scene: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JoinLobbyRequest {
    player_name: String,
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
    Position { x: f32, y: f32, z: f32 },
    Shoot { target_id: u32 },
}

#[derive(Serialize, Deserialize, Debug)]
enum ServerMessage {
    UpdatePosition { client_id: u32, x: f32, y: f32, z: f32 },
    PlayerShot { client_id: u32, target_id: u32 },
}

type LobbyCode = String;

#[derive(Debug, Clone)]
pub struct Player {
    id: u32,
    name: String,
    position: (f32, f32, f32),
    last_update: SystemTime,
}

#[derive(Debug)]
pub struct Lobby {
    code: LobbyCode,
    players: HashMap<u32, Player>,
    max_players: u32,
    dummy_player: Option<Player>,
    created_at: SystemTime,
    client_addresses: HashMap<u32, std::net::SocketAddr>, // player_id -> client address
    scene: String,
}

#[derive(Debug)]
pub struct GameServer {
    lobbies: HashMap<LobbyCode, Lobby>,
    next_player_id: u32,
}

impl GameServer {
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
            next_player_id: 1,
        }
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
                last_update: SystemTime::now(),
            }),
            created_at: SystemTime::now(),
            client_addresses: HashMap::new(),
            scene: scene,
        };

        let code_clone = code.clone();
        self.lobbies.insert(code, lobby);
        Ok(self.lobbies.get(&code_clone).unwrap())
    }

    pub fn join_lobby(&mut self, code: &str, player_name: String) -> Result<(u32, &Lobby), &'static str> {
        let lobby = self.lobbies.get_mut(code).ok_or("Lobby not found")?;

        if lobby.players.len() >= lobby.max_players as usize {
            return Err("Lobby is full");
        }

        let player_id = self.next_player_id;
        self.next_player_id += 1;

        let player = Player {
            id: player_id,
            name: player_name,
            position: (0.0, 1.0, 0.0),
            last_update: SystemTime::now(),
        };

        lobby.players.insert(player_id, player);

        Ok((player_id, self.lobbies.get(code).unwrap()))
    }

    pub fn set_player_address(&mut self, lobby_code: &str, player_id: u32, address: std::net::SocketAddr) {
        if let Some(lobby) = self.lobbies.get_mut(lobby_code) {
            lobby.client_addresses.insert(player_id, address);
        }
    }

    fn get_lobby_clients(&self, lobby_code: &str) -> Vec<(u32, std::net::SocketAddr)> {
        if let Some(lobby) = self.lobbies.get(lobby_code) {
            lobby.client_addresses.iter().map(|(id, addr)| (*id, *addr)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_lobby(&self, code: &str) -> Option<&Lobby> {
        self.lobbies.get(code)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_server = Arc::new(RwLock::new(GameServer::new()));

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
    println!("HTTP server running on {}", http_addr);
    let http_server = tokio::spawn(async move {
        axum::serve(
            tokio::net::TcpListener::bind(http_addr).await.unwrap(),
            app,
        )
        .await
        .unwrap();
    });

    // UDP Server for real-time communication
    let udp_addr = "0.0.0.0:8081";
    println!("UDP server running on {}", udp_addr);

    let udp_socket = Arc::new(UdpSocket::bind(udp_addr).await?);
    let udp_socket_clone = udp_socket.clone();

    let game_server_clone = game_server.clone();
    let udp_server = tokio::spawn(async move {
        let _ = udp_server_loop(udp_socket, game_server_clone).await;
    });

    // Dummy player movement task
    let dummy_mover_game_server = game_server.clone();
    let dummy_mover_socket = udp_socket_clone;
    let dummy_mover = tokio::spawn(async move {
        let _ = dummy_player_mover(dummy_mover_socket, dummy_mover_game_server).await;
    });

    // Wait for both servers
    tokio::try_join!(http_server, udp_server, dummy_mover)?;

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
                println!("Player {} joined lobby {}", pid, code);

                // Store client address for this player
                let mut server = game_server.write().await;
                server.set_player_address(code, pid as u32, addr);

                // Send welcome message
                let response = serde_json::json!({
                    "type": "welcome",
                    "message": "Connected to lobby"
                });
                let _ = socket.send_to(&serde_json::to_vec(&response).unwrap(), addr).await;

                // Notify other players in the lobby about the new player
                if let Some(lobby) = server.lobbies.get(code) {
                    if let Some(player) = lobby.players.get(&(pid as u32)) {
                        let player_joined_packet = serde_json::json!({
                            "type": "player_joined",
                            "player": {
                                "id": pid,
                                "name": player.name
                            }
                        });
                        let packet_data = serde_json::to_vec(&player_joined_packet).unwrap();
                        
                        // Send to all other clients in the lobby
                        for (client_id, client_addr) in &lobby.client_addresses {
                            if *client_id != pid as u32 {
                                if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
                                    println!("Failed to notify player {} about new player: {:?}", client_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Some("position_update") => {
            // Handle position updates
            let player_id = packet.get("player_id").and_then(|v| v.as_u64());
            let pos_data = packet.get("position");

            if let (Some(pid), Some(pos)) = (player_id, pos_data) {
                let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let z = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                // Update player position in server state and find lobby code
                let mut lobby_code_opt: Option<String> = None;
                {
                    let mut server = game_server.write().await;
                    for (code, lobby) in server.lobbies.iter_mut() {
                        if let Some(player) = lobby.players.get_mut(&(pid as u32)) {
                            player.position = (x, y, z);
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
        _ => {
            println!("Unknown packet type: {:?}", packet_type);
        }
    }
}

async fn dummy_player_mover(socket: Arc<UdpSocket>, game_server: Arc<RwLock<GameServer>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = interval(Duration::from_millis(100)); // Update every 100ms

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