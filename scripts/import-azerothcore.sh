#!/usr/bin/env bash
# Import AzerothCore MySQL dumps + retail spell CSVs into SQLite world.db.
#
# Creature data: AzerothCore MySQL dump → awk extraction → SQLite INSERT
# Spell/class data: build-pinned Retail DB2 CSVs (game-engine/data/db2/<build>/) → db2.py import
#
# Usage: ./scripts/import-azerothcore.sh [path/to/azerothcore/data/sql/base/db_world]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"
TMP_DIR="$PROJECT_DIR/data/import_tmp"

SQL_DIR="${1:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"
DB2_CSV_ROOT="${DB2_CSV_ROOT:-$PROJECT_DIR/../game-engine/data/db2}"

CREATURE_TEMPLATE_SQL="$SQL_DIR/creature_template.sql"
CREATURE_SQL="$SQL_DIR/creature.sql"
CREATURE_TEMPLATE_MODEL_SQL="$SQL_DIR/creature_template_model.sql"
WAYPOINT_DATA_SQL="$SQL_DIR/waypoint_data.sql"
GAME_EVENT_CREATURE_SQL="$SQL_DIR/game_event_creature.sql"

for f in "$CREATURE_TEMPLATE_SQL" "$CREATURE_SQL" "$CREATURE_TEMPLATE_MODEL_SQL" "$WAYPOINT_DATA_SQL" "$GAME_EVENT_CREATURE_SQL"; do
    if [[ ! -f "$f" ]]; then
        echo "ERROR: Missing SQL file: $f" >&2
        exit 1
    fi
done

python3 "$SCRIPT_DIR/db2.py" check --csv-root "$DB2_CSV_ROOT"

rm -f "$DB_PATH" "$DB_PATH-wal" "$DB_PATH-shm"
mkdir -p "$TMP_DIR"
echo "Creating $DB_PATH ..."

# Apply hardcoded SQLite DDL schema
SCHEMA_SQL="$SCRIPT_DIR/world_schema.sql"
if [[ ! -f "$SCHEMA_SQL" ]]; then
    echo "ERROR: Missing schema file: $SCHEMA_SQL" >&2
    exit 1
fi
sqlite3 "$DB_PATH" ".read $SCHEMA_SQL"

# --- AzerothCore MySQL dump import (creatures/waypoints) ---

extract_inserts() {
    local sql_file="$1"
    local out_file="$2"
    awk '
    /^INSERT INTO/ {
        gsub(/`/, "")
        print
        in_data = 1
        next
    }
    in_data == 1 && /^UNLOCK TABLES/ {
        in_data = 0
        next
    }
    in_data == 1 && /^\/\*/ { next }
    in_data == 1 { print }
    ' "$sql_file" | sed "s/\\\\'"/"''"/g > "$out_file"
}

import_table() {
    local sql_file="$1"
    local table_name="$2"
    local tmp_sql="$TMP_DIR/${table_name}.sql"
    echo "  Extracting $table_name ..."
    extract_inserts "$sql_file" "$tmp_sql"
    echo "  Importing $table_name ($(wc -l < "$tmp_sql") lines) ..."
    sqlite3 "$DB_PATH" ".read $tmp_sql"
    echo "  Done: $table_name"
    rm -f "$tmp_sql"
}

import_table "$CREATURE_TEMPLATE_SQL"       "creature_template"
import_table "$CREATURE_SQL"                "creature"
import_table "$CREATURE_TEMPLATE_MODEL_SQL" "creature_template_model"
import_table "$WAYPOINT_DATA_SQL"           "waypoint_data"
import_table "$GAME_EVENT_CREATURE_SQL"     "game_event_creature"

# --- Retail DB2 CSV import (build-pinned, see db2.py) ---

echo "  Importing DB2 tables ..."
python3 "$SCRIPT_DIR/db2.py" import --csv-root "$DB2_CSV_ROOT" --db "$DB_PATH"

rmdir --ignore-fail-on-non-empty "$TMP_DIR"

echo ""
echo "Verifying row counts:"
sqlite3 "$DB_PATH" \
    "SELECT 'creature_template: ' || COUNT(*) FROM creature_template;" \
    "SELECT 'creature: ' || COUNT(*) FROM creature;" \
    "SELECT 'game_event_creature: ' || COUNT(*) FROM game_event_creature;" \
    "SELECT 'creature_template_model: ' || COUNT(*) FROM creature_template_model;" \
    "SELECT 'waypoint_data: ' || COUNT(*) FROM waypoint_data;"

echo "Database written to: $DB_PATH"
