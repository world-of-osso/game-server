#!/usr/bin/env bash
# Add only creature equipment (virtual items) to an existing world.db without rebuilding it:
# AzerothCore and TDB creature_equip_template (world_schema.sql / tdb_schema.sql DDL) and the
# content_creature_equip_template table retail_content.build_equipment derives from them.
# Every other table is left as it is.
# Usage: scripts/import-creature-equipment.sh [path/to/world.db] [path/to/azerothcore/db_world]
set -euo pipefail
G="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="$(realpath "${1:-$G/data/world.db}")"
SQL_DIR="${2:-/home/osso/Repos/azerothcore/data/sql/base/db_world}"
T="${TDB_DIR:-$G/data/reference/tdb1210}"
python3 "$G/scripts/mysql_dump.py" --db "$DB" \
  --schema <(sed -n '/^DROP TABLE IF EXISTS creature_equip_template;/,/^);/p' "$G/scripts/world_schema.sql") \
  creature_equip_template="$SQL_DIR/creature_equip_template.sql"
python3 "$G/scripts/mysql_dump.py" --db "$DB" \
  --schema <(sed -n '/^DROP TABLE IF EXISTS tdb_creature_equip_template;/,/^);/p' "$G/scripts/tdb_schema.sql") --prefix tdb_ \
  creature_equip_template="$T/creature_equip_template.sql"
cd "$G/scripts" && python3 - "$DB" <<'PY'
import sqlite3, sys
import retail_content
conn = sqlite3.connect(sys.argv[1], isolation_level=None)
conn.execute("BEGIN")
# build_creatures' temp.retail_entry: the entries of Retail spawns, the content guids from RETAIL_GUID_OFFSET.
conn.execute(f"""CREATE TEMP TABLE retail_entry AS SELECT DISTINCT id1 AS entry FROM content_creature
    WHERE guid >= {retail_content.RETAIL_GUID_OFFSET}""")
retail_content.build_equipment(conn)
conn.execute("COMMIT")
print("  content_creature_equip_template:",
      conn.execute("SELECT COUNT(*) FROM content_creature_equip_template").fetchone()[0])
PY
