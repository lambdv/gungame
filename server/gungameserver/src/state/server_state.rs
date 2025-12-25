use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use crate::state::lobby::{Lobby, LobbyCode};

/// Handle to a lobby with its command queue and tick task
pub struct LobbyHandle {
    pub lobby: Arc<RwLock<Lobby>>,
    pub command_tx: mpsc::Sender<crate::state::commands::LobbyCommand>,
    pub task_handle: JoinHandle<()>,
}

/// Server state partitioned by lobby
/// Uses DashMap for concurrent access without global locks
pub struct ServerState {
    lobbies: DashMap<LobbyCode, LobbyHandle>,
    next_player_id: AtomicU32,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            lobbies: DashMap::new(),
            next_player_id: AtomicU32::new(1),
        }
    }

    /// Get command sender for a lobby (for UDP handlers)
    /// Returns None if lobby doesn't exist
    pub fn get_lobby_tx(&self, lobby_code: &str) -> Option<mpsc::Sender<crate::state::commands::LobbyCommand>> {
        self.lobbies.get(lobby_code)
            .map(|entry| entry.command_tx.clone())
    }

    /// Get lobby handle (for HTTP handlers)
    pub fn get_lobby(&self, lobby_code: &str) -> Option<Arc<RwLock<Lobby>>> {
        self.lobbies.get(lobby_code)
            .map(|entry| entry.lobby.clone())
    }

    /// Check if lobby exists
    pub fn lobby_exists(&self, lobby_code: &str) -> bool {
        self.lobbies.contains_key(lobby_code)
    }

    /// Generate next player ID (lock-free)
    pub fn next_player_id(&self) -> u32 {
        self.next_player_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Insert a new lobby handle
    pub fn insert_lobby(&self, code: LobbyCode, handle: LobbyHandle) {
        self.lobbies.insert(code, handle);
    }

    /// Remove a lobby (graceful shutdown)
    pub fn remove_lobby(&self, lobby_code: &str) -> Option<LobbyHandle> {
        self.lobbies.remove(lobby_code).map(|(_, handle)| handle)
    }

    /// Iterate over all lobbies (for cleanup tasks)
    pub fn iter_lobbies(&self) -> dashmap::iter::Iter<'_, LobbyCode, LobbyHandle> {
        self.lobbies.iter()
    }

    /// Get lobby count
    pub fn lobby_count(&self) -> usize {
        self.lobbies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::commands::LobbyCommand;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn test_server_state_creation() {
        let state = ServerState::new();
        assert_eq!(state.lobby_count(), 0);
    }

    #[test]
    fn test_player_id_generation() {
        let state = ServerState::new();
        let id1 = state.next_player_id();
        let id2 = state.next_player_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_lobby_handle_creation() {
        let lobby = Arc::new(RwLock::new(Lobby::new("TEST".to_string(), 4, "world".to_string())));
        let (tx, _rx) = mpsc::channel::<LobbyCommand>(100);
        let handle = JoinHandle::from(tokio::spawn(async {}));
        
        let lobby_handle = LobbyHandle {
            lobby: lobby.clone(),
            command_tx: tx,
            task_handle: handle,
        };
        
        let state = ServerState::new();
        state.insert_lobby("TEST".to_string(), lobby_handle);
        
        assert!(state.lobby_exists("TEST"));
        assert_eq!(state.lobby_count(), 1);
    }

    #[tokio::test]
    async fn test_get_lobby_tx() {
        let lobby = Arc::new(RwLock::new(Lobby::new("TEST".to_string(), 4, "world".to_string())));
        let (tx, _rx) = mpsc::channel::<LobbyCommand>(100);
        let handle = JoinHandle::from(tokio::spawn(async {}));
        
        let lobby_handle = LobbyHandle {
            lobby,
            command_tx: tx.clone(),
            task_handle: handle,
        };
        
        let state = ServerState::new();
        state.insert_lobby("TEST".to_string(), lobby_handle);
        
        let retrieved_tx = state.get_lobby_tx("TEST");
        assert!(retrieved_tx.is_some());
        
        // Can send command
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        retrieved_tx.unwrap().send(LobbyCommand::Heartbeat { player_id: 1, addr }).await.unwrap();
    }
}

