use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::net::UdpSocket;
use tokio::time::{interval, Duration};
use crate::state::lobby::Lobby;
use crate::state::commands::{LobbyCommand, drain_and_coalesce};
use crate::state::server_state::ServerState;
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
    server_state: Option<Arc<ServerState>>,
) {
    let tick_interval = Duration::from_millis(config.tick_interval_ms());
    let mut tick_timer = interval(tick_interval);
    let mut send_buffer = PacketBuffer::default();
    let lobby_code = lobby.read().await.code.clone();
    
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
        let kill_events: Vec<logic::KillEvent> = Vec::new();
        let mut respawn_events: Vec<u32> = Vec::new();
        
        // 3. Process all commands
        for cmd in commands {
            // Extract info before processing (to avoid borrow issues)
            let join_info = if let LobbyCommand::PlayerJoin { player_id, ref name, addr } = &cmd {
                Some((*player_id, name.clone(), *addr))
            } else {
                None
            };
            
            let udp_connect_info = if let LobbyCommand::UdpConnect { player_id, ref name, addr } = &cmd {
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
            
            // Process the command
            process_command(&mut lobby_guard, &weapons, cmd, server_state.as_deref());
            
            // Handle special cases that need broadcasting
            if let Some((player_id, name, addr)) = join_info {
                players_joined.push((player_id, name.clone()));
                // Send welcome message to new player with current lobby state
                send_welcome_message(&lobby_guard, &socket, player_id, addr).await;
            }
            
            if let Some((player_id, name, addr)) = udp_connect_info {
                players_joined.push((player_id, name.clone()));
                // For UDP connect, player already has scene info from HTTP join
                // Just send acknowledgment without scene info to avoid scene reload
                send_udp_connected_message(&lobby_guard, &socket, player_id, addr).await;
                log::debug!("Player {} ({}) UDP connected, broadcasting join to lobby", player_id, name);
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
        
        // 5. Check respawn timers for dead players
        let now = std::time::SystemTime::now();
        let mut players_to_respawn: Vec<u32> = Vec::new();
        for (player_id, player) in &lobby_guard.players {
            if player.is_dead {
                if let Some(respawn_time) = player.respawn_time {
                    if now >= respawn_time {
                        players_to_respawn.push(*player_id);
                    }
                }
            }
        }
        
        // Respawn players and track events
        for player_id in players_to_respawn {
            if let Err(e) = logic::respawn_player(&mut lobby_guard, player_id) {
                log::debug!("Respawn failed for player {}: {}", player_id, e);
            } else {
                respawn_events.push(player_id);
                log::debug!("Player {} respawned in lobby {}", player_id, lobby_code);
            }
        }
        
        // 6. Cleanup inactive players periodically (every 5 seconds worth of ticks)
        // Use a local counter that persists across ticks via closure
        // For MVP, we'll do cleanup every tick (can be optimized later)
        let (removed, _warned) = lobbies::cleanup_inactive(
            &mut lobby_guard,
            config.player_inactivity_timeout_secs,
            0.5, // Warn at 50% of timeout
        );
        if !removed.is_empty() {
            for player_id in &removed {
                players_left.push(*player_id);
            }
        }
        
        // 6. Broadcast player join/leave events
        log::debug!("Lobby {} has {} players and {} addresses", 
            lobby_code, lobby_guard.players.len(), lobby_guard.client_addresses.len());
        log::debug!("Players: {:?}", lobby_guard.players.keys().collect::<Vec<_>>());
        log::debug!("Addresses: {:?}", lobby_guard.client_addresses.iter()
            .map(|(k, v)| (k, format!("{}", v)))
            .collect::<Vec<_>>());
        
        if !players_joined.is_empty() {
            log::debug!("Broadcasting player joins: {:?}", players_joined);
            broadcast_player_join_events(&lobby_guard, &socket, &players_joined).await;
        }
        if !players_left.is_empty() {
            log::debug!("Broadcasting player leaves: {:?}", players_left);
            broadcast_player_leave_events(&lobby_guard, &socket, &players_left).await;
        }
        
        // 7. Broadcast position updates (every tick for players that moved)
        if !position_updates.is_empty() {
            // log::debug!("Broadcasting position updates for {} players: {:?}", position_updates.len(), position_updates);
            broadcast_position_updates(&lobby_guard, &socket, &position_updates).await;
        }
        
        // 8. Broadcast kill events
        if !kill_events.is_empty() {
            for kill_event in &kill_events {
                broadcast_kill_event(&lobby_guard, &socket, kill_event).await;
            }
        }
        
        // 9. Broadcast respawn events
        if !respawn_events.is_empty() {
            broadcast_respawn_events(&lobby_guard, &socket, &respawn_events).await;
        }
        
        // 10. Delta sync - only send changes (health, ammo, weapon, reload)
        let state_events = delta_sync::collect_dirty_events(&mut lobby_guard);
        
        // 11. Broadcast state events (reuse buffer)
        if !state_events.is_empty() {
            broadcast_state_events(&lobby_guard, &socket, &state_events, &mut send_buffer).await;
        }
        
        // 12. Record stats to global stats and clear dirty flags
        if let Some(ref state) = server_state {
            for player_id in &players_left {
                if let Some(player) = lobby_guard.players.get(player_id) {
                    state.global_stats.record_session(
                        player.id,
                        &player.name,
                        player.kills,
                        player.deaths,
                        player.score,
                    );
                }
            }
        }
        
        lobby_guard.clear_dirty();
    }
}

/// Process a single command
fn process_command(
    lobby: &mut Lobby,
    weapons: &WeaponDb,
    cmd: LobbyCommand,
    server_state: Option<&ServerState>,
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
            if let Some(state) = server_state {
                state.register_player_lobby(player_id, &lobby.code);
            }
        }
        LobbyCommand::PlayerLeave { player_id } => {
            lobbies::remove_player(lobby, player_id);
            if let Some(state) = server_state {
                state.unregister_player(player_id);
            }
        }
        LobbyCommand::UdpConnect { player_id, name: _, addr } => {
            if lobby.players.contains_key(&player_id) {
                lobby.client_addresses.insert(player_id, addr);
                if let Some(player) = lobby.players.get_mut(&player_id) {
                    player.last_update = std::time::SystemTime::now();
                }
                if let Some(state) = server_state {
                    state.register_player_lobby(player_id, &lobby.code);
                }
                log::debug!("Player {} UDP connected from {}, now has {} addresses", 
                    player_id, addr, lobby.client_addresses.len());
            } else {
                log::warn!("UDP connect for unknown player {} from {}", player_id, addr);
            }
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
        "player_id": player_id,
        "scene_load": true
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
        "players": player_list,
        "notification": true
    });

    if let Ok(data) = serde_json::to_vec(&players_packet) {
        let _ = socket.send_to(&data, addr).await;
    }
}

