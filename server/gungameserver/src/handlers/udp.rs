use std::sync::Arc;
use tokio::net::UdpSocket;
use log::{info, warn, debug};
use crate::state::server_state::ServerState;
use crate::state::commands::LobbyCommand;
use crate::utils::weapondb::WeaponDb;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_PACKET_SIZE: usize = 1024;
const RATE_LIMIT_WINDOW_MS: u64 = 1000;
const MAX_PACKETS_PER_WINDOW: u64 = 100;

struct RateLimiter {
    packet_counts: HashMap<std::net::SocketAddr, AtomicU64>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            packet_counts: HashMap::new(),
        }
    }

    fn check_rate_limit(&mut self, addr: &std::net::SocketAddr) -> bool {
        let count = self.packet_counts
            .entry(addr.clone())
            .or_insert_with(|| AtomicU64::new(0));
        
        let current = count.fetch_add(1, Ordering::Relaxed);
        current < MAX_PACKETS_PER_WINDOW
    }

    fn cleanup(&mut self) {
    }
}

async fn send_packet(socket: &UdpSocket, addr: &std::net::SocketAddr, packet: &serde_json::Value) {
    if let Ok(data) = serde_json::to_vec(packet) {
        if let Err(e) = socket.send_to(&data, addr).await {
            debug!("Failed to send packet to {}: {}", addr, e);
        }
    }
}

async fn broadcast_packet(socket: &UdpSocket, addresses: &[(u32, std::net::SocketAddr)], exclude_player: u32, packet: &serde_json::Value) {
    if let Ok(data) = serde_json::to_vec(packet) {
        for (player_id, addr) in addresses {
            if *player_id != exclude_player {
                if let Err(e) = socket.send_to(&data, addr).await {
                    debug!("Failed to broadcast to {}: {}", addr, e);
                }
            }
        }
    }
}

pub async fn handle_udp_packet(
    packet: serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<ServerState>,
    weapons: &Arc<WeaponDb>,
) {
    let packet_type = packet.get("type").and_then(|v| v.as_str());
    
    debug!("UDP packet from {}: type={}", addr, packet_type.unwrap_or("unknown"));

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
            handle_shoot_packet(&packet, addr, socket, game_server, weapons).await;
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
            debug!("Unknown packet type: {:?}", packet_type);
        }
    }
}

async fn handle_join_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let player_name = packet.get("player_name").and_then(|v| v.as_str()).unwrap_or("Unknown");

    info!("UDP JOIN: Player {:?} ({}) attempting to join lobby {:?} from {:?}", player_id, player_name, lobby_code, addr);

    if let (Some(code), Some(pid)) = (lobby_code, player_id) {
        let pid = pid as u32;

        if let Some(command_tx) = game_server.get_lobby_tx(code) {
            let cmd = LobbyCommand::UdpConnect {
                player_id: pid,
                name: player_name.to_string(),
                addr,
            };

            if let Err(e) = command_tx.send(cmd).await {
                warn!("Failed to send UDP connect command: {}", e);
            }

            let response = serde_json::json!({
                "type": "welcome",
                "message": "Connected to lobby",
                "player_id": pid,
                "lobby_code": code
            });

            send_packet(socket, &addr, &response).await;
            info!("Player {} ({}) successfully joined lobby {}", pid, player_name, code);
        } else {
            let error_response = serde_json::json!({
                "type": "error",
                "message": "Lobby not found"
            });
            send_packet(socket, &addr, &error_response).await;
            warn!("Lobby {} not found during UDP join", code);
        }
    }
}

async fn handle_leave_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    info!("UDP LEAVE: Player {:?} leaving from {:?}", player_id, _addr);

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(command_tx) = game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::PlayerLeave { player_id: pid };
                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send player leave command: {}", e);
                }
            }
        }
    }
}

async fn handle_position_update_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let pos_data = packet.get("position");
    let rot_data = packet.get("rotation");

    // debug!("Received position update from {}: {:?}", addr, packet);

    if let (Some(pid), Some(pos)) = (player_id, pos_data) {
        let pid = pid as u32;

        let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let z = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

        let (rx, ry, rz) = if let Some(rot) = rot_data {
            let rx = rot.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let ry = rot.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let rz = rot.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            (rx, ry, rz)
        } else {
            (0.0, 0.0, 0.0)
        };

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            // debug!("Found lobby {} for player {}, sending position update", lobby_code, pid);
            if let Some(command_tx) = game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::PositionUpdate {
                    player_id: pid,
                    position: (x, y, z),
                    rotation: (rx, ry, rz),
                    addr,
                };

                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send position update: {}", e);
                } else {
                    debug!("Position update command sent for player {}", pid);
                }
            }
        } else {
            warn!("No lobby found for player {}", pid);
        }
    }
}

async fn handle_shoot_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    _game_server: &Arc<ServerState>,
    _weapons: &Arc<WeaponDb>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let target_id = packet.get("target_id").and_then(|v| v.as_u64());

    info!("UDP SHOOT: Player {:?} shooting at target {:?}", player_id, target_id);

    if let (Some(pid), Some(tid)) = (player_id, target_id) {
        let pid = pid as u32;
        let tid = tid as u32;

        if let Some(lobby_code) = _game_server.find_lobby_by_player(pid).await {
            if let Some(command_tx) = _game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::Shoot {
                    player_id: pid,
                    target_id: tid,
                };
                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send shoot command: {}", e);
                }
            }
        }
    }
}

async fn handle_reload_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    info!("UDP RELOAD: Player {:?} reloading", player_id);

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(command_tx) = game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::Reload { player_id: pid };
                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send reload command: {}", e);
                }
            }
        }
    }
}

async fn handle_request_state_packet(
    packet: &serde_json::Value,
    addr: std::net::SocketAddr,
    socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    info!("UDP REQUEST STATE: Player {:?} requesting state", player_id);

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let lobby = lobby_handle.read().await;

                if let Some(player) = lobby.players.get(&pid) {
                    let state_packet = serde_json::json!({
                        "type": "player_state_update",
                        "player_id": pid,
                        "health": player.current_health,
                        "max_health": player.max_health,
                        "ammo": player.current_ammo,
                        "max_ammo": player.max_ammo,
                        "is_reloading": player.is_reloading,
                        "weapon_id": player.current_weapon_id,
                        "lobby_code": lobby_code,
                        "lobby_players": lobby.players.len()
                    });

                    send_packet(socket, &addr, &state_packet).await;
                }
            }
        }
    }
}

async fn handle_weapon_switch_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let weapon_id = packet.get("weapon_id").and_then(|v| v.as_u64());

    info!("UDP WEAPON SWITCH: Player {:?} switching to weapon {:?}", player_id, weapon_id);

    if let (Some(pid), Some(wid)) = (player_id, weapon_id) {
        let pid = pid as u32;
        let wid = wid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(command_tx) = game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::WeaponSwitch {
                    player_id: pid,
                    weapon_id: wid,
                };
                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send weapon switch command: {}", e);
                }
            }
        }
    }
}

async fn handle_keepalive_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(command_tx) = game_server.get_lobby_tx(&lobby_code) {
                let cmd = LobbyCommand::Heartbeat {
                    player_id: pid,
                    addr: _addr,
                };
                if let Err(e) = command_tx.send(cmd).await {
                    warn!("Failed to send heartbeat: {}", e);
                }
            }
        }
    }
}
