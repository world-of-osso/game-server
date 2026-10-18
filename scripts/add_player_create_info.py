#!/usr/bin/env python3
"""Add the new-character kit tables to an imported world.db, additively.

  add_player_create_info.py --source-dir DIR --db data/world.db.new [--csv-root DIR] [--tdb-dir DIR]

The import now loads the CharacterLoadout, CharacterLoadoutItem, ItemEffect and
ItemXItemEffect DB2s and TrinityCore's playercreateinfo_action / playercreateinfo_item,
and keeps all installed Item/ItemSparse definitions in content_item; the action rows
get retail_action_bars.py's main bar relocation. This applies
that to an already-imported database without re-running the import: it (re)creates
those six tables exactly as the import would and inserts the missing content_item rows;
no other row changes. Run it on a copy; the caller swaps the copy in.
"""

import argparse
import sqlite3
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
import content_policy  # noqa: E402
import db2  # noqa: E402
import retail_action_bars  # noqa: E402
import retail_content  # noqa: E402
import skyborne_data  # noqa: E402

DB2_TABLES = [
    "CharacterLoadout",
    "CharacterLoadoutItem",
    "ItemEffect",
    "ItemXItemEffect",
]
TDB_TABLES = ["playercreateinfo_action", "playercreateinfo_item"]
KIT_SCHEMA_MARKER = "-- New-character kit"


def load_tdb_tables(db_path, tdb_dir):
    """mysql_dump.py --prefix tdb_ with only the kit tables' DDL from tdb_schema.sql."""
    ddl = (SCRIPT_DIR / "tdb_schema.sql").read_text().split(KIT_SCHEMA_MARKER, 1)[1]
    schema = db_path.with_name(db_path.name + ".kit_schema.sql")
    schema.write_text(KIT_SCHEMA_MARKER + ddl)
    try:
        subprocess.run(
            [
                sys.executable,
                str(SCRIPT_DIR / "mysql_dump.py"),
                "--db",
                str(db_path),
                "--schema",
                str(schema),
                "--prefix",
                "tdb_",
                *(f"{table}={tdb_dir / f'{table}.sql'}" for table in TDB_TABLES),
            ],
            check=True,
        )
    finally:
        schema.unlink()


def table_ddl(name):
    """The CREATE TABLE statement of DB2 table `name` in db2_schema.sql."""
    schema = db2.SCHEMA_SQL.read_text()
    return (
        schema.split(f"DROP TABLE IF EXISTS {name};", 1)[1].split(");", 1)[0].strip()
        + ");"
    )


def add_player_create_info(conn, csv_dir, source_dir):
    """Replace kit DB2 tables and add missing installed item definitions; return added IDs."""
    skyborne_rows = skyborne_data.load_rows(source_dir)
    for db2_name in DB2_TABLES:
        name = db2.table_name(db2_name)
        header, rows = db2.read_csv(csv_dir / f"{db2_name}.csv")
        conn.execute(f"DROP TABLE IF EXISTS {name}")
        conn.execute(table_ddl(name))
        conn.executemany(
            f"INSERT INTO {name} VALUES ({','.join('?' * len(header))})", rows
        )
        conn.execute("DELETE FROM db2_build WHERE table_name = ?", (name,))
        conn.execute(
            "INSERT INTO db2_build (table_name, build) VALUES (?, ?)",
            (name, csv_dir.name),
        )
    skyborne_data.import_rows(conn, skyborne_rows)
    skyborne_data.project_items(conn)
    present = {item for (item,) in conn.execute("SELECT ID FROM content_item")}
    added = sorted(retail_content.retail_item_ids(conn) - present)
    conn.executemany(
        f"""INSERT INTO content_item SELECT {content_policy.ITEM_COLUMNS}, NULL AS SourceEntry
        FROM item_sparse s JOIN item i ON i.ID = s.ID WHERE s.ID = ?""",
        ((item,) for item in added),
    )
    return added


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--source-dir", required=True, type=Path)
    parser.add_argument("--csv-root", type=Path, default=db2.DEFAULT_CSV_ROOT)
    parser.add_argument(
        "--tdb-dir", type=Path, default=SCRIPT_DIR.parent / "data/reference/tdb1210"
    )
    args = parser.parse_args()
    load_tdb_tables(args.db, args.tdb_dir)
    with sqlite3.connect(args.db) as conn:
        added = add_player_create_info(
            conn, args.csv_root / db2.DEFAULT_BUILD, args.source_dir
        )
        retail_action_bars.relocate_unpaged_buttons(conn)
    print(f"added {len(added)} kit items to content_item")


if __name__ == "__main__":
    main()
