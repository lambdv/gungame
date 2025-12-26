use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use crate::handlers::server::ServerState;


pub async fn handle_udp_packet(
    packet: serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let packet_type = packet.get("type").and_then(|v| v.as_str());

    match packet_type {
        Some("join") => {
            handle_join_packet(&packet, addr, socket, game_server).await;
        }
        Some("leave") => {
            handle_leave_packet(&packet, addr, socket, game_server).await;
        }
        Some("position_update") => {
            handle_position_update_packet(&packet, addr, socket, game_server).await;
        }
        Some("shoot") => {
            handle_shoot_packet(&packet, addr, socket, game_server).await;
        }
        Some("reload") => {
            handle_reload_packet(&packet, addr, socket, game_server).await;
        }
        Some("request_state") => {
            handle_request_state_packet(&packet, addr, socket, game_server).await;
        }
        Some("weapon_switch") => {
            handle_weapon_switch_packet(&packet, addr, socket, game_server).await;
        }
        Some("keepalive") => {
            handle_keepalive_packet(&packet, addr, socket, game_server).await;
        }
        _ => {
            println!("Unknown packet type: {:?}", packet_type);
        }
    }
}

pub fn handle_join_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    info!("UDP JOIN: Player {} attempting to join lobby {} from {:?}", player_id, lobby_code, addr);
}

pub fn handle_leave_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    info!("UDP LEAVE: Player {} leaving lobby {} from {:?}", player_id, lobby_code, addr);
}

pub fn handle_position_update_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let position = packet.get("position").and_then(|v| v.as_object());
    let rotation = packet.get("rotation").and_then(|v| v.as_object());
    info!("UDP POSITION UPDATE: Player {} updating position in lobby {} from {:?}", player_id, lobby_code, addr);
}

pub fn handle_shoot_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let target_id = packet.get("target_id").and_then(|v| v.as_u64());
    info!("UDP SHOOT: Player {} shooting at target {} in lobby {} from {:?}", player_id, target_id, lobby_code, addr);
}

pub fn handle_reload_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    info!("UDP RELOAD: Player {} reloading in lobby {} from {:?}", player_id, lobby_code, addr);
}

pub fn handle_request_state_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<GameServer>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    info!("UDP REQUEST STATE: Player {} requesting state in lobby {} from {:?}", player_id, lobby_code, addr);
}   

pub fn handle_weapon_switch_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let weapon_id = packet.get("weapon_id").and_then(|v| v.as_u64());
    info!("UDP WEAPON SWITCH: Player {} switching weapon {} in lobby {} from {:?}", player_id, weapon_id, lobby_code, addr);
}

pub fn handle_keepalive_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<RwLock<ServerState>>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    info!("UDP KEEPALIVE: Player {} sending keepalive in lobby {} from {:?}", player_id, lobby_code, addr);
}

// async fn handle_join_packet(
//     packet: &serde_json::Value,
//     addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid)) = (lobby_code, player_id) {
//         println!("UDP JOIN: Player {} attempting to join lobby {} from {:?}", pid, code, addr);

//         let mut server = game_server.write().await;

//         // Check if the player exists in the lobby first
//         let player_exists = server.lobbies.get(code)
//             .and_then(|lobby| lobby.players.get(&(pid as u32)))
//             .is_some();

//         println!("UDP JOIN: Processing join for player {} in lobby {}", pid, code);
//         println!("UDP JOIN: Player {} existence check - exists: {}", pid, player_exists);

//         if player_exists {
//             // Update client address (handles reconnection from different IP/port)
//             server.set_player_address(code, pid as u32, addr);

//             // Update player activity timestamp to prevent being marked inactive during scene loading
//             if let Some(lobby) = server.lobbies.get_mut(code) {
//                 if let Some(player) = lobby.players.get_mut(&(pid as u32)) {
//                     player.last_update = SystemTime::now();
//                 }
//             }

//             // Get player name for notifications (now that we can safely borrow again)
//             let player_name = server.lobbies.get(code)
//                 .and_then(|lobby| lobby.players.get(&(pid as u32)))
//                 .map(|p| p.name.clone())
//                 .unwrap_or_else(|| "Unknown".to_string());

//             // Send welcome message
//             let response = serde_json::json!({
//                 "type": "welcome",
//                 "message": "Connected to lobby"
//             });
//             println!("UDP JOIN: Sending welcome packet to player {}", pid);
//             let _ = socket.send_to(&serde_json::to_vec(&response).unwrap(), addr).await;

//             // Notify other players in the lobby about the new/reconnecting player
//             let player_joined_packet = serde_json::json!({
//                 "type": "player_joined",
//                 "player": {
//                     "id": pid,
//                     "name": player_name
//                 }
//             });
//             let packet_data = serde_json::to_vec(&player_joined_packet).unwrap();

