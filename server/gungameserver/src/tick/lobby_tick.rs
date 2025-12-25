use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration};
use crate::state::lobby::Lobby;
use crate::state::commands::{LobbyCommand, drain_and_coalesce};
use crate::domain::lobbies;
use crate::domain::logic;
use crate::tick::delta_sync;
use crate::utils::weapondb::WeaponDb;
use crate::utils::config::Config;
use crate::utils::buffers::{SyncEvent, PacketBuffer};
use serde_json::json;

/// Per-lobby tick loop - processes commands and broadcasts updates
/// Runs at fixed tick rate (50Hz by default)
pub async fn lobby_tick_loop(
    lobby: Arc<RwLock<Lobby>>,
    mut command_rx: mpsc::Receiver<LobbyCommand>,
    socket: Arc<UdpSocket>,
    weapons: Arc<WeaponDb>,
    config: Arc<Config>,
) {
    let tick_interval = Duration::from_millis(config.tick_interval_ms());
    let mut tick_timer = interval(tick_interval);
    let mut send_buffer = PacketBuffer::default();
    
    loop {
        tick_timer.tick().await;
        
        // 1. Drain commands (coalesce positions - keep only latest)
        let commands = drain_and_coalesce(&mut command_rx);
        
        // 2. Acquire lock ONCE per tick
        let mut lobby_guard = lobby.write().await;
        
        // Track players that joined/left this tick
        let mut players_joined: Vec<(u32, String)> = Vec::new();
        let mut players_left: Vec<u32> = Vec::new();
        let mut position_updates: Vec<u32> = Vec::new();
        
        // 3. Process all commands
        for cmd in commands {
            // Extract info before processing (to avoid borrow issues)
            let join_info = if let LobbyCommand::PlayerJoin { player_id, ref name, addr } = &cmd {
                Some((*player_id, name.clone(), *addr))
            } else {
                None
            };
            
            let leave_id = if let LobbyCommand::PlayerLeave { player_id } = &cmd {
                Some(*player_id)
            } else {
                None
            };
            
            let position_id = if let LobbyCommand::PositionUpdate { player_id, .. } = &cmd {
                Some(*player_id)
            } else {
                None
            };
            
            let _heartbeat_id = if let LobbyCommand::Heartbeat { player_id, .. } = &cmd {
                Some(*player_id)
            } else {
                None
            };
            
            // Process the command
            process_command(&mut lobby_guard, &weapons, cmd);
            
            // Handle special cases that need broadcasting
            if let Some((player_id, name, addr)) = join_info {
                players_joined.push((player_id, name));
                // Send welcome message to joining player with current lobby state
                send_welcome_message(&lobby_guard, &socket, player_id, addr).await;
            }
            
            if let Some(player_id) = leave_id {
                players_left.push(player_id);
            }
            
            if let Some(player_id) = position_id {
                position_updates.push(player_id);
            }
        }
        
        // 4. Update reload timers
        logic::update_reload_states(&mut lobby_guard);
        
        // 5. Cleanup inactive players periodically (every 5 seconds worth of ticks)
        // Use a local counter that persists across ticks via closure
        // For MVP, we'll do cleanup every tick (can be optimized later)
        let _removed = lobbies::cleanup_inactive(
            &mut lobby_guard,
            config.player_inactivity_timeout_secs,
        );
        
        // 6. Broadcast player join/leave events
        if !players_joined.is_empty() {
            broadcast_player_join_events(&lobby_guard, &socket, &players_joined).await;
        }
        if !players_left.is_empty() {
            broadcast_player_leave_events(&lobby_guard, &socket, &players_left).await;
        }
        
        // 7. Broadcast position updates (every tick for players that moved)
        if !position_updates.is_empty() {
            broadcast_position_updates(&lobby_guard, &socket, &position_updates).await;
        }
        
        // 8. Delta sync - only send changes (health, ammo, weapon, reload)
        let state_events = delta_sync::collect_dirty_events(&mut lobby_guard);
        
        // 9. Broadcast state events (reuse buffer)
        if !state_events.is_empty() {
            broadcast_state_events(&lobby_guard, &socket, &state_events, &mut send_buffer).await;
        }
        
        // 10. Clear dirty flags
        lobby_guard.clear_dirty();
    }
}

