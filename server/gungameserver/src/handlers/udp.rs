use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use log::info;
use crate::state::server_state::ServerState;


pub async fn handle_udp_packet(
    packet: serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let packet_type = packet.get("type").and_then(|v| v.as_str());

    match packet_type {
        Some("join") => {
            handle_join_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("leave") => {
            handle_leave_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("position_update") => {
            handle_position_update_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("shoot") => {
            handle_shoot_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("reload") => {
            handle_reload_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("request_state") => {
            handle_request_state_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("weapon_switch") => {
            handle_weapon_switch_packet(&packet, _addr, _socket, game_server).await;
        }
        Some("keepalive") => {
            handle_keepalive_packet(&packet, _addr, _socket, game_server).await;
        }
        _ => {
            println!("Unknown packet type: {:?}", packet_type);
        }
    }
}

pub async fn handle_join_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let player_name = packet.get("player_name").and_then(|v| v.as_str()).unwrap_or("Unknown");

    info!("UDP JOIN: Player {:?} ({}) attempting to join lobby {:?} from {:?}", player_id, player_name, lobby_code, _addr);

    if let (Some(code), Some(pid)) = (lobby_code, player_id) {
        let pid = pid as u32;

        if let Some(lobby_handle) = game_server.get_lobby_handle(code) {
            let mut lobby = lobby_handle.write().await;

            if lobby.players.contains_key(&pid) {
                lobby.client_addresses.insert(pid, _addr);

                let player_name = lobby.players.get(&pid)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                let response = serde_json::json!({
                    "type": "welcome",
                    "message": "Connected to lobby",
                    "player_id": pid,
                    "lobby_code": code
                });

                if let Ok(data) = serde_json::to_vec(&response) {
                    let _ = _socket.send_to(&data, _addr).await;
                }

                let player_joined_packet = serde_json::json!({
                    "type": "player_joined",
                    "player": {
                        "id": pid,
                        "name": player_name
                    }
                });

                    if let Ok(packet_data) = serde_json::to_vec(&player_joined_packet) {
                        for (_client_id, client_addr) in &lobby.client_addresses {
                        if *_client_id != pid {
                            let _ = _socket.send_to(&packet_data, *client_addr).await;
                        }
                    }
                }

                info!("Player {} ({}) successfully joined lobby {}", pid, player_name, code);
            } else {
                let error_response = serde_json::json!({
                    "type": "error",
                    "message": "Player not found in lobby. Please rejoin via HTTP first."
                });

                if let Ok(data) = serde_json::to_vec(&error_response) {
                    let _ = _socket.send_to(&data, _addr).await;
                }
                info!("Warning: Player {} not found in lobby {} during UDP join", pid, code);
            }
        } else {
            let error_response = serde_json::json!({
                "type": "error",
                "message": "Lobby not found"
            });

            if let Ok(data) = serde_json::to_vec(&error_response) {
                let _ = _socket.send_to(&data, _addr).await;
            }
            info!("Warning: Lobby {} not found during UDP join", code);
        }
    }
}

pub async fn handle_leave_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    info!("UDP LEAVE: Player {:?} leaving from {:?}", player_id, _addr);

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(code) = game_server.find_lobby_by_player(pid).await {
            if let Some(lobby_handle) = game_server.get_lobby_handle(&code) {
                let mut lobby = lobby_handle.write().await;

                if lobby.players.contains_key(&pid) {
                    let player_name = lobby.players.get(&pid)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    lobby.players.remove(&pid);
                    lobby.client_addresses.remove(&pid);

                    let player_left_packet = serde_json::json!({
                        "type": "player_left",
                        "player_id": pid
                    });

                    if let Ok(packet_data) = serde_json::to_vec(&player_left_packet) {
                        for (_client_id, client_addr) in &lobby.client_addresses {
                            let _ = _socket.send_to(&packet_data, *client_addr).await;
                        }
                    }

                    info!("Player {} ({}) left lobby {}", pid, player_name, code);
                }
            }
        }
    }
}

pub async fn handle_position_update_packet(
    packet: &serde_json::Value,
    __addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let pos_data = packet.get("position");
    let rot_data = packet.get("rotation");

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
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let mut lobby = lobby_handle.write().await;

                if let Some(player) = lobby.players.get_mut(&pid) {
                    player.position = (x, y, z);
                    player.rotation = (rx, ry, rz);
                    player.last_update = SystemTime::now();

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

                    if let Ok(packet_data) = serde_json::to_vec(&broadcast_packet) {
                        for (_client_id, client_addr) in &lobby.client_addresses {
                            if *_client_id != pid {
                                let _ = _socket.send_to(&packet_data, *client_addr).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn handle_shoot_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());
    let target_id = packet.get("target_id").and_then(|v| v.as_u64());

    info!("UDP SHOOT: Player {:?} shooting at target {:?}", player_id, target_id);

    if let (Some(pid), Some(tid)) = (player_id, target_id) {
        let pid = pid as u32;
        let tid = tid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let lobby = lobby_handle.read().await;

                let shot_packet = serde_json::json!({
                    "type": "player_shot",
                    "player_id": pid,
                    "target_id": tid
                });

                if let Ok(packet_data) = serde_json::to_vec(&shot_packet) {
                    for (_client_id, client_addr) in &lobby.client_addresses {
                        let _ = _socket.send_to(&packet_data, *client_addr).await;
                    }
                }

                if let Some(target_addr) = lobby.client_addresses.get(&tid) {
                    let damage_packet = serde_json::json!({
                        "type": "player_damaged",
                        "damage": 10,
                        "attacker_id": pid
                    });

                    if let Ok(data) = serde_json::to_vec(&damage_packet) {
                        let _ = _socket.send_to(&data, *target_addr).await;
                    }
                }
            }
        }
    }
}

pub async fn handle_reload_packet(
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
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let lobby = lobby_handle.read().await;

                let reload_packet = serde_json::json!({
                    "type": "reload_started",
                    "player_id": pid
                });

                if let Ok(packet_data) = serde_json::to_vec(&reload_packet) {
                    for (_client_id, client_addr) in &lobby.client_addresses {
                        let _ = _socket.send_to(&packet_data, *client_addr).await;
                    }
                }
            }
        }
    }
}

pub async fn handle_request_state_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
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

                    if let Ok(data) = serde_json::to_vec(&state_packet) {
                        let _ = _socket.send_to(&data, _addr).await;
                    }
                }
            }
        }
    }
}

pub async fn handle_weapon_switch_packet(
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
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let lobby = lobby_handle.read().await;

                let weapon_switch_packet = serde_json::json!({
                    "type": "weapon_switched",
                    "player_id": pid,
                    "weapon_id": wid
                });

                if let Ok(packet_data) = serde_json::to_vec(&weapon_switch_packet) {
                    for (_client_id, client_addr) in &lobby.client_addresses {
                        let _ = _socket.send_to(&packet_data, *client_addr).await;
                    }
                }
            }
        }
    }
}

pub async fn handle_keepalive_packet(
    packet: &serde_json::Value,
    _addr: std::net::SocketAddr,
    _socket: &UdpSocket,
    game_server: &Arc<ServerState>,
) {
    let player_id = packet.get("player_id").and_then(|v| v.as_u64());

    if let Some(pid) = player_id {
        let pid = pid as u32;

        if let Some(lobby_code) = game_server.find_lobby_by_player(pid).await {
            if let Some(lobby_handle) = game_server.get_lobby_handle(&lobby_code) {
                let mut lobby = lobby_handle.write().await;

                if let Some(player) = lobby.players.get_mut(&pid) {
                    player.last_update = SystemTime::now();
                }
            }
        }
    }
}