//             // Get the current lobby state for notifications
//             if let Some(lobby) = server.lobbies.get(code) {
//                 println!("UDP JOIN: Notifying {} existing players about new player {}", lobby.client_addresses.len().saturating_sub(1), pid);
//                 // Send to all other clients in the lobby
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if *client_id != pid as u32 {
//                         println!("UDP JOIN: Notifying player {} about new player {}", client_id, pid);
//                         if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                             println!("Failed to notify player {} about player {} join: {:?}", client_id, pid, e);
//                         }
//                     }
//                 }
//             }

//             println!("Player {} successfully joined lobby {} via UDP", pid, code);
//         } else {
//             println!("Warning: Player {} not found in lobby {} during UDP join", pid, code);
//             let error_response = serde_json::json!({
//                 "type": "error",
//                 "message": "Player not found in lobby. Please rejoin via HTTP first."
//             });
//             let _ = socket.send_to(&serde_json::to_vec(&error_response).unwrap(), addr).await;
//         }
//     }
// }

// async fn handle_leave_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid)) = (lobby_code, player_id) {
//         println!("Player {} left lobby {}", pid, code);

//         let mut server = game_server.write().await;

//         // Remove player from lobby immediately
//         if let Some(lobby) = server.lobbies.get_mut(code) {
//             let player_was_present = lobby.players.contains_key(&(pid as u32));

//             if player_was_present {
//                 lobby.players.remove(&(pid as u32));
//                 lobby.client_addresses.remove(&(pid as u32));

//                 // Notify other players in the lobby about the leaving player
//                 let player_left_packet = serde_json::json!({
//                     "type": "player_left",
//                     "player_id": pid
//                 });
//                 let packet_data = serde_json::to_vec(&player_left_packet).unwrap();

//                 // Send to all remaining clients in the lobby
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                         println!("Failed to notify player {} about leaving player: {:?}", client_id, e);
//                     }
//                 }

//                 println!("Successfully removed player {} from lobby {}", pid, code);
//             } else {
//                 println!("Warning: Player {} was not found in lobby {} during leave", pid, code);
//             }
//         } else {
//             println!("Warning: Lobby {} not found during leave", code);
//         }
//     }
// }

// async fn handle_position_update_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     // Handle position and rotation updates
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());
//     let pos_data = packet.get("position");
//     let rot_data = packet.get("rotation");

//     if let (Some(pid), Some(pos)) = (player_id, pos_data) {
//         let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
//         let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
//         let z = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

//         // Extract rotation data
//         let (rx, ry, rz) = if let Some(rot) = rot_data {
//             let rx = rot.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
//             let ry = rot.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
//             let rz = rot.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
//             (rx, ry, rz)
//         } else {
//             (0.0, 0.0, 0.0)
//         };

//         // Update player position and rotation in server state and find lobby code
//         let mut lobby_code_opt: Option<String> = None;
//         {
//             let mut server = game_server.write().await;
//             for (code, lobby) in server.lobbies.iter_mut() {
//                 if let Some(player) = lobby.players.get_mut(&(pid as u32)) {
//                     player.position = (x, y, z);
//                     player.rotation = (rx, ry, rz);
//                     player.last_update = SystemTime::now();
//                     lobby_code_opt = Some(code.clone());
//                     break;
//                 }
//             }
//         }

//         // Broadcast to all other players in same lobby
//         if let Some(lobby_code) = lobby_code_opt {
//             let server = game_server.read().await;
//             if let Some(lobby) = server.lobbies.get(&lobby_code) {
//                 let broadcast_packet = serde_json::json!({
//                     "type": "position_update",
//                     "player_id": pid,
//                     "position": {
//                         "x": x,
//                         "y": y,
//                         "z": z
//                     },
//                     "rotation": {
//                         "x": rx,
//                         "y": ry,
//                         "z": rz
//                     }
//                 });

//                 let packet_data = serde_json::to_vec(&broadcast_packet).unwrap();

