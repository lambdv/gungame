use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// Command sent from network handlers to lobby tick loop
#[derive(Debug, Clone)]
pub enum LobbyCommand {
    // Player management
    PlayerJoin {
        player_id: u32,
        name: String,
        addr: SocketAddr,
    },
    PlayerLeave {
        player_id: u32,
    },
    
    // Position (only latest kept per player)
    PositionUpdate {
        player_id: u32,
        position: (f32, f32, f32),
        rotation: (f32, f32, f32),
        addr: SocketAddr,  // Track UDP address for broadcasting
    },
    
    // Combat
    Shoot {
        player_id: u32,
        target_id: u32,
    },
    Reload {
        player_id: u32,
    },
    WeaponSwitch {
        player_id: u32,
        weapon_id: u32,
    },
    
    // Keepalive
    Heartbeat {
        player_id: u32,
        addr: SocketAddr,  // Track UDP address for broadcasting
    },
}

/// Coalesce commands from queue, keeping only latest position per player
/// This drops stale position packets and prevents queue overflow
pub fn drain_and_coalesce(
    rx: &mut mpsc::Receiver<LobbyCommand>
) -> Vec<LobbyCommand> {
    let mut latest_positions: HashMap<u32, LobbyCommand> = HashMap::new();
    let mut other_commands: Vec<LobbyCommand> = Vec::new();
    
    // Drain all available commands
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            LobbyCommand::PositionUpdate { player_id, .. } => {
                // Keep only the LATEST position per player
                latest_positions.insert(player_id, cmd);
            }
            _ => other_commands.push(cmd),
        }
    }
    
    // Return: other commands first, then latest positions
    other_commands.extend(latest_positions.into_values());
    other_commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    }

    #[tokio::test]
    async fn test_position_coalescing() {
        let (tx, mut rx) = mpsc::channel(100);
        let addr = test_addr();
        
        // Send multiple position updates for same player
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (1.0, 1.0, 1.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (2.0, 2.0, 2.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (3.0, 3.0, 3.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        
        let commands = drain_and_coalesce(&mut rx);
        
        // Should only have latest position
        assert_eq!(commands.len(), 1);
        if let LobbyCommand::PositionUpdate { position, .. } = &commands[0] {
            assert_eq!(position.0, 3.0);
        } else {
            panic!("Expected PositionUpdate");
        }
    }

    #[tokio::test]
    async fn test_mixed_commands() {
        let (tx, mut rx) = mpsc::channel(100);
        let addr = test_addr();
        
        tx.send(LobbyCommand::Shoot { player_id: 1, target_id: 2 }).await.unwrap();
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (1.0, 1.0, 1.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        tx.send(LobbyCommand::Reload { player_id: 1 }).await.unwrap();
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (2.0, 2.0, 2.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        
        let commands = drain_and_coalesce(&mut rx);
        
        // Should have: Shoot, Reload, then latest Position
        assert_eq!(commands.len(), 3);
        assert!(matches!(commands[0], LobbyCommand::Shoot { .. }));
        assert!(matches!(commands[1], LobbyCommand::Reload { .. }));
        assert!(matches!(commands[2], LobbyCommand::PositionUpdate { .. }));
    }

    #[tokio::test]
    async fn test_multiple_players_positions() {
        let (tx, mut rx) = mpsc::channel(100);
        let addr = test_addr();
        
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (1.0, 1.0, 1.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 2,
            position: (2.0, 2.0, 2.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        tx.send(LobbyCommand::PositionUpdate {
            player_id: 1,
            position: (3.0, 3.0, 3.0),
            rotation: (0.0, 0.0, 0.0),
            addr,
        }).await.unwrap();
        
        let commands = drain_and_coalesce(&mut rx);
        
        // Should have latest position for each player
        assert_eq!(commands.len(), 2);
        let mut player_ids: Vec<u32> = commands.iter()
            .filter_map(|c| {
                if let LobbyCommand::PositionUpdate { player_id, .. } = c {
                    Some(*player_id)
                } else {
                    None
                }
            })
            .collect();
        player_ids.sort();
        assert_eq!(player_ids, vec![1, 2]);
    }
}

