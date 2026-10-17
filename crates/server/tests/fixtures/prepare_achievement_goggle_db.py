"""Snapshot active Retail data and project seven sourced goggle items for tests."""
import argparse
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
from content_policy import ITEM_COLUMNS
from db2 import DEFAULT_CSV_ROOT

GOGGLE_IDS = range(77533, 77540)


def prepare(source, destination):
    if destination.exists() or destination.name == "world.db":
        raise ValueError("goggle fixture requires a fresh owned database copy")
    receipt = DEFAULT_CSV_ROOT / "active/receipt.json"
    identity = json.loads(receipt.read_text())["identity"]
    build = identity["build"]
    if identity["product"] != "wow" or not build:
        raise ValueError(f"goggle fixture requires an active Retail source: {receipt}")
    with closing(sqlite3.connect(f"{source.resolve().as_uri()}?mode=ro", uri=True)) as native:
        builds = dict(native.execute(
            "SELECT table_name, build FROM db2_build WHERE table_name IN ('item', 'item_sparse')"
        ))
        if builds != {"item": build, "item_sparse": build}:
            raise ValueError(f"goggle source build mismatch: expected active {build}, got {builds}")
        with closing(sqlite3.connect(destination)) as copied, copied:
            native.backup(copied)
            for item_id in GOGGLE_IDS:
                rows = copied.execute(
                    f"SELECT {ITEM_COLUMNS}, NULL AS SourceEntry "
                    "FROM item_sparse s JOIN item i ON i.ID=s.ID WHERE s.ID=?",
                    (item_id,),
                ).fetchall()
                if len(rows) != 1:
                    raise ValueError(f"missing sourced goggle item {item_id}")
                exists = copied.execute(
                    "SELECT 1 FROM content_item WHERE ID=?", (item_id,)
                ).fetchone()
                if exists is None:
                    placeholders = ",".join("?" for _ in rows[0])
                    copied.execute(f"INSERT INTO content_item VALUES ({placeholders})", rows[0])
    return list(GOGGLE_IDS)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    items = prepare(args.source, args.destination)
    print(f"goggle fixture prepared from validated active Retail export: {items}; canonical source read-only")


if __name__ == "__main__":
    main()
