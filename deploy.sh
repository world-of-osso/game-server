#!/usr/bin/env bash
set -euo pipefail

SERVER_HOST="sakuin"
SERVER_DIR="/opt/game-server"
# Client asset cache the ground loads from (GAME_SERVER_GROUND_DIR, crates/server/src/ground.rs).
GROUND_DIR="/opt/game-engine/data"
GROUND_LIST="$(mktemp)"
trap 'rm -f "$GROUND_LIST"' EXIT

echo "Building release..."
cargo build --release -p server

echo "Uploading binary..."
scp target/release/game-server "${SERVER_HOST}:${SERVER_DIR}/server.new"
ssh "$SERVER_HOST" "mv ${SERVER_DIR}/server.new ${SERVER_DIR}/server"

echo "Uploading data..."
scp data/world.db "${SERVER_HOST}:${SERVER_DIR}/data/world.db"

echo "Uploading ground (WDTs, ADTs and WMOs of the spawned maps)..."
python3 scripts/ground_files.py --world-db data/world.db --cache ../game-engine/data > "$GROUND_LIST"
rsync -a --files-from="$GROUND_LIST" ../game-engine/data/ "${SERVER_HOST}:${GROUND_DIR}/"
echo "Uploading line of sight (los-bake output beside the ground; docs/specs/line-of-sight.md)..."
# The server refuses to start without the eye table; a map without tiles has no LOS geometry.
test -f ../game-engine/data/los/eye_heights.tsv || { echo "No LOS bake: run los-bake (docs/specs/line-of-sight.md)" >&2; exit 1; }
rsync -a --delete ../game-engine/data/los/ "${SERVER_HOST}:${GROUND_DIR}/los/"
# The binary's default ground path is this checkout's; without the variable it refuses to start.
ssh "$SERVER_HOST" "mkdir -p /etc/systemd/system/game-server.service.d && printf '[Service]\nEnvironment=GAME_SERVER_GROUND_DIR=${GROUND_DIR}\n' > /etc/systemd/system/game-server.service.d/ground.conf && systemctl daemon-reload"

echo "Restarting service..."
ssh "$SERVER_HOST" "systemctl restart game-server"

echo "Waiting for startup..."
sleep 2
ssh "$SERVER_HOST" "systemctl status game-server --no-pager -l"
