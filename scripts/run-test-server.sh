#!/bin/bash
# GunGame Test Server Runner for Linux/Mac
echo "🧪 GunGame Test Server"
echo "====================="
echo ""
echo "Starting Rust server with automatic test lobby creation..."
echo ""

cd server/gungameserver
cargo run
cd ../..
