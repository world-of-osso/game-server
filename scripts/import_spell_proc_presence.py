#!/usr/bin/env python3
"""Import audited TC spell_proc presence; numeric overlays belong to their producer.

The audit supplies effective rows after ordered SQL updates. Verify every pinned
source, then publish only SpellId into an existing world database transactionally.
Retained rows keep any value columns added by the later full importer.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


def verify_pinned_sources(audit):
    sources = [(audit["base"]["path"], audit["base"]["sha256"])]
    sources.extend((patch["source"], patch["sha256"]) for patch in audit["patches"])
    for name, expected in sources:
        actual = hashlib.sha256(Path(name).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"SHA-256 mismatch for pinned source {name}: {actual} != {expected}")
    return [{"path": name, "sha256": digest} for name, digest in sources]


def collect_spell_ids(audit):
    ids = [row["SpellId"] for row in audit["rows"]]
    if any(type(spell) is not int or spell <= 0 for spell in ids):
        raise ValueError("audited rows require positive SpellId; rank expansion is not inferred")
    if not ids or len(ids) != audit["effective_rows"] or len(set(ids)) != len(ids):
        raise ValueError("audited rows contain duplicates or differ from effective_rows")
    return set(ids)


def publish_proc_presence(conn, ids):
    with conn:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute("CREATE TABLE IF NOT EXISTS tdb_spell_proc (SpellId INTEGER PRIMARY KEY)")
        current = {row[0] for row in conn.execute("SELECT SpellId FROM tdb_spell_proc")}
        conn.executemany("DELETE FROM tdb_spell_proc WHERE SpellId=?",
                         [(spell,) for spell in sorted(current - ids)])
        conn.executemany("INSERT INTO tdb_spell_proc (SpellId) VALUES (?)",
                         [(spell,) for spell in sorted(ids - current)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = args.manifest.read_bytes()
        audit = json.loads(raw)
        sources = verify_pinned_sources(audit)
        ids = collect_spell_ids(audit)
        database = args.db.resolve(strict=True)
        with sqlite3.connect(database) as conn:
            publish_proc_presence(conn, ids)
        print(json.dumps({"mode": "provenance only; no numeric overlay", "rows": len(ids),
                          "db": str(database), "manifest": str(args.manifest.resolve()),
                          "manifest_sha256": hashlib.sha256(raw).hexdigest(),
                          "sources": sources}, indent=2))
    except (OSError, ValueError, KeyError, TypeError, sqlite3.Error) as error:
        parser.exit(2, f"spell_proc provenance import failed: {error}\n")


if __name__ == "__main__":
    main()
