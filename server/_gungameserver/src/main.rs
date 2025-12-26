mod handlers;
mod simulator;
mod validation;
mod server;

use fern;
use chrono;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging()?;
    let game_server = Arc::new(RwLock::new(handlers::server::GameServer::new()));
    server::start_servers(game_server).await?;
    Ok(())
}

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Utc::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("gungame.log")?)
        .apply()?;
    Ok(())
}

// let game_server: Arc<RwLock<GameServer>> = Arc::new(RwLock::new(GameServer::new()));

// {
//     let mut server = game_server.write().await;
//     match server.create_lobby("test".to_string(), 8, "test_world".to_string()) {
//         Ok(_) => {
//             if let Some(lobby) = server.get_lobby("test") {
//                 println!("✅ Test server 'test' created successfully");
//                 println!("   - Code: {}", lobby.code);
//                 println!("   - Max players: {}", lobby.max_players);
//                 println!("   - Scene: {}", lobby.scene);
//             }
//         }
//         Err(e) => {
//             eprintln!("❌ Failed to create test server: {}", e);
//         }
//     }
// }

// // HTTP Server
// let app_state: AppState = game_server.clone();
// let app = Router::new()
//     .route("/lobbies", post(create_lobby))
//     .route("/lobbies", get(list_lobbies))
//     .route("/lobbies/:code/join", post(join_lobby))
//     .route("/lobbies/:code", get(get_lobby))
//     .layer(CorsLayer::permissive())
//     .with_state(app_state);

// // Start HTTP server
// let http_addr = "0.0.0.0:8080";
// println!("Starting HTTP server on {}", http_addr);
// let http_server = tokio::spawn(async move {
//     let listener = match tokio::net::TcpListener::bind(http_addr).await {
//         Ok(listener) => {
//             println!("HTTP server successfully bound to {}", http_addr);
//             listener
//         }
//         Err(e) => {
//             eprintln!("Failed to bind HTTP server to {}: {}", http_addr, e);
//             eprintln!("Make sure no other process is using port 8080");
//             return;
//         }
//     };

//     if let Err(e) = axum::serve(listener, app).await {
//         eprintln!("HTTP server error: {}", e);
//     }
// });

// // UDP Server for real-time communication
// let udp_addr = "0.0.0.0:8081";
// println!("Starting UDP server on {}", udp_addr);

// let udp_socket: Arc<UdpSocket> = Arc::new(match UdpSocket::bind(udp_addr).await {
//     Ok(socket) => {
//         println!("UDP server successfully bound to {}", udp_addr);
//         socket
//     }
//     Err(e) => {
//         eprintln!("Failed to bind UDP server to {}: {}", udp_addr, e);
//         eprintln!("Make sure no other process is using port 8081");
//         return Err(e.into());
//     }
// });
// let udp_socket_clone = udp_socket.clone();

// let game_server_clone = game_server.clone();
// let udp_server = tokio::spawn(async move {
//     let _ = udp_server_loop(udp_socket, game_server_clone).await;
// });

// // Dummy player movement task (commented out - function doesn't exist yet)
// // let dummy_mover_game_server = game_server.clone();
// // let dummy_mover_socket = udp_socket_clone.clone();
// // let dummy_mover = tokio::spawn(async move {
// //     let _ = dummy_player_mover(dummy_mover_socket, dummy_mover_game_server).await;
// // });

// // Player cleanup task - runs every 5 seconds
// let cleanup_game_server = game_server.clone();
// let cleanup_socket = udp_socket_clone.clone();
// let cleanup_task = tokio::spawn(async move {
//     let mut interval = interval(Duration::from_secs(5));
//     loop {
//         interval.tick().await;
//         let mut server = cleanup_game_server.write().await;
//         let completed_reloads = server.update_reload_states();

//         // Broadcast reload finished events
//         for (lobby_code, player_id) in completed_reloads {
//             if let Some(lobby) = server.lobbies.get(&lobby_code) {
//                 let reload_finished_packet = serde_json::json!({
//                     "type": "reload_finished",
//                     "player_id": player_id
//                 });
//                 let packet_data = serde_json::to_vec(&reload_finished_packet).unwrap();

//                 // Send to all clients in the lobby
//                 for (client_id, client_addr) in &lobby.client_addresses {
//                     if let Err(e) = cleanup_socket.send_to(&packet_data, *client_addr).await {
//                         println!("Failed to send reload finished to player {}: {:?}", client_id, e);
//                     }
//                 }
//             }
//         }

//         let (players_removed, lobbies_deleted) = server.cleanup_inactive_players_and_empty_lobbies();
//         if players_removed > 0 || lobbies_deleted > 0 {
//             println!("Cleanup: removed {} inactive players, deleted {} empty lobbies", players_removed, lobbies_deleted);
//         }
//     }
// });

// // State synchronization task - runs every 500ms with change detection
// let sync_game_server = game_server.clone();
// let sync_socket = udp_socket_clone.clone();
// let sync_task = tokio::spawn(async move {
//     let mut interval = interval(Duration::from_millis(500)); // 2 updates per second
//     let mut last_sync_states: HashMap<String, Vec<PlayerSyncState>> = HashMap::new();

//     loop {
//         interval.tick().await;
//         let server = sync_game_server.read().await;

//         // Send state sync to all lobbies only if state changed
//         for (lobby_code, lobby) in &server.lobbies {
//             if let Ok(current_states) = server.get_lobby_state_sync(lobby_code) {
//                 // Check if state has actually changed
//                 let should_sync = if let Some(last_states) = last_sync_states.get(lobby_code) {
//                     // Compare relevant state (health, ammo, weapon, reload) - exclude position/rotation
//                     !current_states.iter().zip(last_states.iter()).all(|(curr, last)| {
//                         curr.id == last.id &&
//                         curr.health == last.health &&
//                         curr.max_health == last.max_health &&
//                         curr.current_weapon_id == last.current_weapon_id &&
//                         curr.current_ammo == last.current_ammo &&
//                         curr.max_ammo == last.max_ammo &&
//                         curr.is_reloading == last.is_reloading
//                     })
//                 } else {
//                     // First time syncing this lobby
//                     true
//                 };

//                 if should_sync {
//                     let state_sync_packet = serde_json::json!({
//                         "type": "state_sync",
//                         "players": current_states
//                     });
//                     let packet_data = serde_json::to_vec(&state_sync_packet).unwrap();

//                     // Send to all clients in the lobby
//                     for (client_id, client_addr) in &lobby.client_addresses {
//                         if let Err(e) = sync_socket.send_to(&packet_data, *client_addr).await {
//                             println!("Failed to send state sync to player {}: {:?}", client_id, e);
//                         }
//                     }

//                     // Update last sync state
//                     last_sync_states.insert(lobby_code.clone(), current_states);
//                 }
//             }
//         }
//     }
// });

// // Wait for all tasks
// tokio::try_join!(http_server, udp_server, cleanup_task, sync_task)?;
