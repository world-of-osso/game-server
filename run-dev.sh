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
# -x runs the command
cargo watch -c -w crates -w Cargo.toml -x 'run --bin server'
