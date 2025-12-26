# GunGame Documentation

Complete documentation for the minimalist multiplayer FPS game.

## Documentation Files

### Architecture & Design

- **[Architecture](./architecture.md)** - System design, components, and data flow
- **[Networking](./networking.md)** - Complete networking system documentation
- **[Multiplayer Setup](./multiplayer-setup.md)** - Multiplayer testing and setup guide

### API & Reference

- **[API Reference](./api-reference.md)** - Complete API documentation for all systems
- **[README](../README.md)** - Project overview and quick start guide

## Key Systems

### NetworkingManager
Single autoload handling all client networking:
- HTTP REST API for lobbies
- UDP real-time game synchronization
- Connection state management

### Server (Rust)
Dual-protocol game server:
- HTTP API (Axum) for lobby management
- UDP server (Tokio) for real-time gameplay
- Server-side dummy bot AI

### Build System
- **just**: Task runner for development workflow
- **Podman**: Containerized server deployment
- **Godot**: Web and desktop exports

## Quick Start

```bash
# Install dependencies
cargo install just  # Task runner

# Build and run everything
just start

# Open web client at http://localhost:8000
```

## Project Structure

```
gungame/
├── client/              # Godot game client
│   ├── shared/utils/    # Core autoloads (NetworkingManager, etc.)
│   ├── entites/         # Game entities (Player, Weapons)
│   └── ui/              # User interface
├── server/              # Rust server
│   └── gungameserver/   # HTTP + UDP server
├── docs/                # This documentation
└── scripts/             # Build automation
```

## Contributing

When modifying the codebase:
1. Follow **KISS principle** - keep implementations simple
2. Update relevant documentation
3. Test multiplayer functionality
4. Use signal-based communication for loose coupling