/// Process a single command
fn process_command(
    lobby: &mut Lobby,
    weapons: &WeaponDb,
    cmd: LobbyCommand,
) {
    match cmd {
        LobbyCommand::PlayerJoin { player_id, name, addr } => {
            let default_weapon = WeaponDb::default_weapon_id();
            if let Err(e) = lobbies::add_player(lobby, player_id, name, default_weapon, weapons) {
                log::warn!("Failed to add player {}: {}", player_id, e);
                return;
            }
            if let Err(e) = lobbies::set_player_address(lobby, player_id, addr) {
                log::warn!("Failed to set address for player {}: {}", player_id, e);
            }
        }
        LobbyCommand::PlayerLeave { player_id } => {
            lobbies::remove_player(lobby, player_id);
        }
        LobbyCommand::PositionUpdate { player_id, position, rotation, addr } => {
            // Update client address (ensures HTTP-joined players get their UDP address tracked)
            if lobby.players.contains_key(&player_id) {
                lobby.client_addresses.insert(player_id, addr);
            }
            if let Err(e) = lobbies::update_position(lobby, player_id, position, rotation) {
                log::debug!("Position update failed for player {}: {}", player_id, e);
            }
        }
        LobbyCommand::Shoot { player_id, target_id } => {
            match logic::try_shoot(lobby, weapons, player_id) {
                Ok(can_shoot) => {
                    if can_shoot {
                        // Get weapon damage
                        if let Some(player) = lobby.players.get(&player_id) {
                            if let Some(weapon) = weapons.get(player.current_weapon_id) {
                                let _ = logic::apply_damage(lobby, target_id, weapon.damage);
                            }
                        }
                    }
                }
                Err(e) => log::debug!("Shoot failed for player {}: {}", player_id, e),
            }
        }
        LobbyCommand::Reload { player_id } => {
            if let Err(e) = logic::start_reload(lobby, weapons, player_id) {
                log::debug!("Reload failed for player {}: {}", player_id, e);
            }
        }
        LobbyCommand::WeaponSwitch { player_id, weapon_id } => {
            if let Err(e) = logic::switch_weapon(lobby, weapons, player_id, weapon_id) {
                log::debug!("Weapon switch failed for player {}: {}", player_id, e);
            }
        }
        LobbyCommand::Heartbeat { player_id, addr } => {
            // Update client address (ensures HTTP-joined players get their UDP address tracked)
            if lobby.players.contains_key(&player_id) {
                lobby.client_addresses.insert(player_id, addr);
            }
            // Update last_update timestamp
            if let Some(player) = lobby.players.get_mut(&player_id) {
                player.last_update = std::time::SystemTime::now();
            }
        }
    }
}

/// Send welcome message to joining player with current lobby state
async fn send_welcome_message(
    lobby: &Lobby,
    socket: &UdpSocket,
    player_id: u32,
    addr: std::net::SocketAddr,
) {
    // Send welcome message
    let welcome_packet = json!({
        "type": "welcome",
        "message": "Connected to lobby",
        "player_id": player_id
    });

    if let Ok(data) = serde_json::to_vec(&welcome_packet) {
        let _ = socket.send_to(&data, addr).await;
    }

    // Send current player list to joining player
    let mut player_list = Vec::new();
    for player in lobby.players.values() {
        if player.id != player_id {
            player_list.push(json!({
                "id": player.id,
                "name": player.name,
                "position": {
                    "x": player.position.0,
                    "y": player.position.1,
                    "z": player.position.2
                },
                "rotation": {
                    "x": player.rotation.0,
                    "y": player.rotation.1,
                    "z": player.rotation.2
                }
            }));
        }
    }

    let players_packet = json!({
        "type": "player_list",
        "players": player_list
    });

    if let Ok(data) = serde_json::to_vec(&players_packet) {
        let _ = socket.send_to(&data, addr).await;
    }
}

/// Broadcast player join events to all clients
async fn broadcast_player_join_events(
    lobby: &Lobby,
    socket: &UdpSocket,
    players: &[(u32, String)],
) {
    for (player_id, name) in players {
        let packet = json!({
            "type": "player_joined",
            "player": {
                "id": player_id,
                "name": name
            }
        });

        if let Ok(data) = serde_json::to_vec(&packet) {
            // Send to all clients except the joining player
            for (client_id, addr) in &lobby.client_addresses {
                if *client_id != *player_id {
                    if let Err(e) = socket.send_to(&data, *addr).await {
                        log::debug!("Failed to send join event to {}: {:?}", addr, e);
                    }
                }
            }
        }
    }
}

