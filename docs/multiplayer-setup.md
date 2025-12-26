# GunGame Multiplayer Setup Guide

This guide explains how to set up and test the multiplayer functionality for the GunGame project.

## Overview

GunGame uses a client-server architecture with:
- **Rust server** handling game state, lobby management, and real-time communication
- **Godot client** for the game interface and player controls
- **HTTP API** for lobby management (port 8080)
- **UDP protocol** for real-time position updates (port 8081)

## Prerequisites

- **Rust** installed (`rustc --version` should work)
- **Godot 4.4.1** installed
- **Windows PowerShell** or Command Prompt

## Step 1: Start the Server

### Method 1: Using PowerShell (Recommended)
```powershell
# Navigate to the server directory
cd "C:\Users\Lokesh\Documents\projects\gungame\server\rust\gungameserver"

# Run the server
cargo run
```

### Method 2: Using Command Prompt
```cmd
cd C:\Users\Lokesh\Documents\projects\gungame\server\rust\gungameserver
cargo run
```

### Expected Output
```
HTTP server running on 0.0.0.0:8080
UDP server running on 0.0.0.0:8081
```

### Troubleshooting Server Issues

#### Port Conflict Error
If you see:
```
called `Result::unwrap()` on an `Err` value: Os { code: 10048, kind: AddrInUse, message: "Only one usage of each socket address (protocol/network address/port) is normally permitted." }
```

**Solution:**
```powershell
# Kill any existing server processes
taskkill /f /im gungameserver.exe

# Wait a few seconds, then restart
cargo run
```

#### Server Not Responding
Test the server:
```powershell
curl -X POST http://127.0.0.1:8080/lobbies -H "Content-Type: application/json" -d '{"code":"TEST","max_players":4}'
```

Expected response: `{"code":"TEST","player_count":0,"max_players":4,"players":[],"server_ip":"127.0.0.1","udp_port":8081}`

## Step 2: Run the Client

### Single Client Test
```powershell
& 'C:\Program Files\godot\Godot_v4.4.1-stable_win64_console.exe' --path 'C:\Users\Lokesh\Documents\projects\gungame\client' --scene 'res://test/world/World.tscn' --quit-after 10
```

### Full Game Client
```powershell
& 'C:\Program Files\godot\Godot_v4.4.1-stable_win64.exe' --path 'C:\Users\Lokesh\Documents\projects\gungame\client' --scene 'res://test/world/World.tscn'
```

### Expected Client Output
```
Attempting to connect to test lobby...
Test lobby exists, attempting to join...
Attempting to connect UDP to 127.0.0.1:8081
Connected to UDP server at 127.0.0.1:8081
Joined lobby successfully: TEST
Received welcome from server
```

## Step 3: Test Multiplayer

### Running Multiple Clients
1. Keep the server running in one terminal
2. Open multiple PowerShell/Command Prompt windows
3. Run the client command in each window:
   ```powershell
   & 'C:\Program Files\godot\Godot_v4.4.1-stable_win64.exe' --path 'C:\Users\Lokesh\Documents\projects\gungame\client' --scene 'res://test/world/World.tscn'
   ```

### What You Should See
- **Local Player**: Your character with first-person camera view
- **Remote Players**: Other players' characters that spawn automatically
- **Server Dummy**: Magenta cube that moves in a circle (server-side bot)
- **Real-time Sync**: Movement updates between all clients (10 updates/second)

## Architecture Details

### Server Components
- **HTTP Server (port 8080)**: REST API for lobby management
  - `POST /lobbies` - Create lobby
  - `POST /lobbies/{code}/join` - Join lobby
  - `GET /lobbies/{code}` - Get lobby info

- **UDP Server (port 8081)**: Real-time game communication
  - `join` - Player joins lobby
  - `position_update` - Position synchronization
  - `server_dummy_update` - Server bot updates

### Client Components
- **NetworkingManager**: Singleton handling all network communication
- **World Scene**: Game world with player spawning and management
- **Player Controller**: Local player with camera and movement
- **Position Sync**: Sends updates at 10Hz, receives from all other players

### Lobby System
- **TEST Lobby**: Default test lobby created automatically
- **Max 4 Players**: Configurable per lobby
- **Auto-spawn**: New players spawn with random positions near origin

## Common Issues & Solutions

### 1. "Connection timeout" Error
**Cause**: Server not running or network issues
**Solution**:
- Check server is running: `netstat -ano | findstr 8080`
- Restart server if needed
- Verify firewall isn't blocking ports 8080/8081

### 2. "Failed to connect to UDP server"
**Cause**: UDP connection failed
**Solution**:
- Ensure server is running and UDP port 8081 is available
- Check network connectivity
- Try restarting both server and client

### 3. Blank Screen / No Camera
**Cause**: Player camera not set as current
**Solution**: The code automatically sets the camera. If issue persists, check player scene structure.

### 4. Players Not Appearing
**Cause**: Position updates not being received
**Solution**:
- Check server logs for UDP traffic
- Verify clients are in the same lobby
- Check for network connectivity issues

### 5. Server Crashes on Startup
**Cause**: Port conflicts or system issues
**Solution**:
```powershell
# Kill conflicting processes
taskkill /f /im gungameserver.exe
# Check what's using ports
netstat -ano | findstr :8080
netstat -ano | findstr :8081
```

## Development Workflow

### Making Changes to Server
```bash
cd server/rust/gungameserver
cargo run  # Auto-recompiles on changes
```

### Making Changes to Client
- Edit files in Godot editor
- Test with: `F5` or run scene directly
- Check console output for networking logs

### Testing Protocol
1. **Server**: Start with `cargo run`
2. **Client 1**: Connect and verify lobby join
3. **Client 2**: Connect and verify player spawning
4. **Movement**: Move Client 1, verify Client 2 sees updates
5. **Server Bot**: Verify magenta cube movement

## Performance Notes

- **Position Updates**: Limited to 10Hz to prevent network spam
- **UDP Protocol**: Low-latency real-time communication
- **HTTP API**: RESTful lobby management
- **Threading**: HTTP requests use background threads in Godot

## Files Reference

### Server Files
- `server/rust/gungameserver/src/main.rs` - Main server logic
- `server/rust/gungameserver/Cargo.toml` - Rust dependencies

### Client Files
- `client/shared/utils/networking_manager.gd` - Network communication
- `client/test/world/world.gd` - Game world and player management
- `client/test/world/World.tscn` - Test scene
- `client/entites/player/Player.tscn` - Player prefab

## Next Steps

Once multiplayer is working, you can:
1. Add weapon systems
2. Implement game modes (deathmatch, capture the flag, etc.)
3. Add more server-side game logic
4. Implement proper matchmaking
5. Add spectator mode
6. Implement voice chat
7. Add anti-cheat measures

The foundation is now solid for building a full multiplayer game! 🎮



