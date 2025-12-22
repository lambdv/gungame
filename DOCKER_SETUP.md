# GunGame Docker Setup

This setup provides a complete containerized solution that builds and runs both the Rust server and Godot dedicated server using Docker and Docker Compose.

## Quick Start

Simply run:
```powershell
.\run.ps1 -Start
```

This will:
1. Build the Docker container with both servers
2. Start both the Rust server and Godot dedicated server
3. Expose all necessary ports

## Services

- **Rust Server**: HTTP API on port 8080, UDP game server on port 8081
- **Godot Dedicated Server**: Game server on port 4242

## Available Commands

```powershell
# Start the servers
.\run.ps1 -Start

# Check server status
.\run.ps1 -Status

# View logs
.\run.ps1 -Logs

# Stop servers
.\run.ps1 -Stop

# Restart servers
.\run.ps1 -Restart

# Clean up (stop and remove containers/images)
.\run.ps1 -Clean

# Build only
.\run.ps1 -Build
```

## Architecture

The setup uses:
- **Dockerfile.combined**: Multi-stage build that compiles both Rust and Godot servers
- **supervisord**: Process manager that runs both servers simultaneously
- **docker-compose.yml**: Docker Compose orchestration configuration
- **run.ps1**: PowerShell script for easy management

## Files Modified/Created

- `client/export_presets.cfg`: Added Linux dedicated server export preset
- `scripts/Dockerfile.combined`: New combined Dockerfile
- `scripts/supervisord.conf`: Supervisor configuration for managing both servers
- `docker-compose.yml`: Container orchestration
- `run.ps1`: PowerShell management script
- `DOCKER_SETUP.md`: This documentation

## Ports

- `8080/tcp`: Rust HTTP API (health checks, lobbies)
- `8081/udp`: Rust UDP game server
- `4242/tcp`: Godot dedicated server

## Development

For development, you can still use the individual build processes:
- Use `just web-build` for HTML5 export
- Use `just server-build` for Rust server only
- Use the new `run.ps1` for the full containerized setup