/// Broadcast player leave events to all clients
async fn broadcast_player_leave_events(
    lobby: &Lobby,
    socket: &UdpSocket,
    player_ids: &[u32],
) {
    for player_id in player_ids {
        let packet = json!({
            "type": "player_left",
            "player_id": player_id
        });

        if let Ok(data) = serde_json::to_vec(&packet) {
            // Send to all remaining clients
            for (_client_id, addr) in &lobby.client_addresses {
                if let Err(e) = socket.send_to(&data, *addr).await {
                    log::debug!("Failed to send leave event to {}: {:?}", addr, e);
                }
            }
        }
    }
}

/// Broadcast position updates for players that moved
async fn broadcast_position_updates(
    lobby: &Lobby,
    socket: &UdpSocket,
    player_ids: &[u32],
) {
    for player_id in player_ids {
        if let Some(player) = lobby.players.get(player_id) {
            let packet = json!({
                "type": "position_update",
                "player_id": player_id,
                "position": {
                    "x": player.position.0,
                    "y": player.position.1,
                    "z": player.position.2
                },
                "rotation": {
                    "x": player.rotation.0,
                    "y": player.rotation.1,
                    "z": player.rotation.2
                }
            });

            if let Ok(data) = serde_json::to_vec(&packet) {
                // Send to all clients except the moving player
                for (client_id, addr) in &lobby.client_addresses {
                    if *client_id != *player_id {
                        if let Err(e) = socket.send_to(&data, *addr).await {
                            log::debug!("Failed to send position update to {}: {:?}", addr, e);
                        }
                    }
                }
            }
        }
    }
}

/// Broadcast state events to all clients in lobby
async fn broadcast_state_events(
    lobby: &Lobby,
    socket: &UdpSocket,
    events: &[SyncEvent],
    buffer: &mut PacketBuffer,
) {
    for event in events {
        let packet = match event {
            SyncEvent::HealthChanged { player_id, health } => {
                json!({
                    "type": "player_state_update",
                    "player_id": player_id,
                    "health": health
                })
            }
            SyncEvent::AmmoChanged { player_id, ammo } => {
                json!({
                    "type": "player_state_update",
                    "player_id": player_id,
                    "ammo": ammo
                })
            }
            SyncEvent::MaxAmmoChanged { player_id, max_ammo } => {
                json!({
                    "type": "player_state_update",
                    "player_id": player_id,
                    "max_ammo": max_ammo
                })
            }
            SyncEvent::WeaponChanged { player_id, weapon_id } => {
                json!({
                    "type": "weapon_switched",
                    "player_id": player_id,
                    "weapon_id": weapon_id
                })
            }
            SyncEvent::ReloadStateChanged { player_id, is_reloading } => {
                if *is_reloading {
                    json!({
                        "type": "reload_started",
                        "player_id": player_id
                    })
                } else {
                    json!({
                        "type": "reload_finished",
                        "player_id": player_id
                    })
                }
            }
            SyncEvent::PositionChanged { .. } => {
                // Position updates are handled separately
                continue;
            }
        };

        // Serialize to buffer
        buffer.clear();
        if let Ok(data) = serde_json::to_vec(&packet) {
            // Send to all clients in lobby
            for (_player_id, addr) in &lobby.client_addresses {
                if let Err(e) = socket.send_to(&data, *addr).await {
                    log::debug!("Failed to send event to {}: {:?}", addr, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::lobby::Lobby;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn test_process_command_player_join() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        let cmd = LobbyCommand::PlayerJoin {
            player_id: 1,
            name: "Test".to_string(),
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
        };
        
        process_command(&mut lobby, &weapons, cmd);
        
        assert!(lobby.players.contains_key(&1));
        assert!(lobby.client_addresses.contains_key(&1));
    }

    #[test]
    fn test_process_command_shoot() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        // Add shooter and target
        let mut shooter = crate::state::lobby::Player {
            id: 1,
            name: "Shooter".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: std::time::SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 20,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: std::time::SystemTime::now() - std::time::Duration::from_secs(1),
        };
        
        let mut target = crate::state::lobby::Player {
            id: 2,
            name: "Target".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: std::time::SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 20,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: std::time::SystemTime::now(),
        };
        
        lobby.players.insert(1, shooter);
        lobby.players.insert(2, target);
        
        let cmd = LobbyCommand::Shoot { player_id: 1, target_id: 2 };
        process_command(&mut lobby, &weapons, cmd);
        
        let shooter = lobby.players.get(&1).unwrap();
        assert_eq!(shooter.current_ammo, 19);
        
        let target = lobby.players.get(&2).unwrap();
        assert_eq!(target.current_health, 80); // 100 - 20 damage
    }
}

