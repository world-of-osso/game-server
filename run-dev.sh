#!/bin/bash

set -euo pipefail

cleanup() {
    exit 0
}

trap cleanup SIGINT SIGTERM

# Watch for changes using cargo-watch.
# The shell command loops so the server is restarted after crashes/exits.
# On source changes, cargo-watch kills the running loop and starts a fresh one.
cargo watch -w crates -w Cargo.toml -s '
while true; do
    cargo run --bin game-server
    code=$?
    echo ""
    echo "Server exited with code $code. Restarting in 20s..."
    sleep 20
done
'
