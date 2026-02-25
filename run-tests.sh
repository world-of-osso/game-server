#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo test "$@"
cargo clippy --all-targets -- -D warnings
