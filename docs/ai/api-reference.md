# API Reference

Complete API documentation for GunGame's core systems and components.

## NetworkingManager

Singleton autoload handling all client-side networking.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `current_lobby` | `Dictionary` | Current lobby data (code, players, etc.) |
| `player_id` | `int` | Local player's server-assigned ID |
| `connected_to_udp` | `bool` | UDP connection status |
| `SERVER_URL` | `String` | HTTP server URL (default: "http://127.0.0.1:8080") |
| `UDP_PORT` | `int` | UDP server port (default: 8081) |

### Methods

#### Lobby Management
```gdscript
func create_lobby(code: String, scene: String = "world", max_players: int = 4) -> void
```
Creates a new lobby on the server.

**Parameters:**
- `code`: Unique lobby identifier
- `scene`: Scene name for the lobby
- `max_players`: Maximum players allowed

**Emits:** `lobby_created` or `lobby_join_failed`

---

```gdscript
func join_lobby(code: String) -> void
```
Joins an existing lobby.

**Parameters:**
- `code`: Lobby code to join

**Emits:** `lobby_joined` or `lobby_join_failed`

---

```gdscript
func connect_to_test_lobby() -> void
```
Automatically connects to the "test" lobby, creating it if it doesn't exist.

---

```gdscript
func leave_current_lobby() -> void
```
Disconnects from UDP game connection while keeping HTTP connection alive.

---

```gdscript
func disconnect_from_network() -> void
```
Fully disconnects from all networking (HTTP + UDP).

#### Real-time Communication
```gdscript
func send_position_update(position: Vector3, rotation: Vector3) -> void
```
Sends player position update to server.

**Parameters:**
- `position`: Player's 3D position
- `rotation`: Player's 3D rotation

---

```gdscript
func is_connected_to_lobby() -> bool
```
Returns true if connected to both HTTP and UDP.

### Signals

| Signal | Parameters | Description |
|--------|------------|-------------|
| `lobby_created` | `lobby_data: Dictionary` | New lobby successfully created |
| `lobby_joined` | `lobby_data: Dictionary` | Successfully joined a lobby |
| `lobby_join_failed` | `error: String` | Failed to join/create lobby |
| `lobby_list_received` | `lobby_list: Array` | Received list of available lobbies |
| `player_joined` | `player_data: Dictionary` | New player joined current lobby |
| `player_left` | `player_id: int` | Player disconnected from lobby |
| `position_update_received` | `player_id: int, position: Vector3, rotation: Vector3` | Position update from another player |
| `server_dummy_updated` | `position: Vector3` | Server dummy bot position update |
| `connection_confirmed` | - | UDP connection confirmed by server |

## InputManager

Singleton autoload for centralized input handling.

### Signals

```gdscript
signal input_changed(action: String, pressed: bool, strength: float)
```
Emitted when any input action changes.

**Parameters:**
- `action`: Input action name ("move_left", "attack", etc.)
- `pressed`: True if action is pressed
- `strength`: Input strength (0.0 to 1.0)

### Input Actions

| Action | Keys/Controllers | Description |
|--------|----------------|-------------|
| `move_left` | A / Left Stick Left | Move left |
| `move_right` | D / Left Stick Right | Move right |
| `move_up` | W / Left Stick Up | Move forward |
| `move_down` | S / Left Stick Down | Move backward |
| `look_left` | Mouse Left / Right Stick Left | Look left |
| `look_right` | Mouse Right / Right Stick Right | Look right |
| `look_up` | Mouse Up / Right Stick Up | Look up |
| `look_down` | Mouse Down / Right Stick Down | Look down |
| `accept` | Space / A Button | Accept/UI select |
| `escape` | Escape | Cancel/menu |
| `attack` | Left Click / Right Trigger | Primary attack |
| `shoot` | Left Click / Right Trigger | Shoot weapon |
| `reload` | R / X Button | Reload weapon |

## GameStateManager

Singleton autoload for game state management.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `current_state` | `String` | Current game state |

### States

| State | Description |
|-------|-------------|
| `"menu"` | Main menu active |
| `"playing"` | Game in progress |
| `"paused"` | Game paused |

### Signals

```gdscript
signal state_changed(new_state: String, old_state: String)
```
Emitted when game state changes.

## Player Entity

Main player character controller.

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `move_speed` | `float` | 5.0 | Movement speed in units/second |
| `jump_velocity` | `float` | 4.5 | Jump velocity |
| `mouse_sensitivity` | `float` | 0.001 | Mouse look sensitivity |
| `is_local` | `bool` | true | Whether this is the local player |

### Methods

```gdscript
func set_is_local(local: bool) -> void
```
Sets whether this player instance is controlled locally.

**Parameters:**
- `local`: True for local player, false for remote players

### Signals

```gdscript
signal health_changed(current: int, max: int)
```
Emitted when player health changes.