//                 // Send to all clients in the lobby except the sender
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if *client_id != pid as u32 {
//                         if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                             println!("Failed to send position update to player {}: {:?}", client_id, e);
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

// async fn handle_shoot_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());
//     let target_id = packet.get("target_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid), Some(tid)) = (lobby_code, player_id, target_id) {
//         let mut server = game_server.write().await;

//         // Get weapon info before doing mutable operations
//         let weapon_damage = if let Some(lobby) = server.lobbies.get(code) {
//             if let Some(player) = lobby.players.get(&(pid as u32)) {
//                 server.weapons.get(&player.current_weapon_id)
//                     .map(|w| w.damage)
//                     .unwrap_or(0)
//             } else {
//                 0
//             }
//         } else {
//             0
//         };

//         // Try to shoot (server validates ammo/fire rate)
//         if let Ok(can_shoot) = server.player_shoot(code, pid as u32) {
//             if can_shoot && weapon_damage > 0 {
//                 println!("Player {} shot at target {}", pid, tid);

//                 // Apply damage to target
//                 let _ = server.player_take_damage(code, tid as u32, weapon_damage);

//                 // Broadcast shot event
//                 if let Some(lobby) = server.lobbies.get(code) {
//                     let shot_packet = serde_json::json!({
//                         "type": "player_shot",
//                         "player_id": pid,
//                         "target_id": tid
//                     });
//                     let packet_data = serde_json::to_vec(&shot_packet).unwrap();

//                     // Send to all clients in the lobby
//                     for (client_id, client_addr) in &lobby.client_addresses {
//                         if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                             println!("Failed to send shot event to player {}: {:?}", client_id, e);
//                         }
//                     }

//                     // Send damage notification to target
//                     if let Some(target_addr) = lobby.client_addresses.get(&(tid as u32)) {
//                         let damage_packet = serde_json::json!({
//                             "type": "player_damaged",
//                             "damage": weapon_damage,
//                             "attacker_id": pid
//                         });
//                         let _ = socket.send_to(&serde_json::to_vec(&damage_packet).unwrap(), target_addr).await;
//                     }
//                 }
//             }
//         }
//     }
// }

// async fn handle_reload_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid)) = (lobby_code, player_id) {
//         let mut server = game_server.write().await;

//         if let Ok(()) = server.player_start_reload(code, pid as u32) {
//             println!("Player {} started reload", pid);

//             // Broadcast reload started
//             if let Some(lobby) = server.lobbies.get(code) {
//                 let reload_packet = serde_json::json!({
//                     "type": "reload_started",
//                     "player_id": pid
//                 });
//                 let packet_data = serde_json::to_vec(&reload_packet).unwrap();

//                 // Send to all clients in the lobby
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                         println!("Failed to send reload event to player {}: {:?}", client_id, e);
//                     }
//                 }
//             }
//         }
//     }
// }

// async fn handle_request_state_packet(
//     packet: &serde_json::Value,
//     addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid)) = (lobby_code, player_id) {
//         let server = game_server.read().await;

//         if let Ok(player) = server.get_player_state(code, pid as u32) {
//             let state_packet = serde_json::json!({
//                 "type": "player_state_update",
//                 "player_id": pid,
//                 "health": player.current_health,
//                 "max_health": player.max_health,
//                 "ammo": player.current_ammo,
//                 "max_ammo": player.max_ammo,
//                 "is_reloading": player.is_reloading,
//                 "weapon_id": player.current_weapon_id
//             });

//             let _ = socket.send_to(&serde_json::to_vec(&state_packet).unwrap(), addr).await;
//         }
//     }
// }

// async fn handle_weapon_switch_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());
//     let weapon_id = packet.get("weapon_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid), Some(wid)) = (lobby_code, player_id, weapon_id) {
//         let mut server = game_server.write().await;

//         if let Ok(()) = server.player_switch_weapon(code, pid as u32, wid as u32) {
//             println!("Player {} switched to weapon {}", pid, wid);

//             // Broadcast weapon switch to all clients in lobby
//             if let Some(lobby) = server.lobbies.get(code) {
//                 let weapon_switch_packet = serde_json::json!({
//                     "type": "weapon_switched",
//                     "player_id": pid,
//                     "weapon_id": wid
//                 });
//                 let packet_data = serde_json::to_vec(&weapon_switch_packet).unwrap();

//                 // Send to all clients in the lobby
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if let Err(e) = socket.send_to(&packet_data, *client_addr).await {
//                         println!("Failed to send weapon switch to player {}: {:?}", client_id, e);
//                     }
//                 }
//             }
//         }
//     }
// }

// async fn handle_keepalive_packet(
//     packet: &serde_json::Value,
//     _addr: std::net::SocketAddr,
//     socket: &UdpSocket,
//     game_server: &Arc<RwLock<GameServer>>,
// ) {
//     // Handle keepalive heartbeat to prevent player from being marked inactive
//     let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
//     let player_id = packet.get("player_id").and_then(|v| v.as_u64());

//     if let (Some(code), Some(pid)) = (lobby_code, player_id) {
//         let mut server = game_server.write().await;
//         if let Some(lobby) = server.lobbies.get_mut(code) {
//             if let Some(player) = lobby.players.get_mut(&(pid as u32)) {
//                 player.last_update = SystemTime::now();
//             }
//         }
//     }
// }
