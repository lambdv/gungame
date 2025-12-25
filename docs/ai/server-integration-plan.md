# Server Integration Plan

## Overview
This plan outlines the integration of the Godot client with the existing Rust server to enable multiplayer functionality with lobby-based matchmaking.

## Architecture Overview

```
┌─────────────────┐    HTTP/REST    ┌─────────────────┐
│   Godot Client  │◄──────────────► │   Rust Server   │
│                 │                 │                 │
│ - HTTP Client   │    UDP          │ - HTTP Endpoints│
│ - UDP Client    │◄──────────────► │ - UDP Server    │
│ - Lobby System  │                 │ - Lobby Manager │
│ - Position Sync │                 │ - Game State    │
└─────────────────┘                 └─────────────────┘
```

## Phase 1: HTTP Communication Setup

### Godot HTTP Client
- Add HTTPRequest node for REST API communication
- Implement lobby creation endpoint (`POST /lobbies`)
- Implement lobby joining endpoint (`POST /lobbies/{code}/join`)
- Add error handling and retry logic

### Rust Server HTTP Endpoints
- `POST /lobbies` - Create new lobby with code
- `POST /lobbies/{code}/join` - Join existing lobby
- `GET /lobbies/{code}` - Get lobby info
- `DELETE /lobbies/{code}` - Cleanup empty lobbies

## Phase 2: UDP Real-time Communication

### Godot UDP Client
- Implement UDP socket for real-time communication
- Send position updates to server
- Receive position updates from other players
- Handle connection state management

### Rust UDP Server
- Add UDP server to handle real-time packets
- Process position updates from clients
- Broadcast position updates to lobby members
- Handle player join/leave events

## Phase 3: Lobby System Implementation

### Lobby Management
- **Client-side**: Lobby browser, join/create UI
- **Server-side**: Lobby state management, player tracking
- **Test Implementation**: Auto-connect to "TEST" lobby

### Server-side Dummy Player
- Mock player entity that exists only on server
- Sends periodic position updates
- Used for testing multiplayer functionality

## Phase 4: Position Synchronization

### Client Position Updates
- Send local player position via UDP
- Receive and interpolate remote player positions
- Handle prediction and reconciliation

### Server State Management
- Maintain authoritative player positions
- Broadcast updates to all lobby members
- Handle disconnections and cleanup

## Technical Requirements

### Godot Client Changes
- Add `NetworkingManager` autoload
- Modify `test/world.gd` to use networking
- Update player movement to sync with server
- Add lobby UI components

### Rust Server Changes
- Add HTTP endpoints for lobby management
- Implement UDP server for real-time communication
- Add lobby state management
- Create dummy player system

## Implementation Steps

### Step 1: Godot HTTP Client
```gdscript
# Add to NetworkingManager.gd
func create_lobby(code: String) -> Dictionary:
func join_lobby(code: String) -> Dictionary:
func get_lobby_info(code: String) -> Dictionary:
```

### Step 2: Godot UDP Client
```gdscript
# Add to NetworkingManager.gd
func connect_udp_server(ip: String, port: int) -> void:
func send_position(position: Vector3, rotation: Vector3) -> void:
func _process_udp_packets() -> void:
```

### Step 3: Rust HTTP Endpoints
```rust
// Add to main.rs or new module
async fn create_lobby() -> impl Responder
async fn join_lobby(code: String) -> impl Responder
async fn get_lobby(code: String) -> impl Responder
```

### Step 4: Rust UDP Server
```rust
// Add UDP server implementation
struct UdpServer {
    socket: UdpSocket,
    lobbies: HashMap<String, Lobby>,
}
```

### Step 5: Test Integration
```gdscript
# In test/world.gd
func _ready():
    NetworkingManager.connect_to_test_lobby()
    # Spawn local and remote players
```

## Data Structures

### Lobby
```rust
struct Lobby {
    code: String,
    players: HashMap<u32, Player>,
    dummy_player: Option<Player>,
    created_at: SystemTime,
}
```

### Player
```rust
struct Player {
    id: u32,
    position: Vec3,
    rotation: Vec3,
    last_update: SystemTime,
}
```

### Network Packets
```rust
enum Packet {
    PositionUpdate { player_id: u32, position: Vec3, rotation: Vec3 },
    PlayerJoined { player_id: u32 },
    PlayerLeft { player_id: u32 },
}
```

## Testing Strategy

1. **Unit Tests**: HTTP endpoints, UDP packet handling
2. **Integration Tests**: Full lobby creation/join flow
3. **Dummy Player Tests**: Verify server-side player simulation
4. **Position Sync Tests**: Verify client-server synchronization

## Security Considerations

- Input validation for lobby codes
- Rate limiting on HTTP endpoints
- UDP packet validation and size limits
- Timeout handling for inactive connections

## Performance Considerations

- UDP packet batching for position updates
- HTTP connection pooling
- Memory management for lobby state
- Garbage collection for disconnected players

## Deployment

- Environment-specific server endpoints
- Docker containerization for Rust server
- Load balancing for multiple server instances
- Monitoring and logging setup
