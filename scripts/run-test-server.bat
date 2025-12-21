@echo off
REM GunGame Test Server Runner for Windows
echo 🧪 GunGame Test Server
echo =====================
echo.
echo Starting Rust server with automatic test lobby creation...
echo.
cd server\gungameserver
cargo run
cd ..\..
