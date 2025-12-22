# GunGame Server Runner
# This script builds and runs both the Rust server and Godot dedicated server in Docker

param(
    [switch]$Build,
    [switch]$Start,
    [switch]$Stop,
    [switch]$Restart,
    [switch]$Logs,
    [switch]$Status,
    [switch]$Clean
)

function Write-Header {
    Write-Host "GunGame Server Manager" -ForegroundColor Cyan
    Write-Host "======================" -ForegroundColor Cyan
    Write-Host ""
}

function Show-Help {
    Write-Host "Usage: .\run.ps1 [command]" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Commands:" -ForegroundColor Green
    Write-Host "  -Build     Build the server container"
    Write-Host "  -Start     Start the server (builds if needed)"
    Write-Host "  -Stop      Stop the server"
    Write-Host "  -Restart   Restart the server"
    Write-Host "  -Logs      Show server logs"
    Write-Host "  -Status    Show server status"
    Write-Host "  -Clean     Stop and remove containers/images"
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Magenta
    Write-Host "  .\run.ps1 -Start"
    Write-Host "  .\run.ps1 -Logs"
    Write-Host "  .\run.ps1 -Clean"
    Write-Host ""
}

function Invoke-DockerCommand {
    param([string]$Command, [string]$Description)

    Write-Host "Running $Description..." -ForegroundColor Yellow
    try {
        $output = Invoke-Expression "docker $Command" 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "$Description completed successfully" -ForegroundColor Green
            return $true
        } else {
            Write-Host "$Description failed" -ForegroundColor Red
            Write-Host $output -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

function Invoke-DockerCompose {
    param([string]$Command, [string]$Description)

    Write-Host "Running $Description..." -ForegroundColor Yellow
    try {
        # Try modern Docker Compose V2 first (docker compose), fall back to legacy (docker-compose)
        $dockerCommand = "docker compose $Command"
        $output = Invoke-Expression $dockerCommand 2>&1

        if ($LASTEXITCODE -ne 0) {
            # Fall back to legacy docker-compose command
            $dockerCommand = "docker-compose $Command"
            $output = Invoke-Expression $dockerCommand 2>&1
        }

        if ($LASTEXITCODE -eq 0) {
            Write-Host "$Description completed successfully" -ForegroundColor Green
            return $true
        } else {
            Write-Host "$Description failed" -ForegroundColor Red
            Write-Host $output -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Main logic
Write-Header

# Check if Docker is running
try {
    docker version > $null 2>&1
} catch {
    Write-Host "❌ Docker is not running or not installed. Please start Docker Desktop." -ForegroundColor Red
    exit 1
}

# Check if Docker and docker-compose are available
try {
    docker version > $null 2>&1
} catch {
    Write-Host "Docker is not available. Please install Docker Desktop." -ForegroundColor Red
    exit 1
}

# Check for Docker Compose (either V2 'docker compose' or legacy 'docker-compose')
$dockerComposeAvailable = $false
try {
    docker compose version > $null 2>&1
    $dockerComposeAvailable = $true
} catch {
    try {
        docker-compose version > $null 2>&1
        $dockerComposeAvailable = $true
    } catch {
        Write-Host "Docker Compose is not available. Please install Docker Compose." -ForegroundColor Red
        exit 1
    }
}

# Parse commands
$commandCount = 0
if ($Build) { $commandCount++ }
if ($Start) { $commandCount++ }
if ($Stop) { $commandCount++ }
if ($Restart) { $commandCount++ }
if ($Logs) { $commandCount++ }
if ($Status) { $commandCount++ }
if ($Clean) { $commandCount++ }

if ($commandCount -eq 0) {
    Show-Help
    exit 0
}

if ($commandCount -gt 1) {
    Write-Host "❌ Please specify only one command at a time." -ForegroundColor Red
    Show-Help
    exit 1
}

# Execute commands
if ($Build) {
    Invoke-DockerCompose "build" "Building GunGame server container"
}

if ($Start) {
    Write-Host "Starting GunGame server..." -ForegroundColor Green
    Write-Host "This will run both Rust server and Godot dedicated server" -ForegroundColor Cyan
    Write-Host ""

    if (Invoke-DockerCompose "up -d" "Starting GunGame server") {
        Write-Host ""
        Write-Host "GunGame Server is running!" -ForegroundColor Green
        Write-Host "==========================" -ForegroundColor Green
        Write-Host "Rust API Server:    http://localhost:8080" -ForegroundColor White
        Write-Host "UDP Game Server:    localhost:8081" -ForegroundColor White
        Write-Host "Godot Server:       localhost:4242" -ForegroundColor White
        Write-Host ""
        Write-Host "Use '.\run.ps1 -Logs' to view server logs" -ForegroundColor Cyan
        Write-Host "Use '.\run.ps1 -Stop' to stop the server" -ForegroundColor Cyan
    }
}

if ($Stop) {
    Invoke-DockerCompose "down" "Stopping GunGame server"
}

if ($Restart) {
    Write-Host "Restarting GunGame server..." -ForegroundColor Yellow
    if (Invoke-DockerCompose "restart" "Restarting GunGame server") {
        Write-Host "Server restarted successfully" -ForegroundColor Green
    }
}

if ($Logs) {
    Write-Host "Showing GunGame server logs (Ctrl+C to exit)..." -ForegroundColor Cyan
    Write-Host ""
    try {
        docker-compose logs -f
    } catch {
        Write-Host "Failed to show logs" -ForegroundColor Red
    }
}

if ($Status) {
    Write-Host "GunGame Server Status" -ForegroundColor Cyan
    Write-Host "=====================" -ForegroundColor Cyan
    Write-Host ""

    try {
        $containers = docker-compose ps 2>$null
        if ($containers) {
            Write-Host "Running Containers:" -ForegroundColor White
            Write-Host $containers
        } else {
            Write-Host "No containers running" -ForegroundColor Red
        }
    } catch {
        Write-Host "Unable to check container status" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "Service URLs:" -ForegroundColor Cyan
    Write-Host "  Rust API:    http://localhost:8080/lobbies"
    Write-Host "  UDP Server:  localhost:8081"
    Write-Host "  Godot Server: localhost:4242"
}

if ($Clean) {
    Write-Host "Cleaning up GunGame server..." -ForegroundColor Yellow
    Write-Host "This will stop containers and remove images" -ForegroundColor Yellow

    $confirm = Read-Host "Are you sure you want to clean everything? (y/N)"
    if ($confirm -eq "y" -or $confirm -eq "Y") {
        Invoke-DockerCompose "down" "Stopping containers"

        # Remove containers using the same Docker Compose method
        try {
            $rmCommand = "docker compose rm -f"
            $output = Invoke-Expression $rmCommand 2>&1
            if ($LASTEXITCODE -ne 0) {
                $rmCommand = "docker-compose rm -f"
                Invoke-Expression $rmCommand > $null 2>&1
            }
        } catch {
            # Ignore errors if containers don't exist
        }
        Write-Host "Removed stopped containers" -ForegroundColor Green

        try {
            docker rmi gungame_gungame-server > $null 2>&1
            Write-Host "Removed server image" -ForegroundColor Green
        } catch {
            Write-Host "No image to remove" -ForegroundColor Gray
        }

        Write-Host "Cleanup completed" -ForegroundColor Green
    } else {
        Write-Host "Cleanup cancelled" -ForegroundColor Gray
    }
}
