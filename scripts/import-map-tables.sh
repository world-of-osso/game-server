#!/usr/bin/env bash
# Add only the map-transfer tables (dungeons phase 1) to an existing world.db without
# rebuilding it: TDB areatrigger_teleport, world_safe_locs, graveyard_zone, instance_template
# (tdb_schema.sql DDL, data/reference/tdb1210 excerpts) and DB2 Map, MapDifficulty, AreaTrigger,
# AreaTriggerActionSet (db2_schema.sql DDL, build CSVs). Every other table is left as it is.
# Usage: scripts/import-map-tables.sh [path/to/world.db]
set -euo pipefail
G="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="$(realpath "${1:-$G/data/world.db}")"
T="${TDB_DIR:-$G/data/reference/tdb1210}"
# TDB tables: the tdb_schema.sql DDL from "DROP TABLE IF EXISTS tdb_areatrigger_teleport" to the end.
python3 "$G/scripts/mysql_dump.py" --db "$DB" \
  --schema <(sed -n '/^DROP TABLE IF EXISTS tdb_areatrigger_teleport;/,$p' "$G/scripts/tdb_schema.sql") --prefix tdb_ \
  areatrigger_teleport=$T/areatrigger_teleport.sql world_safe_locs=$T/world_safe_locs.sql \
  graveyard_zone=$T/graveyard_zone.sql instance_template=$T/instance_template.sql
# DB2 tables: their db2_schema.sql DDL, rows from the build CSVs, recorded in db2_build.
cd "$G/scripts" && python3 - "$DB" <<'PY'
import re, sqlite3, sys
import db2
tables = ["Map", "MapDifficulty", "AreaTrigger", "AreaTriggerActionSet"]
schema = db2.SCHEMA_SQL.read_text()
csv_dir = db2.DEFAULT_CSV_ROOT / db2.DEFAULT_BUILD
ddl = "".join(
    re.search(rf"DROP TABLE IF EXISTS {n};\nCREATE TABLE {n} \(.*?\n\);", schema, re.S).group(0) + "\n"
    for n in map(db2.table_name, tables))
conn = sqlite3.connect(sys.argv[1], isolation_level=None)
conn.executescript("BEGIN;\n" + ddl)
for t in tables:
    name = db2.table_name(t)
    header, rows = db2.read_csv(csv_dir / f"{t}.csv")
    conn.executemany(f"INSERT INTO {name} VALUES ({','.join('?' * len(header))})", rows)
    conn.execute("INSERT OR REPLACE INTO db2_build (table_name, build) VALUES (?, ?)", (name, db2.DEFAULT_BUILD))
    print(f"  {name}: {len(rows)}")
conn.execute("COMMIT")
PY
