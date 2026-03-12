#!/bin/bash

set -e

# PID of the running server process
SERVER_PID=""

cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        echo "Stopping server (PID $SERVER_PID)..."
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGINT SIGTERM

# Watch for changes using cargo-watch
# -c clears screen on each restart
# -w watches directories
# -s runs a shell command (allows backoff on panic)
cargo watch -c -w crates -w Cargo.toml -s 'cargo run --bin server || { echo ""; echo "Server exited with code $?. Waiting 20s before restart..."; sleep 20; }'
