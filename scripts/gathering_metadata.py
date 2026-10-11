#!/usr/bin/env python3
"""Import gathering prerequisites from local Lock.db2 and the TDB1210 world dump.

Lead-owned import only. Never downloads data. Uses the existing WDC5 decoder,
MySQL INSERT parser and Retail loot mapper. Idempotent, transactional; does not
change spawns, pools, skill formulas or invent missing loot. Lock slots remain
alternatives, not a conjunction of eight requirements.
"""
import argparse
from pathlib import Path
import sqlite3

import db2_casc
from mysql_dump import dump_rows
from retail_content import TDB_LOOT_OFFSET, insert_tdb_loot

LOCK_LAYOUT = 0xDA418152
LOCK_COLUMNS = ["ID", "Flags"] + [
    f"{field}_{slot}"
    for field in ("Index", "Skill", "Type", "Action")
    for slot in range(8)
]


class ImportReport:
    def __init__(self):
        self.gaps = []

    def drop(self, table, key, item, reason):
        self.gaps.append(f"{table} {key} item={item}: {reason}")


def read_locks(path):
    header, records = db2_casc.decode(
        path.read_bytes(), inline_arrays={1: 8, 2: 8, 3: 8, 4: 8}
    )
    if header["layout_hash"] != LOCK_LAYOUT:
        raise ValueError(f"Lock layout {header['layout_hash']:08X}, expected {LOCK_LAYOUT:08X}")
    if not records or len(records) < header["record_count"]:
        raise ValueError("Lock records are empty or unavailable; refusing partial import")
    rows = list(records.values())
    if any(len(row) != len(LOCK_COLUMNS) for row in rows):
        raise ValueError("Lock records do not match the authored eight-slot layout")
    return rows


def read_skinning(path):
    rows = []
    with path.open(encoding="utf-8") as source:
        for line in source:
            if line.startswith("INSERT INTO `skinning_loot_template`"):
                rows.extend(row for _, row in dump_rows(line, "skinning_loot_template", path))
    if not rows:
        raise ValueError("TDB dump has no skinning_loot_template INSERT records")
    return rows


def import_metadata(conn, lock_path, tdb_path):
    locks, skinning = read_locks(lock_path), read_skinning(tdb_path)
    report = ImportReport()
    with conn:
        conn.execute("CREATE TABLE IF NOT EXISTS gathering_lock (" + ",".join(
            f'"{column}" INTEGER NOT NULL' + (" PRIMARY KEY" if column == "ID" else "")
            for column in LOCK_COLUMNS) + ")")
        conn.execute("DELETE FROM gathering_lock")
        conn.executemany("INSERT INTO gathering_lock VALUES (" + ",".join("?" for _ in LOCK_COLUMNS) + ")", locks)
        conn.execute("CREATE TABLE IF NOT EXISTS tdb_skinning_loot_template AS SELECT * FROM tdb_creature_loot_template WHERE 0")
        conn.execute("DELETE FROM tdb_skinning_loot_template")
        conn.executemany("INSERT INTO tdb_skinning_loot_template VALUES (?,?,?,?,?,?,?,?,?,?)", skinning)
        conn.execute("DROP TABLE IF EXISTS content_skinning_loot_template")
        conn.execute("CREATE TABLE content_skinning_loot_template AS SELECT * FROM content_creature_loot_template WHERE 0")
        conn.execute("CREATE UNIQUE INDEX gathering_skin_loot_key ON content_skinning_loot_template (Entry,Item,Reference)")
        insert_tdb_loot(conn, "skinning_loot_template", "1", report)
        # Existing Retail remapping excludes type50. Resolve from the original
        # TDB ID every time, so repeated imports never offset twice.
        conn.execute(f"""UPDATE content_gameobject_template SET Data1 = (
            SELECT CASE WHEN t.Data1 = 0 THEN 0 ELSE t.Data1 + {TDB_LOOT_OFFSET} END
            FROM tdb_gameobject_template t WHERE t.entry = content_gameobject_template.entry)
            WHERE type = 50 AND entry IN (SELECT entry FROM tdb_gameobject_template WHERE type = 50)""")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock-db2", type=Path, required=True)
    parser.add_argument("--tdb-dump", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    # mode=rw refuses to create a new, accidentally empty world database.
    with sqlite3.connect(f"file:{args.db.resolve()}?mode=rw", uri=True) as conn:
        report = import_metadata(conn, args.lock_db2, args.tdb_dump)
        print("Lock rows:", conn.execute("SELECT COUNT(*) FROM gathering_lock").fetchone()[0])
        print("Skinning rows:", conn.execute("SELECT COUNT(*) FROM content_skinning_loot_template").fetchone()[0])
    for gap in report.gaps:
        print("GAP:", gap)


if __name__ == "__main__":
    main()
