#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CHARACTER_NAME="${1:-Theron}"
SOCKET_PATH="/tmp/game-server-admin.sock"

if [[ ! -S "$SOCKET_PATH" ]]; then
    echo "ERROR: admin socket not found at $SOCKET_PATH" >&2
    echo "Start the game server first so game-server-admin can connect." >&2
    exit 1
fi

if [[ -x "$PROJECT_DIR/target/debug/game-server-admin" ]]; then
    ADMIN_CMD=("$PROJECT_DIR/target/debug/game-server-admin")
else
    ADMIN_CMD=(cargo run -p server --bin game-server-admin --)
fi

set_slot() {
    local slot="$1"
    local display_info_id="$2"
    "${ADMIN_CMD[@]}" set-equipment-appearance "$CHARACTER_NAME" "$slot" "$display_info_id"
}

echo "Restoring debugcharacter M2 appearance for '$CHARACTER_NAME'..."

set_slot head 685129
set_slot shoulder 148865
set_slot back 181925
set_slot chest 175942
set_slot hands 154616
set_slot waist 160997
set_slot legs 73783
set_slot feet 154620

echo "Restore complete for '$CHARACTER_NAME'."
