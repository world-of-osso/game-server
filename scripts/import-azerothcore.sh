#!/usr/bin/env bash
# Import AzerothCore MySQL dumps into SQLite world.db.
#
# Uses hardcoded DDL (no MySQL→SQLite DDL parsing) + streaming INSERT extraction.
# INSERT data is filtered with awk (state machine), backtick-stripped, and
# MySQL \' escapes converted to SQLite '' before loading.
#
# Usage: ./scripts/import-azerothcore.sh [path/to/azerothcore/data/sql/base/db_world]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"
TMP_DIR="$PROJECT_DIR/data/import_tmp"

SQL_DIR="${1:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"

CREATURE_TEMPLATE_SQL="$SQL_DIR/creature_template.sql"
CREATURE_SQL="$SQL_DIR/creature.sql"
CREATURE_TEMPLATE_MODEL_SQL="$SQL_DIR/creature_template_model.sql"
WAYPOINT_DATA_SQL="$SQL_DIR/waypoint_data.sql"

for f in "$CREATURE_TEMPLATE_SQL" "$CREATURE_SQL" "$CREATURE_TEMPLATE_MODEL_SQL" "$WAYPOINT_DATA_SQL"; do
    if [[ ! -f "$f" ]]; then
        echo "ERROR: Missing SQL file: $f" >&2
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

# Extract INSERT data from a MySQL dump file.
# - State machine: emits lines between INSERT INTO and UNLOCK TABLES
# - Strips backticks from INSERT INTO header line
# - Skips /*!...*/ comment lines inside data blocks
# - Converts MySQL \' escape sequences to SQLite '' syntax
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

rmdir --ignore-fail-on-non-empty "$TMP_DIR"

echo ""
echo "Verifying row counts:"
sqlite3 "$DB_PATH" \
    "SELECT 'creature_template: ' || COUNT(*) FROM creature_template;" \
    "SELECT 'creature: ' || COUNT(*) FROM creature;" \
    "SELECT 'creature_template_model: ' || COUNT(*) FROM creature_template_model;" \
    "SELECT 'waypoint_data: ' || COUNT(*) FROM waypoint_data;"

echo "Database written to: $DB_PATH"
