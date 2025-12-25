use crate::state::lobby::Lobby;
use crate::utils::buffers::{SmallEventVec, SyncEvent};

/// Collect dirty events for delta-based state sync
/// Only includes changed fields compared to last sync state
pub fn collect_dirty_events(lobby: &mut Lobby) -> SmallEventVec {
    let mut events = SmallEventVec::new();
    
    for &player_id in &lobby.dirty_players {
        if let Some(player) = lobby.players.get(&player_id) {
            let last = lobby.last_sync_state.get(&player_id);
            
            // Only include changed fields
            if last.map(|l| l.health != player.current_health).unwrap_or(true) {
                events.push(SyncEvent::HealthChanged { 
                    player_id, 
                    health: player.current_health 
                });
            }
            
            if last.map(|l| l.max_health != player.max_health).unwrap_or(true) {
                // Max health rarely changes, but include if it does
            }
            
            if last.map(|l| l.current_ammo != player.current_ammo).unwrap_or(true) {
                events.push(SyncEvent::AmmoChanged { 
                    player_id, 
                    ammo: player.current_ammo 
                });
            }
            
            if last.map(|l| l.max_ammo != player.max_ammo).unwrap_or(true) {
                events.push(SyncEvent::MaxAmmoChanged { 
                    player_id, 
                    max_ammo: player.max_ammo 
                });
            }
            
            if last.map(|l| l.current_weapon_id != player.current_weapon_id).unwrap_or(true) {
                events.push(SyncEvent::WeaponChanged { 
                    player_id, 
                    weapon_id: player.current_weapon_id 
                });
            }
            
            if last.map(|l| l.is_reloading != player.is_reloading).unwrap_or(true) {
                events.push(SyncEvent::ReloadStateChanged { 
                    player_id, 
                    is_reloading: player.is_reloading 
                });
            }
            
            // Position changes are handled separately (more frequent)
            // Only sync position if it's a new player or significant change
            
            // Update last sync state
            lobby.last_sync_state.insert(player_id, player.to_sync_state());
        }
    }
    
    events
}

/// Collect position updates for players (separate from state sync)
pub fn collect_position_events(lobby: &Lobby, player_ids: &[u32]) -> SmallEventVec {
    let mut events = SmallEventVec::new();
    
    for &player_id in player_ids {
        if let Some(player) = lobby.players.get(&player_id) {
            events.push(SyncEvent::PositionChanged {
                player_id,
                position: player.position,
                rotation: player.rotation,
            });
        }
    }
    
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::lobby::Lobby;
    use std::time::SystemTime;

    #[test]
    fn test_collect_dirty_events() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        
        // Add player
        let mut player = crate::state::lobby::Player {
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
        lobby.players.insert(1, player);
        lobby.mark_dirty(1);
        
        let events = collect_dirty_events(&mut lobby);
        // Should have events for new player
        assert!(!events.is_empty());
    }

    #[test]
    fn test_collect_dirty_events_no_changes() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        
        let mut player = crate::state::lobby::Player {
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
        lobby.players.insert(1, player);
        
        // Set last sync state to match current
        lobby.last_sync_state.insert(1, lobby.players.get(&1).unwrap().to_sync_state());
        
        // Mark dirty but no actual changes
        lobby.mark_dirty(1);
        
        let events = collect_dirty_events(&mut lobby);
        // Should have no events since nothing changed
        assert!(events.is_empty());
    }

    #[test]
    fn test_collect_position_events() {
        let lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let player_ids = vec![1, 2];
        
        let events = collect_position_events(&lobby, &player_ids);
        // Empty since no players exist
        assert!(events.is_empty());
    }
}

