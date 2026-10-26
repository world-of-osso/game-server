#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python3 scripts/format-spell-scripts.py --check
cargo test "$@"
cargo clippy --all-targets -- -D warnings
