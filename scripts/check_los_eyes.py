#!/usr/bin/env python3
"""Reject a LOS table that does not cover every hosted spawned display and pose."""

import argparse
import csv
import sqlite3
from pathlib import Path

from ground_files import selected_map_sources


STAND_STATES = set(range(10))


def missing_eyes(world_db, table):
    maps = {row[0] for row in selected_map_sources(world_db)}
    with sqlite3.connect(f"file:{world_db}?mode=ro", uri=True) as db:
        runtime_sql = Path(__file__).with_name("los-runtime-displays.sql").read_text()
        runtime_displays = {row[0] for row in db.execute(runtime_sql)}
        displays = {
            (map_id, display)
            for map_id, display in db.execute(
                """SELECT c.map, m.CreatureDisplayID FROM content_creature c
                   JOIN content_creature_template_model m ON m.CreatureID = c.id1
                   UNION SELECT map, CAST(modelid AS INTEGER) FROM content_creature
                   WHERE modelid != 0"""
            )
            if map_id in maps
        }
    states = {}
    with table.open(newline="") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            states.setdefault(int(row["display_id"]), set()).add(int(row["stand_state"]))
    missing = [(str(map_id), display, sorted(STAND_STATES - states.get(display, set())))
               for map_id, display in displays if STAND_STATES - states.get(display, set())]
    missing.extend(("runtime", display, sorted(STAND_STATES - states.get(display, set())))
                   for display in runtime_displays if STAND_STATES - states.get(display, set()))
    return sorted(missing)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--table", type=Path, required=True)
    parser.add_argument("--gaps", type=Path, default=Path(__file__).with_name("los-eye-gaps.tsv"))
    args = parser.parse_args()
    with args.gaps.open(newline="") as stream:
        allowed = {int(row["display_id"]) for row in csv.DictReader(stream, delimiter="\t") if row["cause"]}
    missing = [(map_id, display, states) for map_id, display, states in missing_eyes(args.world_db, args.table)
               if display not in allowed]
    if missing:
        for map_id, display, states in missing:
            print(f"{map_id}: display {display} missing eye states {states}")
        raise SystemExit(f"LOS eye table incomplete: {len(missing)} map/display pairs")
    print("LOS eye table covers hosted spawned displays except listed gaps")


if __name__ == "__main__":
    main()
