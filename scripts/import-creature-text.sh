#!/usr/bin/env bash
# Add only creature texts (boss scripts' Talk) to an existing world.db without rebuilding it:
# TDB creature_text (tdb_schema.sql DDL, data/reference/tdb1210/creature_text.sql) and the
# content_creature_text table content_policy.py derives from it. Every other table is left as it is.
# Usage: scripts/import-creature-text.sh [path/to/world.db]
set -euo pipefail
G="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB="$(realpath "${1:-$G/data/world.db}")"
T="${TDB_DIR:-$G/data/reference/tdb1210}"
python3 "$G/scripts/mysql_dump.py" --db "$DB" \
  --schema <(sed -n '/^DROP TABLE IF EXISTS tdb_creature_text;/,/^);/p' "$G/scripts/tdb_schema.sql") --prefix tdb_ \
  creature_text=$T/creature_text.sql
cd "$G/scripts" && python3 - "$DB" <<'PY'
import sqlite3, sys
import retail_content
conn = sqlite3.connect(sys.argv[1], isolation_level=None)
conn.execute("BEGIN")
retail_content.build_creature_text(conn)
conn.execute("COMMIT")
print("  content_creature_text:", conn.execute("SELECT COUNT(*) FROM content_creature_text").fetchone()[0])
PY
