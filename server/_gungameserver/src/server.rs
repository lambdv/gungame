use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use log::info;
use tokio::net::{TcpListener, UdpSocket};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use std::collections::HashMap;
use crate::handlers::lobby::Lobby;
use crate::handlers::http::{create_lobby, list_lobbies, join_lobby, get_lobby};
use crate::handlers::networking::handle_udp_packet;

type LobbyCode = String;

pub async fn start_servers(game_server: Arc<RwLock<ServerState>>) -> Result<(), Box<dyn std::error::Error>> {
    let http_server = _init_http_server(game_server.clone());
    let udp_server = _init_udp_server(game_server.clone()).await;

    tokio::try_join!(http_server, udp_server)?;
    Ok(())
}

fn _init_http_server(game_server: AppState) -> JoinHandle<()> {
    let app = Router::new()
        .route("/lobbies", post(create_lobby))
        .route("/lobbies", get(list_lobbies))
        .route("/lobbies/:code/join", post(join_lobby))
        .route("/lobbies/:code", get(get_lobby))
        .layer(CorsLayer::permissive())
        .with_state(game_server);

    let http_addr = "0.0.0.0:8080";
    info!("Starting HTTP server on {}", http_addr);

    tokio::spawn(async move {
        let listener = TcpListener::bind(http_addr).await;
        match listener {
            Ok(listener) => {
                info!("HTTP server successfully bound to {}", http_addr);
                if let Err(e) = axum::serve(listener, app).await {
                    eprintln!("HTTP server error: {}", e);
                }
            }
            Err(e) => panic!("Failed to bind HTTP: {}", e),
        }
    })
}

async fn _init_udp_server(game_server: Arc<RwLock<ServerState>>) -> JoinHandle<()> {
    let udp_addr = "0.0.0.0:8081";
    info!("Starting UDP server on {}", udp_addr);

    let socket = UdpSocket::bind(udp_addr).await.expect("Failed to bind UDP");
    let socket = Arc::new(socket);
    info!("UDP server successfully bound to {}", udp_addr);

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        loop {
            let (len, addr) = socket.recv_from(&mut buf).await?;
            let data = &buf[..len];
    
            if let Ok(packet) = serde_json::from_slice::<serde_json::Value>(data) {
                handle_udp_packet(packet, addr, &socket, &game_server).await;
            }
        }
    })
}

