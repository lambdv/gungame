# Architecture Overview

GunGame follows a minimalist, signal-based architecture designed for maintainability and simplicity.

## Core Principles

### Keep It Stupid Simple (KISS)
- Minimal code for each implementation
- Single responsibility per component
- No unnecessary abstractions
- Clear, readable control flow

### Signal-Based Communication
- Loose coupling between systems
- Event-driven architecture
- Easy to test and debug
- Follows Godot's design patterns

## System Architecture

### Autoload Singletons
These run for the entire game lifetime and coordinate global systems:

#### NetworkingManager (`client/shared/utils/networking_manager.gd`)
**Purpose**: Unified networking for lobbies and real-time gameplay
- **HTTP Client**: Lobby creation/joining via REST API
- **UDP Client**: Real-time position synchronization
- **State Management**: Current lobby, player ID, connection status

**Key Methods**:
- `connect_to_test_lobby()` - Auto-connect to "test" lobby
- `send_position_update(position, rotation)` - Sync player position
- `create_lobby(code, scene, max_players)` - Create new lobby

#### InputManager (`client/shared/utils/input_manager.gd`)
**Purpose**: Centralized input handling
- **Signal Emission**: `input_changed(action, pressed, strength)`
- **Device Support**: Keyboard/mouse + gamepad
- **Deadzone Handling**: Configurable input filtering

#### GameStateManager (`client/shared/utils/game_state_manager.gd`)
**Purpose**: Game state coordination
- **State Transitions**: Menu ↔ Playing ↔ Paused
- **Signal Broadcasting**: State change notifications

### Entity System

#### Player (`client/entites/player/player.gd`)
**Purpose**: Player character controller
- **Movement**: WASD + mouse look
- **Health System**: Damageable component integration
- **Networking**: Position sync via NetworkingManager

**Key Components**:
- **CharacterBody3D**: Physics-based movement
- **CameraRig**: First-person camera system
- **Damageable**: Health and damage handling

#### Weapons (`client/entites/weapons/`)
**Purpose**: Weapon entities and behavior
- **Weapon Base**: Common weapon functionality
- **Individual Weapons**: Knife, shotgun, etc.
- **Scene-based**: Each weapon is a Godot scene

### UI System

#### Lobby System
- **LobbyList**: Display available lobbies
- **LobbyItem**: Individual lobby display
- **CreateLobbyDialog**: Lobby creation interface

#### HUD Elements
- **HealthBar3D**: 3D health display above players
- **Crosshair**: Center screen aiming indicator
- **PauseMenu**: Game pause and disconnect functionality

## Data Flow

### Multiplayer Game Flow
1. **Connection**: Player connects to test lobby via HTTP
2. **Join**: Server assigns player ID and UDP port
3. **Synchronization**: UDP connection established for real-time updates
4. **Gameplay**: Position updates sent 10x/second
5. **Disconnect**: Clean shutdown of HTTP + UDP connections

### Input Flow
1. **Raw Input**: Godot Input system captures device input
2. **Processing**: InputManager filters and normalizes input
3. **Signals**: Input changes broadcast to interested systems
4. **Response**: Player controller responds to input signals

## Networking Architecture

### Dual-Protocol Design
- **HTTP (REST)**: Lobby management, reliable operations
- **UDP**: Real-time gameplay, position sync

### Server Architecture (Rust)
- **HTTP Server**: Axum web framework for REST API
- **UDP Server**: Tokio async UDP handling
- **State Management**: Shared RwLock for thread-safe state
- **Dummy Bot**: Server-side AI for testing

### Client Architecture (Godot)
- **Single NetworkingManager**: Consolidates HTTP + UDP logic
- **Signal Integration**: Network events as Godot signals
- **State Synchronization**: Client maintains local state

## File Organization

```
client/
├── shared/utils/        # Global systems (autoloads)
├── entites/            # Game objects (player, weapons)
├── ui/                 # User interface
├── test/               # Test scenes
└── assets/             # Game assets (models, textures)
```

## Performance Considerations

### Minimal Overhead
- No redundant systems or managers
- Efficient signal connections
- Optimized network updates (10Hz position sync)

### Memory Management
- Automatic cleanup of disconnected players
- Scene-based instantiation for entities
- Proper node lifecycle management

## Testing Strategy

### Integration Testing
- **World Test Scene**: Full multiplayer simulation
- **Server Tests**: HTTP API validation
- **Client Tests**: Godot scene testing

### Manual Testing
- Multi-tab browser testing for multiplayer
- Network disconnection simulation
- Edge case validation

## Future Extensibility

The minimalist architecture supports easy extension:

- **New Weapons**: Add scenes to `entites/weapons/`
- **UI Elements**: Follow existing signal patterns
- **Network Features**: Extend NetworkingManager
- **Game Modes**: Add to GameStateManager states
