#!/usr/bin/env bash
# Import AzerothCore MySQL dumps + retail spell CSVs into SQLite world.db.
#
# Creature data: AzerothCore MySQL dump → awk extraction → SQLite INSERT
# Spell data: Retail DB2 CSVs (from game-engine/data/) → sqlite3 .import
#
# Usage: ./scripts/import-azerothcore.sh [path/to/azerothcore/data/sql/base/db_world]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"
TMP_DIR="$PROJECT_DIR/data/import_tmp"

SQL_DIR="${1:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"
SPELL_CSV_DIR="${SPELL_CSV_DIR:-$PROJECT_DIR/../game-engine/data}"

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

SPELL_CSVS=(SpellName Spell SpellMisc SpellEffect SpellPower SpellCastTimes SpellRange SpellCooldowns SpellDuration)
SPELL_TABLES=(spell_name spell spell_misc spell_effect spell_power spell_cast_times spell_range spell_cooldowns spell_duration)
for csv in "${SPELL_CSVS[@]}"; do
    if [[ ! -f "$SPELL_CSV_DIR/$csv.csv" ]]; then
        echo "ERROR: Missing CSV file: $SPELL_CSV_DIR/$csv.csv" >&2
        exit 1
    fi
done

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

# --- Retail spell CSV import ---

import_spell_csv() {
    local csv_file="$1"
    local table_name="$2"
    echo "  Importing $table_name from $(basename "$csv_file") ..."
    sqlite3 "$DB_PATH" <<EOF
.mode csv
.import --skip 1 $csv_file $table_name
EOF
    echo "  Done: $table_name"
}

for i in "${!SPELL_CSVS[@]}"; do
    import_spell_csv "$SPELL_CSV_DIR/${SPELL_CSVS[$i]}.csv" "${SPELL_TABLES[$i]}"
done

rmdir --ignore-fail-on-non-empty "$TMP_DIR"

echo ""
echo "Verifying row counts:"
sqlite3 "$DB_PATH" \
    "SELECT 'creature_template: ' || COUNT(*) FROM creature_template;" \
    "SELECT 'creature: ' || COUNT(*) FROM creature;" \
    "SELECT 'game_event_creature: ' || COUNT(*) FROM game_event_creature;" \
    "SELECT 'creature_template_model: ' || COUNT(*) FROM creature_template_model;" \
    "SELECT 'waypoint_data: ' || COUNT(*) FROM waypoint_data;" \
    "SELECT 'spell_name: ' || COUNT(*) FROM spell_name;" \
    "SELECT 'spell: ' || COUNT(*) FROM spell;" \
    "SELECT 'spell_misc: ' || COUNT(*) FROM spell_misc;" \
    "SELECT 'spell_effect: ' || COUNT(*) FROM spell_effect;" \
    "SELECT 'spell_power: ' || COUNT(*) FROM spell_power;" \
    "SELECT 'spell_cast_times: ' || COUNT(*) FROM spell_cast_times;" \
    "SELECT 'spell_range: ' || COUNT(*) FROM spell_range;" \
    "SELECT 'spell_cooldowns: ' || COUNT(*) FROM spell_cooldowns;" \
    "SELECT 'spell_duration: ' || COUNT(*) FROM spell_duration;"

echo "Database written to: $DB_PATH"
