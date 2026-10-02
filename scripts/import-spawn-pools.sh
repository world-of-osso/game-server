#!/usr/bin/env bash
# Add only the spawn pools to an existing world.db without rebuilding it: TDB pool_template, pool_members and
# game_event_pool (tdb_schema.sql DDL, data/reference/tdb1210 excerpts) and the content_pool_* and
# content_game_event_pool tables retail_content.build_pools derives from them. Every other table is left as it is.
# Usage: scripts/import-spawn-pools.sh [path/to/world.db]
set -euo pipefail
G="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="$(realpath "${1:-$G/data/world.db}")"
T="${TDB_DIR:-$G/data/reference/tdb1210}"
python3 "$G/scripts/mysql_dump.py" --db "$DB" \
  --schema <(sed -n '/^DROP TABLE IF EXISTS tdb_pool_template;/,$p' "$G/scripts/tdb_schema.sql") --prefix tdb_ \
  pool_template="$T/pool_template.sql" pool_members="$T/pool_members.sql" game_event_pool="$T/game_event_pool.sql"
cd "$G/scripts" && python3 - "$DB" <<'PY'
import sqlite3, sys
import retail_content
from content_common import Report
conn = sqlite3.connect(sys.argv[1], isolation_level=None)
conn.execute("BEGIN")
report = Report()
retail_content.build_pools(conn, report)
conn.execute("COMMIT")
for table in ("content_pool_template", "content_pool_members", "content_game_event_pool"):
    print(f"  {table}:", conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0])
print("  dropped pool members (spawn not imported):", report.counts["tdb_pool_members"])
PY
