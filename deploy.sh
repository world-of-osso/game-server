#!/usr/bin/env bash
set -euo pipefail

# The game-server unit, its env and UDP 5000 are declared in the sakuin repo
# (ops/nixos/game-server.nix); this script only ships the binary and data.
SERVER_HOST="sakuin"
STATE_DIR="/var/lib/game-server"
# Client asset cache the ground loads from (GAME_SERVER_GROUND_DIR, crates/server/src/ground.rs).
GROUND_DIR="${STATE_DIR}/ground"
GROUND_LIST="$(mktemp)"
trap 'rm -f "$GROUND_LIST"' EXIT

echo "Building release..."
cargo build --release -p server

ssh "$SERVER_HOST" "mkdir -p ${STATE_DIR}/bin ${STATE_DIR}/data/economy ${GROUND_DIR}"

echo "Uploading binaries..."
for bin in game-server game-server-admin; do
	scp "target/release/${bin}" "${SERVER_HOST}:${STATE_DIR}/bin/${bin}.new"
	ssh "$SERVER_HOST" "mv ${STATE_DIR}/bin/${bin}.new ${STATE_DIR}/bin/${bin}"
done

echo "Uploading data (world.db and economy config are read relative to ${STATE_DIR})..."
scp data/world.db "${SERVER_HOST}:${STATE_DIR}/data/world.db"
scp data/economy/config.json "${SERVER_HOST}:${STATE_DIR}/data/economy/config.json"

echo "Uploading ground (WDTs, ADTs and WMOs of the spawned maps)..."
python3 scripts/ground_files.py --world-db data/world.db --cache ../game-engine/data > "$GROUND_LIST"
rsync -a --files-from="$GROUND_LIST" ../game-engine/data/ "${SERVER_HOST}:${GROUND_DIR}/"
echo "Uploading line of sight (los-bake output beside the ground; docs/specs/line-of-sight.md)..."
# The server refuses to start without the eye table; a map without tiles has no LOS geometry.
test -f ../game-engine/data/los/eye_heights.tsv || { echo "No LOS bake: run los-bake (docs/specs/line-of-sight.md)" >&2; exit 1; }
rsync -a --delete ../game-engine/data/los/ "${SERVER_HOST}:${GROUND_DIR}/los/"

ssh "$SERVER_HOST" "chown -R game-server:game-server ${STATE_DIR}"

echo "Restarting service..."
ssh "$SERVER_HOST" "systemctl restart game-server"

echo "Waiting for startup..."
sleep 2
ssh "$SERVER_HOST" "systemctl status game-server --no-pager -l"
