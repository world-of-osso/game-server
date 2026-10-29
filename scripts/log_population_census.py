#!/usr/bin/env python3
"""Stream combat logs into a per-run Creature GUID census; input is untrusted data."""
import argparse
import csv
import json
from pathlib import Path


def census(paths, maps):
    runs = []
    for path in sorted(paths):
        current = None
        with path.open(encoding="utf-8", errors="replace") as source:
            for line in source:
                timestamp, sep, payload = line.partition("  ")
                if not sep:
                    continue
                event = payload.split(",", 1)[0]
                if event == "CHALLENGE_MODE_START":
                    fields = next(csv.reader([payload]))
                    current = None
                    if len(fields) > 2 and fields[2].isdigit() and int(fields[2]) in maps:
                        current = {"file": path.name, "map": int(fields[2]), "start": timestamp,
                                   "name": fields[1], "creatures": {}}
                        runs.append(current)
                    continue
                if event in ("CHALLENGE_MODE_END", "ZONE_CHANGE"):
                    if event == "CHALLENGE_MODE_END":
                        current = None
                    continue
                if current is None or "Creature-" not in payload:
                    continue
                fields = next(csv.reader([payload]))
                if len(fields) < 9:
                    continue
                for index in (1, 5):
                    guid = fields[index]
                    parts = guid.split("-")
                    if len(parts) != 7 or parts[0] != "Creature" or not parts[5].isdigit():
                        continue
                    creatures = current["creatures"]
                    item = creatures.setdefault(guid, {"npc_id": int(parts[5]), "spawn_uid": parts[6],
                                                       "instance": parts[4], "name": fields[index + 1],
                                                       "first_seen": timestamp, "first_event": event,
                                                       "summoned": False})
                    if event == "SPELL_SUMMON" and index == 5:
                        item["summoned"] = True
    return runs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--map", type=int, action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("logs", type=Path, nargs="+")
    args = parser.parse_args()
    paths = [f for p in args.logs for f in (sorted(p.glob("*.txt")) if p.is_dir() else [p])]
    runs = census(paths, set(args.map))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"runs": runs}, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
