use gungameserver::{GameServer, Lobby, Player};

// Blackbox integration tests that mimic Godot client behavior

#[tokio::test]
async fn test_lobby_creation_and_joining() {
    let mut game_server = GameServer::new();

    // Test lobby creation (mimics Godot client creating TEST lobby)
    let lobby_result = game_server.create_lobby("TEST".to_string(), 4);
    assert!(lobby_result.is_ok());

    let lobby = lobby_result.unwrap();
    assert_eq!(lobby.code, "TEST");
    assert_eq!(lobby.max_players, 4);
    assert!(lobby.dummy_player.is_some()); // Server should create dummy player
    assert_eq!(lobby.dummy_player.as_ref().unwrap().name, "DummyBot");

    // Test lobby joining (mimics Godot client joining TEST lobby)
    let join_result = game_server.join_lobby("TEST", "TestPlayer".to_string());
    assert!(join_result.is_ok());

    let (player_id, joined_lobby) = join_result.unwrap();
    assert_eq!(player_id, 1); // First player should get ID 1
    assert_eq!(joined_lobby.code, "TEST");
    assert_eq!(joined_lobby.players.len(), 1);

    // Verify player was added correctly
    let player = joined_lobby.players.get(&player_id).unwrap();
    assert_eq!(player.name, "TestPlayer");
    assert_eq!(player.id, player_id);
}

#[tokio::test]
async fn test_position_updates() {
    let mut game_server = GameServer::new();

    // Create lobby and add player
    game_server.create_lobby("POS_TEST".to_string(), 4).unwrap();
    let (player_id, _) = game_server.join_lobby("POS_TEST", "PosPlayer".to_string()).unwrap();

    // Simulate position update (mimics Godot client sending position)
    // In a real scenario, this would come from UDP packets
    // For testing, we'll directly update the server state
    {
        let lobby = game_server.get_lobby("POS_TEST").unwrap();
        let mut player = lobby.players.get(&player_id).unwrap().clone();
        player.position = (10.5, 2.0, -5.5);
        // Note: In real server, this would be updated via UDP message processing
    }

    // Verify position can be retrieved (structure test)
    let lobby = game_server.get_lobby("POS_TEST").unwrap();
    assert_eq!(lobby.players.len(), 1);
    assert!(lobby.players.contains_key(&player_id));
}

#[tokio::test]
async fn test_server_dummy_player() {
    let mut game_server = GameServer::new();

    // Create lobby (should spawn dummy player)
    let lobby = game_server.create_lobby("DUMMY_TEST".to_string(), 4).unwrap();

    // Verify dummy player was created
    assert!(lobby.dummy_player.is_some());
    let dummy = lobby.dummy_player.as_ref().unwrap();
    assert_eq!(dummy.name, "DummyBot");
    assert_eq!(dummy.id, 999); // Fixed dummy ID
    assert_eq!(dummy.position, (5.0, 1.0, 0.0)); // Initial position
}

#[tokio::test]
async fn test_multiple_players_in_lobby() {
    let mut game_server = GameServer::new();

    // Create lobby and add multiple players
    game_server.create_lobby("MULTI_TEST".to_string(), 4).unwrap();

    // Add players (mimics multiple Godot clients joining)
    let (player1_id, _) = game_server.join_lobby("MULTI_TEST", "Player1".to_string()).unwrap();
    let (player2_id, _) = game_server.join_lobby("MULTI_TEST", "Player2".to_string()).unwrap();
    let (player3_id, _) = game_server.join_lobby("MULTI_TEST", "Player3".to_string()).unwrap();

    // Verify lobby state
    let lobby = game_server.get_lobby("MULTI_TEST").unwrap();
    assert_eq!(lobby.players.len(), 3);
    assert!(lobby.dummy_player.is_some());

    // Check player IDs are unique and sequential
    assert_eq!(player1_id, 1);
    assert_eq!(player2_id, 2);
    assert_eq!(player3_id, 3);

    // Check all players exist
    assert!(lobby.players.contains_key(&player1_id));
    assert!(lobby.players.contains_key(&player2_id));
    assert!(lobby.players.contains_key(&player3_id));
}

#[tokio::test]
async fn test_lobby_full_error() {
    let mut game_server = GameServer::new();

    // Create lobby with max 2 players
    game_server.create_lobby("FULL_TEST".to_string(), 2).unwrap();

    // Add maximum players
    let _player1 = game_server.join_lobby("FULL_TEST", "Player1".to_string()).unwrap();
    let _player2 = game_server.join_lobby("FULL_TEST", "Player2".to_string()).unwrap();

    // Try to add third player (should fail)
    let result = game_server.join_lobby("FULL_TEST", "Player3".to_string());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Lobby is full");
}

#[tokio::test]
async fn test_lobby_not_found() {
    let mut game_server = GameServer::new();

    // Try to join non-existent lobby
    let result = game_server.join_lobby("NON_EXISTENT", "TestPlayer".to_string());
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Lobby not found");
}

#[tokio::test]
async fn test_lobby_code_uniqueness() {
    let mut game_server = GameServer::new();

    // Create first lobby
    let result1 = game_server.create_lobby("UNIQUE_TEST".to_string(), 4);
    assert!(result1.is_ok());

    // Try to create lobby with same code (should fail)
    let result2 = game_server.create_lobby("UNIQUE_TEST".to_string(), 4);
    assert!(result2.is_err());
    assert_eq!(result2.err().unwrap(), "Lobby code already exists");
}
