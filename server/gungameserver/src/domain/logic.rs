use crate::state::lobby::{Lobby, PlayerSyncState};
use crate::utils::weapondb::WeaponDb;
use std::time::SystemTime;

/// Try to shoot - validates ammo, fire rate, reload state
/// Returns true if shot was successful
pub fn try_shoot(
    lobby: &mut Lobby,
    weapons: &WeaponDb,
    player_id: u32,
) -> Result<bool, &'static str> {
    let player = lobby.players.get_mut(&player_id)
        .ok_or("Player not found")?;

    // Check if player is reloading
    if player.is_reloading {
        return Ok(false);
    }

    // Check ammo
    if player.current_ammo == 0 {
        return Ok(false);
    }

    // Check fire rate
    let weapon = weapons.get(player.current_weapon_id)
        .ok_or("Invalid weapon")?;
    
    let now = SystemTime::now();
    let time_since_last_shot = now.duration_since(player.last_shot_time)
        .map_err(|_| "Time error")?;

    if time_since_last_shot.as_secs_f32() < (1.0 / weapon.fire_rate) {
        return Ok(false); // Too soon to shoot again
    }

    // Consume ammo
    player.current_ammo = player.current_ammo.saturating_sub(1);
    player.last_shot_time = now;
    
    lobby.mark_dirty(player_id);
    Ok(true)
}

/// Apply damage to a player
pub fn apply_damage(
    lobby: &mut Lobby,
    target_id: u32,
    damage: u32,
) -> Result<(), &'static str> {
    let player = lobby.players.get_mut(&target_id)
        .ok_or("Player not found")?;

    // Validate damage is reasonable
    if damage == 0 || damage > 100 {
        return Err("Invalid damage amount");
    }

    // Apply damage with underflow protection
    player.current_health = player.current_health.saturating_sub(damage);
    
    lobby.mark_dirty(target_id);
    Ok(())
}

/// Start player reload
pub fn start_reload(
    lobby: &mut Lobby,
    weapons: &WeaponDb,
    player_id: u32,
) -> Result<(), &'static str> {
    let player = lobby.players.get_mut(&player_id)
        .ok_or("Player not found")?;

    // Can't reload if already reloading or at max ammo
    if player.is_reloading || player.current_ammo == player.max_ammo {
        return Err("Cannot reload");
    }

    let weapon = weapons.get(player.current_weapon_id)
        .ok_or("Weapon not found")?;
    
    player.is_reloading = true;
    player.reload_end_time = Some(
        SystemTime::now() + std::time::Duration::from_secs_f32(weapon.reload_time)
    );
    
    lobby.mark_dirty(player_id);
    Ok(())
}

/// Update reload states - check and complete finished reloads
/// Returns list of (player_id) that completed reload
pub fn update_reload_states(lobby: &mut Lobby) -> Vec<u32> {
    let now = SystemTime::now();
    let mut completed_reloads = Vec::new();

    // First pass: update reload states
    for player in lobby.players.values_mut() {
        if player.is_reloading {
            if let Some(end_time) = player.reload_end_time {
                if now >= end_time {
                    // Reload complete
                    player.current_ammo = player.max_ammo;
                    player.is_reloading = false;
                    player.reload_end_time = None;
                    completed_reloads.push(player.id);
                }
            }
        }
    }

    // Second pass: mark dirty (after mutable borrow is released)
    for player_id in &completed_reloads {
        lobby.mark_dirty(*player_id);
    }

    completed_reloads
}

/// Switch player weapon
pub fn switch_weapon(
    lobby: &mut Lobby,
    weapons: &WeaponDb,
    player_id: u32,
    weapon_id: u32,
) -> Result<(), &'static str> {
    let player = lobby.players.get_mut(&player_id)
        .ok_or("Player not found")?;

    // Validate weapon exists
    if !weapons.contains(weapon_id) {
        return Err("Invalid weapon");
    }

    // Update player's weapon and reset ammo
    let weapon = weapons.get(weapon_id).unwrap();
    player.current_weapon_id = weapon_id;
    player.current_ammo = weapon.ammo;
    player.max_ammo = weapon.ammo;

    // Cancel any ongoing reload
    player.is_reloading = false;
    player.reload_end_time = None;
    
    lobby.mark_dirty(player_id);
    Ok(())
}

/// Get player's current sync state
pub fn get_player_state(
    lobby: &Lobby,
    player_id: u32,
) -> Result<PlayerSyncState, &'static str> {
    let player = lobby.players.get(&player_id)
        .ok_or("Player not found")?;
    Ok(player.to_sync_state())
}

/// Get full state sync data for all players in a lobby
pub fn get_lobby_state_sync(lobby: &Lobby) -> Vec<PlayerSyncState> {
    lobby.players.values()
        .map(|player| player.to_sync_state())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::weapondb::WeaponDb;

    #[test]
    fn test_try_shoot_success() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        // Add player with ammo
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
            last_shot_time: SystemTime::now() - std::time::Duration::from_secs(1),
        };
        lobby.players.insert(1, player);
        
        let result = try_shoot(&mut lobby, &weapons, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        
        let player = lobby.players.get(&1).unwrap();
        assert_eq!(player.current_ammo, 19);
    }

    #[test]
    fn test_try_shoot_no_ammo() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        let mut player = crate::state::lobby::Player {
            id: 1,
            name: "Test".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 0,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: SystemTime::now(),
        };
        lobby.players.insert(1, player);
        
        let result = try_shoot(&mut lobby, &weapons, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_apply_damage() {
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
        
        let result = apply_damage(&mut lobby, 1, 25);
        assert!(result.is_ok());
        
        let player = lobby.players.get(&1).unwrap();
        assert_eq!(player.current_health, 75);
    }

    #[test]
    fn test_start_reload() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        let mut player = crate::state::lobby::Player {
            id: 1,
            name: "Test".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 10,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: SystemTime::now(),
        };
        lobby.players.insert(1, player);
        
        let result = start_reload(&mut lobby, &weapons, 1);
        assert!(result.is_ok());
        
        let player = lobby.players.get(&1).unwrap();
        assert!(player.is_reloading);
        assert!(player.reload_end_time.is_some());
    }

    #[test]
    fn test_switch_weapon() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();
        
        let mut player = crate::state::lobby::Player {
            id: 1,
            name: "Test".to_string(),
            position: (0.0, 1.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            last_update: SystemTime::now(),
            current_health: 100,
            max_health: 100,
            current_weapon_id: 1,
            current_ammo: 10,
            max_ammo: 20,
            is_reloading: false,
            reload_end_time: None,
            last_shot_time: SystemTime::now(),
        };
        lobby.players.insert(1, player);
        
        let result = switch_weapon(&mut lobby, &weapons, 1, 2);
        assert!(result.is_ok());
        
        let player = lobby.players.get(&1).unwrap();
        assert_eq!(player.current_weapon_id, 2);
        assert_eq!(player.current_ammo, 8); // Prototype ammo
    }
}

