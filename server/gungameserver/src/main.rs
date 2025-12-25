mod handlers;
mod state;
mod domain;
mod tick;
mod utils;
mod server;

use fern;
use chrono;
use std::sync::Arc;
use crate::utils::weapondb::WeaponDb;
use crate::utils::config::Config;
use crate::state::server_state::ServerState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging()?;
    
    // Load immutable globals (zero contention)
    let weapons = Arc::new(WeaponDb::load());
    let config = Arc::new(Config::default());
    
    // Create server state (partitioned by lobby)
    let state = Arc::new(ServerState::new());
    
    // Create UDP socket for lobby tick loops
    let udp_socket = Arc::new(
        tokio::net::UdpSocket::bind(format!("0.0.0.0:{}", config.udp_port)).await?
    );
    
    // Create default test lobby
    server::create_lobby_with_tick(
        state.clone(),
        "test".to_string(),
        8,
        "test_world".to_string(),
        weapons.clone(),
        config.clone(),
        udp_socket.clone(),
    ).await?;
    
    log::info!("Created test lobby 'test'");
    
    // Start HTTP and UDP servers
    server::start_servers(state, weapons, config, udp_socket).await?;
    
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
