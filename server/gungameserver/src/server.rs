use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use log::info;
use tokio::net::{TcpListener, UdpSocket};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use crate::state::server_state::{ServerState, LobbyHandle};
use crate::state::lobby::Lobby;
use crate::handlers::http::{create_lobby, list_lobbies, join_lobby, get_lobby, AppState};
use crate::handlers::udp::handle_udp_packet;
use crate::tick::lobby_tick::lobby_tick_loop;
use crate::utils::weapondb::WeaponDb;
use crate::utils::config::Config;

/// Start HTTP and UDP servers
pub async fn start_servers(
    state: Arc<ServerState>,
    weapons: Arc<WeaponDb>,
    config: Arc<Config>,
    udp_socket: Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http_server = init_http_server(state.clone(), weapons.clone(), config.clone(), udp_socket.clone());
    let udp_server = init_udp_server(state.clone(), udp_socket.clone()).await?;

    tokio::try_join!(http_server, udp_server)?;
    Ok(())
}

/// Initialize HTTP server
fn init_http_server(
    state: Arc<ServerState>,
    weapons: Arc<WeaponDb>,
    config: Arc<Config>,
    udp_socket: Arc<UdpSocket>,
) -> tokio::task::JoinHandle<()> {
    let app_state = AppState {
        state,
        weapons,
        config,
        udp_socket,
    };
    
    let app = Router::new()
        .route("/lobbies", post(create_lobby))
        .route("/lobbies", get(list_lobbies))
        .route("/lobbies/:code/join", post(join_lobby))
        .route("/lobbies/:code", get(get_lobby))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let http_addr = format!("0.0.0.0:{}", 8080);
    info!("Starting HTTP server on {}", http_addr);

    tokio::spawn(async move {
        let listener = match TcpListener::bind(&http_addr).await {
            Ok(listener) => {
                info!("HTTP server successfully bound to {}", http_addr);
                listener
            }
            Err(e) => {
                eprintln!("Failed to bind HTTP server to {}: {}", http_addr, e);
                return;
            }
        };

        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("HTTP server error: {}", e);
        }
    })
}

/// Initialize UDP server
async fn init_udp_server(
    state: Arc<ServerState>,
    socket: Arc<UdpSocket>,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let socket_clone = socket.clone();
    let state_clone = state.clone();

    Ok(tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        loop {
            match socket_clone.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let data = &buf[..len];
                    if let Ok(packet) = serde_json::from_slice::<serde_json::Value>(data) {
                        handle_udp_packet(packet, addr, &state_clone).await;
                    }
                }
                Err(e) => {
                    log::error!("UDP recv error: {}", e);
                }
            }
        }
    }))
}

/// Create a new lobby and spawn its tick loop
pub async fn create_lobby_with_tick(
    state: Arc<ServerState>,
    code: String,
    max_players: u32,
    scene: String,
    weapons: Arc<WeaponDb>,
    config: Arc<Config>,
    socket: Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    if state.lobby_exists(&code) {
        return Err("Lobby already exists".into());
    }

    // Create lobby
    let lobby = Arc::new(RwLock::new(Lobby::new(code.clone(), max_players, scene.clone())));

    // Create command channel
    let (tx, rx) = mpsc::channel::<crate::state::commands::LobbyCommand>(1000);

    // Spawn tick loop
    let tick_weapons = weapons.clone();
    let tick_config = config.clone();
    let tick_socket = socket.clone();
    let tick_lobby = lobby.clone();
    let task_handle = tokio::spawn(async move {
        lobby_tick_loop(tick_lobby, rx, tick_socket, tick_weapons, tick_config).await;
    });

    // Create handle
    let handle = LobbyHandle {
        lobby,
        command_tx: tx,
        task_handle,
    };

    // Insert into state
    state.insert_lobby(code, handle);

    Ok(())
}
