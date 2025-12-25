# GunGame - Minimalist Multiplayer FPS

A clean, minimalist 3D first-person shooter built with Godot Engine 4.4. Features real-time multiplayer with a simple lobby system and server-side bot AI.

## Features

- **Minimal Architecture**: Single networking manager handles all multiplayer communication
- **Real-time Multiplayer**: HTTP lobby system + UDP game synchronization
- **Server-side AI**: Dummy bot that moves in circles for testing
- **Web Export**: Runs in browsers via WebGL
- **Cross-platform**: Windows, Linux, macOS, and web support

## Quick Start

### Prerequisites
- [Godot Engine 4.4+](https://godotengine.org/download)
- [Rust](https://rustup.rs/) (for server)
- [Podman/Docker](https://podman.io/) (for containerized server)

### Running Everything

```bash
# Install just (task runner)
cargo install just

# Build and run server + client
just start

# Open web client
# Visit: http://localhost:8000
```

### Manual Setup

1. **Start Server**:
   ```bash
   cd server/gungameserver
   cargo run
   ```

2. **Export Web Client**:
   ```bash
   cd client
   godot --export-release "HTML5" "html5/index.html"
   ```

3. **Serve Web Client**:
   ```bash
   cd client/html5
   python -m http.server 8000
   ```

## Architecture

### Client (`client/`)
- **NetworkingManager**: Singleton handling HTTP lobbies + UDP game sync
- **InputManager**: Centralized input handling with signal-based communication
- **GameStateManager**: Game state coordination
- **Player**: Character controller with health/damage system

### Server (`server/gungameserver/`)
- **HTTP API**: Lobby creation/joining (Axum web framework)
- **UDP Game Server**: Real-time position sync and player management
- **Dummy Bot**: Server-side AI for testing

### Key Design Principles
- **Single Responsibility**: Each component has one clear purpose
- **Signal-based Communication**: Loose coupling between systems
- **Minimal Code**: No unnecessary abstractions or redundant systems
- **Testable**: Clear separation of concerns

## Networking Flow

1. **Lobby Management** (HTTP):
   - Create/join lobbies via REST API
   - Server assigns player IDs and UDP ports

2. **Game Synchronization** (UDP):
   - Real-time position updates (10/s)
   - Player join/leave notifications
   - Server dummy position broadcasts

## Development

### Project Structure
```
gungame/
├── client/              # Godot game client
│   ├── shared/utils/    # Core utilities (NetworkingManager, etc.)
│   ├── entites/         # Game entities (Player, Weapons)
│   ├── ui/              # User interface scenes
│   └── test/            # Test scenes
├── server/              # Rust game server
│   └── gungameserver/   # Main server application
├── docs/                # Documentation
└── scripts/             # Build automation (justfile)
```

### Testing Multiplayer
```bash
# Run test world scene
godot --path client client/test/world/World.tscn
```

### Building for Production
```bash
# Web build
just web-build

# Server container
just server-build
```

## API Reference

### NetworkingManager Signals
- `lobby_created(lobby_data: Dictionary)`
- `lobby_joined(lobby_data: Dictionary)`
- `player_joined(player_data: Dictionary)`
- `position_update_received(player_id: int, position: Vector3, rotation: Vector3)`

### HTTP API Endpoints
- `POST /lobbies` - Create lobby
- `POST /lobbies/{code}/join` - Join lobby
- `GET /lobbies/{code}` - Get lobby info
- `GET /lobbies` - List all lobbies

## Contributing

1. Follow the **Keep It Stupid Simple** rule - minimal code for maximum functionality
2. Use **single responsibility** principle for all components
3. Add tests for new features
4. Update documentation for any changes

## License

MIT License - see individual asset licenses for 3D models and textures.
