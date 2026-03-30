#!/usr/bin/env bash
set -euo pipefail

SERVER_HOST="sakuin"
SERVER_DIR="/opt/game-server"
TERRAIN_DIR="/opt/game-engine/data/terrain"

echo "Building release..."
cargo build --release -p server

echo "Uploading binary..."
scp target/release/game-server "${SERVER_HOST}:${SERVER_DIR}/server.new"
ssh "$SERVER_HOST" "mv ${SERVER_DIR}/server.new ${SERVER_DIR}/server"

echo "Uploading data..."
scp data/world.db "${SERVER_HOST}:${SERVER_DIR}/data/world.db"
scp ../game-engine/data/terrain/azeroth_32_48.adt "${SERVER_HOST}:${TERRAIN_DIR}/azeroth_32_48.adt"

echo "Restarting service..."
ssh "$SERVER_HOST" "systemctl restart game-server"

echo "Waiting for startup..."
sleep 2
ssh "$SERVER_HOST" "systemctl status game-server --no-pager -l"
