#!/usr/bin/env python3
"""Import complete audited TC spell_proc rows into the existing world database.

Verify every pinned SQL source, then replace the authoritative table atomically.
Stored zero values remain zero; native DB2 defaults are resolved by the loader.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sqlite3


COLUMNS = (
    "SpellId", "SchoolMask", "SpellFamilyName", "SpellFamilyMask0", "SpellFamilyMask1",
    "SpellFamilyMask2", "SpellFamilyMask3", "ProcFlags", "ProcFlags2", "SpellTypeMask",
    "SpellPhaseMask", "HitMask", "AttributesMask", "DisableEffectsMask", "ProcsPerMinute",
    "Chance", "Cooldown", "Charges",
)
REAL_COLUMNS = {"ProcsPerMinute", "Chance"}


def verify_pinned_sources(audit):
    sources = [(audit["base"]["path"], audit["base"]["sha256"])]
    sources.extend((patch["source"], patch["sha256"]) for patch in audit["patches"])
    for name, expected in sources:
        actual = hashlib.sha256(Path(name).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"SHA-256 mismatch for pinned source {name}: {actual} != {expected}")
    return [{"path": name, "sha256": digest} for name, digest in sources]


def collect_proc_rows(audit):
    rows = audit["rows"]
    ids = [row["SpellId"] for row in rows]
    if any(type(spell) is not int or spell <= 0 for spell in ids):
        raise ValueError("audited rows require positive SpellId; rank expansion is not inferred")
    if not ids or len(ids) != audit["effective_rows"] or len(set(ids)) != len(ids):
        raise ValueError("audited rows contain duplicates or differ from effective_rows")
    values = []
    for row in rows:
        for column in COLUMNS:
            value = row[column]
            if column in REAL_COLUMNS:
                valid = type(value) in (int, float) and math.isfinite(value)
            else:
                valid = type(value) is int
            if not valid:
                raise ValueError(f"audited spell_proc {row['SpellId']} has invalid {column}: {value!r}")
        values.append(tuple(row[column] for column in COLUMNS))
    return sorted(values)


def publish_proc_values(conn, rows):
    fields = ", ".join(
        "SpellId INTEGER PRIMARY KEY" if column == "SpellId"
        else f"{column} {'REAL' if column in REAL_COLUMNS else 'INTEGER'} NOT NULL"
        for column in COLUMNS
    )
    placeholders = ", ".join("?" for _ in COLUMNS)
    with conn:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute("DROP TABLE IF EXISTS tdb_spell_proc")
        conn.execute(f"CREATE TABLE tdb_spell_proc ({fields})")
        conn.executemany(f"INSERT INTO tdb_spell_proc VALUES ({placeholders})", rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = args.manifest.read_bytes()
        audit = json.loads(raw)
        sources = verify_pinned_sources(audit)
        rows = collect_proc_rows(audit)
        database = args.db.resolve(strict=True)
        with sqlite3.connect(database) as conn:
            publish_proc_values(conn, rows)
        print(json.dumps({"mode": "complete audited spell_proc values", "rows": len(rows),
                          "db": str(database), "manifest": str(args.manifest.resolve()),
                          "manifest_sha256": hashlib.sha256(raw).hexdigest(),
                          "columns": COLUMNS, "sources": sources}, indent=2))
    except (OSError, ValueError, KeyError, TypeError, sqlite3.Error) as error:
        parser.exit(2, f"spell_proc value import failed: {error}\n")


if __name__ == "__main__":
    main()
