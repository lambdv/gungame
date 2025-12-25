use serde_json::Value;
use std::net::SocketAddr;
use crate::state::server_state::ServerState;
use crate::state::commands::LobbyCommand;
use std::sync::Arc;

/// Ultra-thin UDP packet handler - no locks in hot path
/// Parses packet and enqueues command to lobby's command queue
pub async fn handle_udp_packet(
    packet: Value,
    addr: SocketAddr,
    state: &Arc<ServerState>,
) {
    let lobby_code = packet.get("lobby_code").and_then(|v| v.as_str());

    // Get command sender for lobby (read-only DashMap lookup, no lock)
    let Some(tx) = lobby_code.and_then(|code| state.get_lobby_tx(code)) else {
        log::debug!("UDP packet for unknown lobby: {:?}", lobby_code);
        return;
    };

    // Parse command from packet
    let cmd = parse_command(&packet, addr);

    // Non-blocking send - drop if queue is full (prevents backpressure)
    if let Err(_) = tx.try_send(cmd) {
        log::debug!("Command queue full for lobby {}, dropping packet", lobby_code.unwrap_or("unknown"));
    }
}

/// Parse UDP packet into LobbyCommand
fn parse_command(packet: &Value, addr: SocketAddr) -> LobbyCommand {
    let player_id = packet.get("player_id")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    match packet.get("type").and_then(|v| v.as_str()) {
        Some("join") => {
            let player_id = player_id.unwrap_or(0);
            let name = packet.get("player_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            LobbyCommand::PlayerJoin { player_id, name, addr }
        }
        Some("leave") => {
            LobbyCommand::PlayerLeave { 
                player_id: player_id.unwrap_or(0) 
            }
        }
        Some("position_update") => {
            let pos = packet.get("position").and_then(|v| v.as_object());
            let rot = packet.get("rotation").and_then(|v| v.as_object());
            
            let position = if let Some(pos) = pos {
                (
                    pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                )
            } else {
                (0.0, 0.0, 0.0)
            };
            
            let rotation = if let Some(rot) = rot {
                (
                    rot.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    rot.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    rot.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                )
            } else {
                (0.0, 0.0, 0.0)
            };
            
            LobbyCommand::PositionUpdate {
                player_id: player_id.unwrap_or(0),
                position,
                rotation,
                addr,
            }
        }
        Some("shoot") => {
            let target_id = packet.get("target_id")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0);
            
            LobbyCommand::Shoot {
                player_id: player_id.unwrap_or(0),
                target_id,
            }
        }
        Some("reload") => {
            LobbyCommand::Reload {
                player_id: player_id.unwrap_or(0),
            }
        }
        Some("weapon_switch") => {
            let weapon_id = packet.get("weapon_id")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or(0);
            
            LobbyCommand::WeaponSwitch {
                player_id: player_id.unwrap_or(0),
                weapon_id,
            }
        }
        Some("keepalive") | Some("heartbeat") => {
            LobbyCommand::Heartbeat {
                player_id: player_id.unwrap_or(0),
                addr,
            }
        }
        _ => {
            log::debug!("Unknown packet type: {:?}", packet.get("type"));
            // Return heartbeat as fallback to update timestamp
            LobbyCommand::Heartbeat {
                player_id: player_id.unwrap_or(0),
                addr,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_parse_position_command() {
        let packet = serde_json::json!({
            "type": "position_update",
            "player_id": 1,
            "lobby_code": "TEST",
            "position": { "x": 10.0, "y": 2.0, "z": 5.0 },
            "rotation": { "x": 0.0, "y": 1.0, "z": 0.0 }
        });
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let cmd = parse_command(&packet, addr);
        
        if let LobbyCommand::PositionUpdate { player_id, position, addr: cmd_addr, .. } = cmd {
            assert_eq!(player_id, 1);
            assert_eq!(position.0, 10.0);
            assert_eq!(position.1, 2.0);
            assert_eq!(position.2, 5.0);
            assert_eq!(cmd_addr, addr);
        } else {
            panic!("Expected PositionUpdate command");
        }
    }

    #[tokio::test]
    async fn test_parse_shoot_command() {
        let packet = serde_json::json!({
            "type": "shoot",
            "player_id": 1,
            "target_id": 2,
            "lobby_code": "TEST"
        });
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let cmd = parse_command(&packet, addr);
        
        if let LobbyCommand::Shoot { player_id, target_id } = cmd {
            assert_eq!(player_id, 1);
            assert_eq!(target_id, 2);
        } else {
            panic!("Expected Shoot command");
        }
    }
}