## Damageable Component

Health and damage handling system.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `current_health` | `int` | Current health points |
| `max_health` | `int` | Maximum health points |

### Methods

```gdscript
func take_damage(amount: int, attacker: Node = null) -> void
```
Applies damage to the entity.

**Parameters:**
- `amount`: Damage amount
- `attacker`: Node that caused the damage (optional)

---

```gdscript
func heal(amount: int) -> void
```
Restores health.

**Parameters:**
- `amount`: Healing amount

---

```gdscript
func is_alive() -> bool
```
Returns true if current_health > 0.

### Signals

```gdscript
signal died(attacker: Node)
signal health_changed(current: int, max: int)
signal damaged(amount: int, attacker: Node)
```

## Weapon Base

Base class for all weapons.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `damage` | `int` | Base damage per shot |
| `fire_rate` | `float` | Shots per second |
| `ammo_capacity` | `int` | Maximum ammo |
| `current_ammo` | `int` | Current ammo |

### Methods

```gdscript
func fire() -> bool
```
Attempts to fire the weapon.

**Returns:** True if shot fired successfully

---

```gdscript
func reload() -> void
```
Reloads the weapon to full capacity.

---

```gdscript
func can_fire() -> bool
```
Returns true if weapon can fire (has ammo, not on cooldown).

### Signals

```gdscript
signal fired()
signal reloaded()
signal ammo_changed(current: int, max: int)
```

## HTTP REST API

Server-side REST API endpoints.

### Lobbies

#### Create Lobby
```
POST /lobbies
```

**Request Body:**
```json
{
  "code": "string",
  "scene": "string",
  "max_players": 4
}
```

**Response:** `LobbyInfo` (200) or Error (409)

#### Join Lobby
```
POST /lobbies/{code}/join
```

**Request Body:**
```json
{
  "player_name": "string"
}
```

**Response:** `JoinLobbyResponse` (200) or Error (404/409)

#### Get Lobby
```
GET /lobbies/{code}
```

**Response:** `LobbyInfo` (200) or Error (404)

#### List Lobbies
```
GET /lobbies
```

**Response:** `Array<LobbyInfo>` (200)

### Data Types

#### LobbyInfo
```json
{
  "code": "string",
  "player_count": 2,
  "max_players": 4,
  "players": [{"id": 1, "name": "Player1"}],
  "server_ip": "127.0.0.1",
  "udp_port": 8081,
  "scene": "world"
}
```

#### JoinLobbyResponse
```json
{
  "lobby": LobbyInfo,
  "player_id": 1
}
```

#### PlayerInfo
```json
{
  "id": 1,
  "name": "Player1"
}
```

## UDP Game Protocol

Real-time game communication protocol.

### Client → Server Messages

#### Join Lobby
```json
{
  "type": "join",
  "lobby_code": "string",
  "player_id": 1
}
```

#### Position Update
```json
{
  "type": "position_update",
  "player_id": 1,
  "position": {"x": 1.0, "y": 2.0, "z": 3.0},
  "rotation": {"x": 0.0, "y": 1.57, "z": 0.0}
}
```

### Server → Client Messages

#### Welcome
```json
{
  "type": "welcome",
  "message": "Connected to lobby"
}
```

#### Position Update
```json
{
  "type": "position_update",
  "player_id": 2,
  "position": {"x": 5.0, "y": 1.0, "z": 0.0}
}
```

#### Player Joined
```json
{
  "type": "player_joined",
  "player": {"id": 2, "name": "Player2"}
}
```

#### Player Left
```json
{
  "type": "player_left",
  "player_id": 2
}
```

#### Server Dummy Update
```json
{
  "type": "server_dummy_update",
  "position": {"x": 3.0, "y": 1.0, "z": 0.0}
}
```

## Scene Structure

### Player.tscn
```
Player (CharacterBody3D)
├── CameraRig (Node3D)
│   └── Head (Node3D)
│       └── Camera3D
├── Damageable (Node)
└── HealthBar3D (Node3D)
```

### World.tscn (Test Scene)
```
World (Node3D)
├── LocalPlayer (Player)
├── RemotePlayers (Node3D)
├── ServerDummy (Node3D)
├── UI (CanvasLayer)
│   ├── ErrorLabel (Label)
│   └── ...
└── Environment (Node3D)
```

## Build System (just)

Task runner for development and deployment.

### Commands

| Command | Description |
|---------|-------------|
| `just` | Show available commands |
| `just start` | Build and run everything |
| `just web-build` | Export web client |
| `just server-build` | Build server container |
| `just server-run` | Run server container |
| `just clean` | Clean all builds |
| `just status` | Show project status |

### Configuration

Build settings in `scripts/justfile`:
- **Web Export**: Godot HTML5 preset
- **Server**: Podman container with Rust
- **Ports**: HTTP (8080), UDP (8081), Web (8000)
