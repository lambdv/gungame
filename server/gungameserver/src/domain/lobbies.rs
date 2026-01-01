use crate::state::lobby::{Lobby, LobbyCode, Player};
use crate::utils::weapondb::WeaponDb;
use std::net::SocketAddr;
use std::time::SystemTime;

/// Create a new lobby
pub fn create_lobby(
    lobby: &mut Lobby,
    code: LobbyCode,
    _max_players: u32,
    _scene: String,
) -> Result<(), &'static str> {
    if lobby.code != code {
        return Err("Lobby code mismatch");
    }
    // Lobby is already created, just validate
    Ok(())
}

/// Add a player to a lobby
pub fn add_player(
    lobby: &mut Lobby,
    player_id: u32,
    name: String,
    default_weapon_id: u32,
    weapon_data: &WeaponDb,
) -> Result<(), &'static str> {
    if lobby.players.len() >= lobby.max_players as usize {
        return Err("Lobby is full");
    }

    if lobby.players.contains_key(&player_id) {
        return Err("Player already exists");
    }

    let weapon = weapon_data
        .get(default_weapon_id)
        .ok_or("Invalid default weapon")?;

    let player = Player {
        id: player_id,
        name: name.clone(),
        position: (0.0, 1.0, 0.0),
        rotation: (0.0, 0.0, 0.0),
        last_update: SystemTime::now(),
        current_health: 100,
        max_health: 100,
        current_weapon_id: default_weapon_id,
        current_ammo: weapon.ammo,
        max_ammo: weapon.ammo,
        is_reloading: false,
        reload_end_time: None,
        last_shot_time: SystemTime::UNIX_EPOCH,
        kills: 0,
        deaths: 0,
        score: 0,
        killstreak: 0,
        warned_at: None,
        is_dead: false,
        respawn_time: None,
    };

    lobby.players.insert(player_id, player);
    lobby.mark_dirty(player_id);
    Ok(())
}

/// Remove a player from a lobby
pub fn remove_player(lobby: &mut Lobby, player_id: u32) {
    lobby.players.remove(&player_id);
    lobby.client_addresses.remove(&player_id);
    lobby.last_sync_state.remove(&player_id);
}

/// Update player position and rotation
pub fn update_position(
    lobby: &mut Lobby,
    player_id: u32,
    position: (f32, f32, f32),
    rotation: (f32, f32, f32),
) -> Result<(), &'static str> {
    let player = lobby
        .players
        .get_mut(&player_id)
        .ok_or("Player not found")?;

    player.position = position;
    player.rotation = rotation;
    player.last_update = SystemTime::now();

    lobby.mark_dirty(player_id);
    Ok(())
}

/// Set player's UDP address
pub fn set_player_address(
    lobby: &mut Lobby,
    player_id: u32,
    addr: SocketAddr,
) -> Result<(), &'static str> {
    if !lobby.players.contains_key(&player_id) {
        return Err("Player not found");
    }
    lobby.client_addresses.insert(player_id, addr);
    Ok(())
}

/// Clean up inactive players with warning system
/// Returns tuple of (removed_player_ids, warned_player_ids)
pub fn cleanup_inactive(
    lobby: &mut Lobby,
    timeout_secs: u64,
    warning_fraction: f64,
) -> (Vec<u32>, Vec<u32>) {
    let now = SystemTime::now();
    let warning_threshold = (timeout_secs as f64 * warning_fraction) as u64;
    let mut inactive_players = Vec::new();
    let mut warned_players = Vec::new();

    for (player_id, player) in &lobby.players {
        if *player_id == 999 {
            continue;
        }

        if let Ok(duration) = now.duration_since(player.last_update) {
            let elapsed_secs = duration.as_secs();

            if elapsed_secs > timeout_secs {
                inactive_players.push(*player_id);
            } else if elapsed_secs > warning_threshold && player.warned_at.is_none() {
                warned_players.push(*player_id);
            }
        }
    }

    for player_id in &inactive_players {
        remove_player(lobby, *player_id);
    }

    for player_id in &warned_players {
        if let Some(player) = lobby.players.get_mut(player_id) {
            player.warned_at = Some(now);
        }
    }

    (inactive_players, warned_players)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::weapondb::WeaponDb;

    #[test]
    fn test_add_player() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();

        let result = add_player(&mut lobby, 1, "Player1".to_string(), 1, &weapons);
        assert!(result.is_ok());
        assert_eq!(lobby.players.len(), 1);
        assert!(lobby.players.contains_key(&1));
    }

    #[test]
    fn test_add_player_full_lobby() {
        let mut lobby = Lobby::new("TEST".to_string(), 2, "world".to_string());
        let weapons = WeaponDb::load();

        add_player(&mut lobby, 1, "Player1".to_string(), 1, &weapons).unwrap();
        add_player(&mut lobby, 2, "Player2".to_string(), 1, &weapons).unwrap();

        let result = add_player(&mut lobby, 3, "Player3".to_string(), 1, &weapons);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_player() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();

        add_player(&mut lobby, 1, "Player1".to_string(), 1, &weapons).unwrap();
        assert_eq!(lobby.players.len(), 1);

        remove_player(&mut lobby, 1);
        assert_eq!(lobby.players.len(), 0);
    }

    #[test]
    fn test_update_position() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();

        add_player(&mut lobby, 1, "Player1".to_string(), 1, &weapons).unwrap();

        let result = update_position(&mut lobby, 1, (10.0, 2.0, 5.0), (0.0, 1.0, 0.0));
        assert!(result.is_ok());

        let player = lobby.players.get(&1).unwrap();
        assert_eq!(player.position.0, 10.0);
        assert!(lobby.dirty_players.contains(&1));
    }

    #[test]
    fn test_cleanup_inactive() {
        let mut lobby = Lobby::new("TEST".to_string(), 4, "world".to_string());
        let weapons = WeaponDb::load();

        add_player(&mut lobby, 1, "Player1".to_string(), 1, &weapons).unwrap();

        // Manually set old update time
        if let Some(player) = lobby.players.get_mut(&1) {
            player.last_update = SystemTime::now() - std::time::Duration::from_secs(20);
        }

        let (removed, _) = cleanup_inactive(&mut lobby, 15, 0.5);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], 1);
        assert_eq!(lobby.players.len(), 0);
    }
}