/// Send UDP connection acknowledgment without scene info
/// Used when player reconnects via UDP after HTTP join
async fn send_udp_connected_message(
    lobby: &Lobby,
    socket: &UdpSocket,
    player_id: u32,
    addr: std::net::SocketAddr,
) {
    let ack_packet = json!({
        "type": "udp_connected",
        "player_id": player_id,
        "lobby_code": lobby.code,
        "notification": true
    });

    if let Ok(data) = serde_json::to_vec(&ack_packet) {
        let _ = socket.send_to(&data, addr).await;
    }

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
        "players": player_list,
        "notification": true
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
        log::debug!("Sending player_joined to others for player {} ({})", player_id, name);
        
        let packet = json!({
            "type": "player_joined",
            "player": {
                "id": player_id,
                "name": name
            },
            "notification": true
        });

        if let Ok(data) = serde_json::to_vec(&packet) {
            // Send to all clients except the joining player
            let recipients: Vec<(u32, std::net::SocketAddr)> = lobby.client_addresses.iter()
                .filter(|(cid, _)| **cid != *player_id)
                .map(|(cid, addr)| (*cid, *addr))
                .collect();
            
            log::debug!("Sending to {} recipients: {:?}", recipients.len(), recipients);
            
            for (client_id, addr) in recipients {
                log::debug!("Sending player_joined to client {} at {}", client_id, addr);
                if let Err(e) = socket.send_to(&data, addr).await {
                    log::debug!("Failed to send join event to {} ({}): {:?}", client_id, addr, e);
                } else {
                    log::debug!("Successfully sent player_joined to client {} at {}", client_id, addr);
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
            // log::debug!("Broadcasting position for player {}: ({}, {}, {})", 
            //     player_id, player.position.0, player.position.1, player.position.2);
            
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
                let recipients: Vec<(u32, std::net::SocketAddr)> = lobby.client_addresses.iter()
                    .filter(|(cid, _)| **cid != *player_id)
                    .map(|(cid, addr)| (*cid, *addr))
                    .collect();
                
                // log::debug!("Sending position update to {} recipients: {:?}", recipients.len(), recipients);
                
            for (client_id, addr) in recipients {
                // log::debug!("Sending position update to client {} at {}", client_id, addr);
                if let Err(e) = socket.send_to(&data, addr).await {
                    // log::debug!("Failed to send position update to {} ({}): {:?}", client_id, addr, e);
                } else {
                    // log::debug!("Successfully sent position update to client {} at {}", client_id, addr);
                }
            }
            }
        }
    }
}

