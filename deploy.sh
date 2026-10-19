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

echo "Building release (Debian bookworm, glibc 2.36: runs on sakuin's older glibc)..."
# A host `cargo build` links against the deploying machine's glibc (Arch: 2.43),
# which sakuin's NixOS glibc cannot load.
python3 ../game-engine/scripts/desktop-server-build.py --root "$PWD" --release --build-host local

ssh "$SERVER_HOST" "mkdir -p ${STATE_DIR}/bin ${STATE_DIR}/data/economy ${GROUND_DIR}"

echo "Uploading binaries..."
for bin in game-server game-server-admin; do
	scp "target/release/${bin}" "${SERVER_HOST}:${STATE_DIR}/bin/${bin}.new"
	ssh "$SERVER_HOST" "mv ${STATE_DIR}/bin/${bin}.new ${STATE_DIR}/bin/${bin}"
done

echo "Uploading data (world.db and economy config are read relative to ${STATE_DIR})..."
scp data/world.db "${SERVER_HOST}:${STATE_DIR}/data/world.db"
test -f data/trading-post-monthly-catalog.json || { echo "Missing committed data/trading-post-monthly-catalog.json" >&2; exit 1; }
scp data/trading-post-monthly-catalog.json "${SERVER_HOST}:${STATE_DIR}/data/trading-post-monthly-catalog.json"
test -f data/game_object_locks.json || { echo "Missing data/game_object_locks.json: run scripts/import_game_object_locks.py" >&2; exit 1; }
scp data/game_object_locks.json "${SERVER_HOST}:${STATE_DIR}/data/game_object_locks.json"
scp data/economy/config.json "${SERVER_HOST}:${STATE_DIR}/data/economy/config.json"
# Startup panics without the generated Toy casting requirements (world_data.rs).
for toy_data in toy_required_areas.json toy_facing_requirements.json; do
	test -f "data/${toy_data}" || { echo "Missing data/${toy_data}: run scripts/generate_toy_area_requirements.py and scripts/generate_toy_facing_requirements.py" >&2; exit 1; }
	scp "data/${toy_data}" "${SERVER_HOST}:${STATE_DIR}/data/${toy_data}"
done
echo "Uploading installed-build DB2 tables (the server reads data/db2/active at startup)..."
# Missing here crash-looped production on 2026-10-14 (guild tabard catalog). Ship the whole
# active build so new runtime readers cannot be forgotten, and keep the active link.
db2_build=$(readlink data/db2/active)
test -n "$db2_build" -a -d "data/db2/${db2_build}" || { echo "data/db2/active must link to an installed build directory" >&2; exit 1; }
rsync -a --delete --chown=game-server:game-server "data/db2/${db2_build}/" "${SERVER_HOST}:${STATE_DIR}/data/db2/${db2_build}/"
ssh "$SERVER_HOST" "ln -sfn ${db2_build} ${STATE_DIR}/data/db2/active && chown -h game-server:game-server ${STATE_DIR}/data/db2/active"

echo "Uploading ground (WDTs, ADTs and WMOs of the spawned and hosted maps)..."
python3 scripts/ground_files.py --world-db data/world.db --cache ../game-engine/data > "$GROUND_LIST"
rsync -a --files-from="$GROUND_LIST" ../game-engine/data/ "${SERVER_HOST}:${GROUND_DIR}/"
echo "Uploading line of sight (los-bake output beside the ground; docs/specs/line-of-sight.md)..."
# The server refuses to start without the eye table; a map without tiles has no LOS geometry.
test -f ../game-engine/data/los/eye_heights.tsv || { echo "No LOS bake: run los-bake (docs/specs/line-of-sight.md)" >&2; exit 1; }
# Stockade supplement must be promoted into this cache before this --delete sync.
test -d ../game-engine/data/los/34 || { echo "Missing Stockade LOS: copy data/ground/los/34/ into ../game-engine/data/los/34/" >&2; exit 1; }
rsync -a --delete ../game-engine/data/los/ "${SERVER_HOST}:${GROUND_DIR}/los/"

echo "Uploading navigation (incremental nav-bake output)..."
test -d data/ground/navmesh || { echo "No navigation bake: run nav-bake (docs/specs/navigation.md)" >&2; exit 1; }
# Delete stale navigation artifacts only, never neighboring ground/LOS assets.
rsync -a --delete data/ground/navmesh/ "${SERVER_HOST}:${GROUND_DIR}/navmesh/"

ssh "$SERVER_HOST" "chown -R game-server:game-server ${STATE_DIR}"

echo "Restarting service..."
ssh "$SERVER_HOST" "systemctl restart game-server"

echo "Waiting for the first tick (startup loads world data for ~90 s)..."
# A crash-looping server used to leave this script exiting 0 (2026-10-14); fail unless it ticks without restarting.
ssh "$SERVER_HOST" 'since=$(systemctl show -p ActiveEnterTimestamp --value game-server)
for _ in $(seq 1 48); do
	sleep 5
	if [ "$(systemctl show -p NRestarts --value game-server)" != 0 ]; then echo "game-server restarted during startup" >&2; journalctl -u game-server -n 20 --no-pager >&2; exit 1; fi
	journalctl -u game-server --since "$since" --no-pager | grep -q "Tick " && { systemctl status game-server --no-pager | head -5; exit 0; }
done
echo "game-server did not tick within 240 s" >&2; journalctl -u game-server -n 20 --no-pager >&2; exit 1'
