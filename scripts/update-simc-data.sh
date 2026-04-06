#!/usr/bin/env bash
# Update SimC spell data: pull latest SimC, re-run converter, apply overlay.
#
# Usage: ./scripts/update-simc-data.sh [simc_dir]
#
# Steps:
#   1. git pull in SimC repo
#   2. Run import-simc-spells.py to generate SQL overlay
#   3. Apply overlay to world.db (spell_bonus_data table)
#
# Prerequisites:
#   - SimC repo cloned at ~/Repos/simc (or pass path as argument)
#   - world.db exists at data/world.db (run import-azerothcore.sh first)
#   - Python 3 available
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"
OVERLAY_SQL="$PROJECT_DIR/data/simc_overlay.sql"

SIMC_DIR="${1:-$HOME/Repos/simc}"

if [[ ! -d "$SIMC_DIR/.git" ]]; then
    echo "ERROR: SimC repo not found at $SIMC_DIR" >&2
    echo "Clone it: git clone https://github.com/simulationcraft/simc.git $SIMC_DIR" >&2
    exit 1
fi

if [[ ! -f "$DB_PATH" ]]; then
    echo "ERROR: world.db not found at $DB_PATH" >&2
    echo "Run ./scripts/import-azerothcore.sh first" >&2
    exit 1
fi

# Step 1: Update SimC
echo "==> Updating SimC repo at $SIMC_DIR ..."
(cd "$SIMC_DIR" && git pull --ff-only)
SIMC_COMMIT=$(cd "$SIMC_DIR" && git rev-parse --short HEAD)
echo "    SimC at commit: $SIMC_COMMIT"

# Step 2: Generate SQL overlay
echo "==> Running SimC spell converter ..."
python3 "$SCRIPT_DIR/import-simc-spells.py" "$SIMC_DIR/engine/dbc/generated" > "$OVERLAY_SQL" 2>/dev/null
SPELL_COUNT=$(grep -c "^INSERT" "$OVERLAY_SQL" || echo 0)
echo "    Generated $SPELL_COUNT spell coefficient entries"

# Step 3: Apply to world.db
echo "==> Applying overlay to $DB_PATH ..."
sqlite3 "$DB_PATH" ".read $OVERLAY_SQL"

# Verify
TOTAL=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM spell_bonus_data;")
echo ""
echo "Done. spell_bonus_data now has $TOTAL entries."
echo "SimC commit: $SIMC_COMMIT"
echo "Overlay SQL saved to: $OVERLAY_SQL"
