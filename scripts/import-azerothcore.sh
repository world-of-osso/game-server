#!/usr/bin/env bash
# Import AzerothCore MySQL dumps + retail spell CSVs into SQLite world.db.
#
# World content (creatures, quests, vendors, trainers, loot, gossip, items): AzerothCore
# MySQL dumps → mysql_dump.py → SQLite
# Spell/class/item/quest/taxi data: build-pinned Retail DB2 CSVs (game-engine/data/db2/<build>/) → db2.py import
# Server-read content_* tables: content_policy.py (Retail-first ID mapping)
#
# Usage: ./scripts/import-azerothcore.sh [path/to/azerothcore/data/sql/base/db_world]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"

SQL_DIR="${1:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"
DB2_CSV_ROOT="${DB2_CSV_ROOT:-$PROJECT_DIR/../game-engine/data/db2}"

# Tables loaded from same-named dumps in $SQL_DIR (<table>.sql); DDL in world_schema.sql.
DUMP_TABLES=(
    creature_template creature creature_template_model waypoint_data game_event_creature
    creature_addon creature_template_addon creature_formations
    quest_template quest_template_addon quest_offer_reward quest_request_items quest_poi quest_poi_points
    creature_queststarter creature_questender gameobject_queststarter gameobject_questender
    npc_vendor npc_trainer trainer trainer_spell creature_default_trainer
    creature_loot_template gameobject_loot_template reference_loot_template item_loot_template
    gossip_menu gossip_menu_option npc_text broadcast_text points_of_interest item_template
)

DUMP_ARGS=()
for table in "${DUMP_TABLES[@]}"; do
    f="$SQL_DIR/$table.sql"
    if [[ ! -f "$f" ]]; then
        echo "ERROR: Missing SQL file: $f" >&2
        exit 1
    fi
    DUMP_ARGS+=("$table=$f")
done

python3 "$SCRIPT_DIR/db2.py" check --csv-root "$DB2_CSV_ROOT"
python3 "$SCRIPT_DIR/content_policy.py" --check --csv-root "$DB2_CSV_ROOT"

rm -f "$DB_PATH" "$DB_PATH-wal" "$DB_PATH-shm"
echo "Creating $DB_PATH ..."

SCHEMA_SQL="$SCRIPT_DIR/world_schema.sql"
sqlite3 "$DB_PATH" "PRAGMA journal_mode=WAL;" > /dev/null

# --- AzerothCore MySQL dump import (DDL from world_schema.sql, same transaction) ---

echo "  Importing AzerothCore dumps ..."
python3 "$SCRIPT_DIR/mysql_dump.py" --db "$DB_PATH" --schema "$SCHEMA_SQL" "${DUMP_ARGS[@]}"

# --- Retail DB2 CSV import (build-pinned, see db2.py) ---

echo "  Importing DB2 tables ..."
python3 "$SCRIPT_DIR/db2.py" import --csv-root "$DB2_CSV_ROOT" --db "$DB_PATH"

# --- Retail-first content tables (content_*, see content_policy.py) ---

echo "  Deriving content tables ..."
python3 "$SCRIPT_DIR/content_policy.py" --db "$DB_PATH" --csv-root "$DB2_CSV_ROOT"


echo "Database written to: $DB_PATH"
