#!/usr/bin/env bash
# Import AzerothCore MySQL dumps + retail spell CSVs into SQLite world.db.
#
# World content (creatures, quests, vendors, loot, gossip, items): AzerothCore
# MySQL dumps → mysql_dump.py → SQLite
# Spell/class/item/quest/taxi data: build-pinned Retail DB2 CSVs (game-engine/data/db2/<build>/) → db2.py import
# Retail profession trainers + skill tier caps, Retail creature models and the Retail
# creature spawns of maps 0/1, areatrigger teleports and graveyards: TrinityCore TDB1210 excerpts
# (data/reference/tdb1210/<table>.sql) → mysql_dump.py --prefix tdb_ → tdb_* tables
# Server-read content_* tables: content_policy.py (Retail-first ID mapping)
#
# Usage: FOREVER_DB2_SOURCE=/path/to/forever-db2 ./scripts/import-azerothcore.sh [path/to/azerothcore/data/sql/base/db_world]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DB_PATH="$PROJECT_DIR/data/world.db"
: "${FOREVER_DB2_SOURCE:?Set FOREVER_DB2_SOURCE to the untracked Forever DB2 directory}"

SQL_DIR="${1:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"
DB2_CSV_ROOT="${DB2_CSV_ROOT:-$PROJECT_DIR/../game-engine/data/db2}"
# TrinityCore Retail world DB excerpts: trainer tables and skill_tiers for profession trainers,
# creature tables for Retail displays and the Retail spawns of maps 0/1 (content_policy.py),
# playercreateinfo_action/_item for the new-character kit.
# pool_template/pool_members/game_event_pool: PoolMgr spawn pools of the Retail spawns (content_pool_*).
TDB_DIR="${TDB_DIR:-$PROJECT_DIR/data/reference/tdb1210}"
# Seven hash/layout/count-pinned Midnight crafting reference files.
CRAFTING_SOURCE_DIR="${CRAFTING_SOURCE_DIR:-$PROJECT_DIR/data/reference/crafting/12.1.0.69933}"

# Tables loaded from same-named dumps in $SQL_DIR (<table>.sql); DDL in world_schema.sql.
DUMP_TABLES=(
    creature_template creature creature_template_model creature_classlevelstats waypoint_data game_event_creature
    creature_addon creature_template_addon creature_formations creature_equip_template
    quest_template quest_template_addon quest_offer_reward quest_request_items quest_poi quest_poi_points
    creature_queststarter creature_questender gameobject_queststarter gameobject_questender
    areatrigger_involvedrelation
    npc_vendor
    creature_loot_template gameobject_loot_template reference_loot_template item_loot_template
    areatrigger_teleport world_safe_locs graveyard_zone instance_template
    gossip_menu gossip_menu_option npc_text broadcast_text points_of_interest item_template
    gameobject_template gameobject_template_addon gameobject
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

# Tables loaded from same-named TDB excerpts in $TDB_DIR; DDL in tdb_schema.sql.
TDB_TABLES=(
    skill_tiers trainer trainer_spell creature_trainer
    creature creature_template creature_template_model creature_template_difficulty
    creature_addon creature_template_addon creature_formations creature_equip_template waypoint_path waypoint_path_node game_event_creature
    gameobject gameobject_template gameobject_template_addon game_event_gameobject
    quest_template quest_template_addon quest_offer_reward quest_request_items quest_objectives quest_poi quest_poi_points
    creature_queststarter creature_questender gameobject_queststarter gameobject_questender areatrigger_involvedrelation
    npc_vendor gossip_menu gossip_menu_option npc_text creature_template_gossip broadcast_text creature_text
    creature_loot_template gameobject_loot_template reference_loot_template item_loot_template disenchant_loot_template
    areatrigger_teleport world_safe_locs graveyard_zone instance_template conditions smart_scripts
    playercreateinfo_action playercreateinfo_item spell_target_position jump_charge_params
    pool_template pool_members game_event_pool
)
TDB_ARGS=()
for table in "${TDB_TABLES[@]}"; do
    f="$TDB_DIR/$table.sql"
    if [[ ! -f "$f" ]]; then
        echo "ERROR: Missing TDB excerpt: $f" >&2
        exit 1
    fi
    TDB_ARGS+=("$table=$f")
done

if [[ ! -f "$TDB_DIR/spell_area.sql" ]]; then
    echo "ERROR: Missing TDB excerpt: $TDB_DIR/spell_area.sql" >&2
    exit 1
fi

python3 "$SCRIPT_DIR/db2.py" check --csv-root "$DB2_CSV_ROOT"
python3 "$SCRIPT_DIR/crafting_data.py" --check --source-dir "$CRAFTING_SOURCE_DIR"
python3 "$SCRIPT_DIR/content_policy.py" --source-dir "$FOREVER_DB2_SOURCE" --check --csv-root "$DB2_CSV_ROOT"

rm -f "$DB_PATH" "$DB_PATH-wal" "$DB_PATH-shm"
echo "Creating $DB_PATH ..."

SCHEMA_SQL="$SCRIPT_DIR/world_schema.sql"
sqlite3 "$DB_PATH" "PRAGMA journal_mode=WAL;" > /dev/null

# --- AzerothCore MySQL dump import (DDL from world_schema.sql, same transaction) ---

echo "  Importing AzerothCore dumps ..."
python3 "$SCRIPT_DIR/mysql_dump.py" --db "$DB_PATH" --schema "$SCHEMA_SQL" "${DUMP_ARGS[@]}"

# --- TrinityCore TDB import (Retail trainers, skill tiers, creature models and spawns) ---

echo "  Importing TDB tables ..."
python3 "$SCRIPT_DIR/mysql_dump.py" --db "$DB_PATH" --schema "$SCRIPT_DIR/tdb_schema.sql" --prefix tdb_ "${TDB_ARGS[@]}"
python3 "$SCRIPT_DIR/mysql_dump.py" --db "$DB_PATH" --schema "$SCRIPT_DIR/spell_area_schema.sql" --prefix tdb_ "spell_area=$TDB_DIR/spell_area.sql"
python3 "$SCRIPT_DIR/retail_action_bars.py" --db "$DB_PATH"

# --- Retail DB2 CSV import (build-pinned, see db2.py) ---

echo "  Importing DB2 tables ..."
python3 "$SCRIPT_DIR/db2.py" import --source-dir "$FOREVER_DB2_SOURCE" --csv-root "$DB2_CSV_ROOT" --db "$DB_PATH"

# --- Pinned crafting references (required by content_policy.py) ---

echo "  Importing crafting reference tables ..."
python3 "$SCRIPT_DIR/crafting_data.py" --db "$DB_PATH" --source-dir "$CRAFTING_SOURCE_DIR"

# --- Retail-first content tables (content_*, see content_policy.py) ---

echo "  Deriving content tables ..."
python3 "$SCRIPT_DIR/content_policy.py" --source-dir "$FOREVER_DB2_SOURCE" --db "$DB_PATH" --csv-root "$DB2_CSV_ROOT"


echo "Database written to: $DB_PATH"
