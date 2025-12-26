# Networking System

GunGame uses a streamlined networking architecture combining HTTP for lobby management and UDP for real-time gameplay.

## Overview

The system consolidates all networking into a single `NetworkingManager` autoload, eliminating the complexity of multiple networking systems.

## NetworkingManager

### Purpose
Handles all client-side networking:
- HTTP REST API for lobbies
- UDP real-time game communication
- Connection state management
- Player and lobby state tracking

### Key Properties
```gdscript
const SERVER_URL = "http://127.0.0.1:8080"
const UDP_PORT = 8081

var current_lobby: Dictionary
var player_id: int
var connected_to_udp: bool
```

### HTTP API Methods

#### Lobby Management
```gdscript
func create_lobby(code: String, scene: String = "world", max_players: int = 4) -> void
func join_lobby(code: String) -> void
func get_lobby_info(code: String) -> void
func get_lobby_list() -> void
```

#### Connection Management
```gdscript
func connect_to_test_lobby() -> void  # Auto-connect to "test" lobby
func leave_current_lobby() -> void     # Disconnect UDP, keep HTTP
func disconnect_from_network() -> void # Full disconnect
```

### UDP Methods

#### Position Synchronization
```gdscript
func send_position_update(position: Vector3, rotation: Vector3) -> void
```

#### Connection Handling
```gdscript
func connect_udp_server(ip: String, port: int) -> void
func send_join_packet() -> void
```

### Signals

#### Lobby Events
```gdscript
signal lobby_created(lobby_data: Dictionary)
signal lobby_joined(lobby_data: Dictionary)
signal lobby_join_failed(error: String)
```

#### Player Events
```gdscript
signal player_joined(player_data: Dictionary)
signal player_left(player_id: int)
signal position_update_received(player_id: int, position: Vector3, rotation: Vector3)
```

#### Connection Events
```gdscript
signal connection_confirmed()
```

## Server Implementation (Rust)

### HTTP Server (Axum)
- **Port**: 8080
- **Framework**: Axum web framework
- **State**: Shared `RwLock<GameServer>` for thread safety

### UDP Server (Tokio)
- **Port**: 8081
- **Protocol**: JSON messages over UDP
- **Features**: Real-time position sync, player management

### Server State Structure
```rust
struct GameServer {
    lobbies: HashMap<LobbyCode, Lobby>,
    next_player_id: u32,
}

struct Lobby {
    code: String,
    players: HashMap<u32, Player>,
    max_players: u32,
    dummy_player: Option<Player>,
    client_addresses: HashMap<u32, SocketAddr>,
}
```

## Network Protocols

### HTTP REST API

#### Create Lobby
```http
POST /lobbies
Content-Type: application/json

{
  "code": "mylobby",
  "scene": "world",
  "max_players": 4
}
```

#### Join Lobby
```http
POST /lobbies/{code}/join
Content-Type: application/json

{
  "player_name": "Player1"
}
```

#### Get Lobby Info
```http
GET /lobbies/{code}
```

### UDP Game Protocol

#### Client Messages
```json
{"type": "join", "lobby_code": "test", "player_id": 1}
{"type": "position_update", "player_id": 1, "position": {"x": 1.0, "y": 2.0, "z": 3.0}}
```

#### Server Messages
```json
{"type": "welcome", "message": "Connected to lobby"}
{"type": "position_update", "player_id": 2, "position": {"x": 5.0, "y": 1.0, "z": 0.0}}
{"type": "player_joined", "player": {"id": 2, "name": "Player2"}}
```

## Connection Flow

### Joining a Game
1. **HTTP Lobby Join**: Client sends join request to server
2. **Server Response**: Server returns lobby info with UDP port
3. **UDP Connection**: Client connects to UDP server
4. **Join Packet**: Client sends join confirmation via UDP
5. **Welcome Message**: Server acknowledges connection
6. **Game Start**: Real-time position updates begin

### Position Synchronization
- **Frequency**: 10 updates per second
- **Format**: JSON with position/rotation vectors
- **Broadcast**: Server relays updates to all lobby members
- **Interpolation**: Client-side smoothing (not implemented yet)

## Server Features

### Dummy Bot System
- **Purpose**: AI player for testing multiplayer
- **Movement**: Circular path around center
- **Updates**: 100ms intervals
- **Broadcast**: Position sent to all lobby clients

### Player Management
- **ID Assignment**: Server assigns unique player IDs
- **State Tracking**: Position, name, connection status
- **Cleanup**: Automatic removal on disconnect

## Error Handling

### Connection Failures
- **Timeout**: 10-second connection timeout in test scene
- **Retry Logic**: Automatic fallback to lobby creation
- **User Feedback**: Error messages displayed in UI

### Network Disruptions
- **Reconnection**: Not implemented (future feature)
- **State Recovery**: Clean disconnect preserves HTTP connection
- **Graceful Degradation**: UI indicates connection status

## Performance Optimization

### Bandwidth Usage
- **Position Updates**: ~50 bytes per update × 10 Hz × players
- **Lobby Management**: Minimal HTTP traffic
- **Binary Protocol**: Future optimization opportunity

### Server Scalability
- **Async Processing**: Tokio handles concurrent connections
- **State Locking**: RwLock allows concurrent reads
- **Resource Limits**: Configurable max players per lobby

## Testing

### Local Development
```bash
# Start server
cargo run

# Test HTTP API
curl http://localhost:8080/lobbies

# Run client test scene
godot --path client client/test/world/World.tscn
```

### Multiplayer Testing
- Open multiple browser tabs to test lobby system
- Use different devices for realistic testing
- Monitor server logs for connection events

## Future Improvements

### Planned Features
- **WebRTC**: Browser-to-browser direct connections
- **Matchmaking**: Automated lobby assignment
- **Voice Chat**: Real-time audio communication
- **Anti-Cheat**: Server-side validation

### Protocol Enhancements
- **Binary Protocol**: Replace JSON with binary format
- **Compression**: Reduce bandwidth usage
- **Reliable UDP**: Custom reliability layer