/// Broadcast kill event to all clients
async fn broadcast_kill_event(
    lobby: &Lobby,
    socket: &UdpSocket,
    event: &logic::KillEvent,
) {
    let packet = json!({
        "type": "player_killed",
        "killer_id": event.killer_id,
        "killer_name": event.killer_name,
        "victim_id": event.victim_id,
        "victim_name": event.victim_name,
        "weapon_id": event.weapon_id,
        "weapon_name": event.weapon_name,
        "killer_killstreak": event.killer_new_killstreak
    });

    if let Ok(data) = serde_json::to_vec(&packet) {
        for (_player_id, addr) in &lobby.client_addresses {
            if let Err(e) = socket.send_to(&data, *addr).await {
                log::debug!("Failed to send kill event to {}: {:?}", addr, e);
            }
        }
    }
}

/// Broadcast respawn events to all clients
async fn broadcast_respawn_events(
    lobby: &Lobby,
    socket: &UdpSocket,
    player_ids: &[u32],
) {
    for player_id in player_ids {
        let packet = json!({
            "type": "player_respawned",
            "player_id": player_id
        });

        if let Ok(data) = serde_json::to_vec(&packet) {
            for (_player_id, addr) in &lobby.client_addresses {
                if let Err(e) = socket.send_to(&data, *addr).await {
                    log::debug!("Failed to send respawn event to {}: {:?}", addr, e);
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
            SyncEvent::PlayerKilled { killer_id, killer_name, victim_id, victim_name, weapon_id, weapon_name, killer_killstreak } => {
                json!({
                    "type": "player_killed",
                    "killer_id": killer_id,
                    "killer_name": killer_name,
                    "victim_id": victim_id,
                    "victim_name": victim_name,
                    "weapon_id": weapon_id,
                    "weapon_name": weapon_name,
                    "killer_killstreak": killer_killstreak
                })
            }
            SyncEvent::PlayerRespawned { player_id } => {
                json!({
                    "type": "player_respawned",
                    "player_id": player_id
                })
            }
            SyncEvent::ScoreChanged { player_id, score, kills, deaths, killstreak } => {
                json!({
                    "type": "score_update",
                    "player_id": player_id,
                    "score": score,
                    "kills": kills,
                    "deaths": deaths,
                    "killstreak": killstreak
                })
            }
            SyncEvent::PlayerKicked { player_id, reason } => {
                json!({
                    "type": "player_kicked",
                    "player_id": player_id,
                    "reason": reason
                })
            }
            SyncEvent::InactivityWarning { player_id, seconds_remaining } => {
                json!({
                    "type": "inactivity_warning",
                    "player_id": player_id,
                    "seconds_remaining": seconds_remaining
                })
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
        
        process_command(&mut lobby, &weapons, cmd, None);
        
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
            kills: 0,
            deaths: 0,
            score: 0,
            killstreak: 0,
            warned_at: None,
            is_dead: false,
            respawn_time: None,
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
            kills: 0,
            deaths: 0,
            score: 0,
            killstreak: 0,
            warned_at: None,
            is_dead: false,
            respawn_time: None,
        };
        
        lobby.players.insert(1, shooter);
        lobby.players.insert(2, target);
        
        let cmd = LobbyCommand::Shoot { player_id: 1, target_id: 2 };
        process_command(&mut lobby, &weapons, cmd, None);
        
        let shooter = lobby.players.get(&1).unwrap();
        assert_eq!(shooter.current_ammo, 19);
        
        let target = lobby.players.get(&2).unwrap();
        assert_eq!(target.current_health, 80); // 100 - 20 damage
    }
}

