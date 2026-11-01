#!/usr/bin/env python3
"""Select each healer specialization's most common WCL talent loadout.

Input: JSON mapping "<encounter>-<spec_id>" to ranked characters, each with
`name`, `server`, `hardModeLevel` and WCL `talents` ([{talentID, points}]), as
returned by `wcl rankings <encounter> --difficulty 10 --metric hps --class C
--spec S --bracket B --page N --combatant-info --json`. Characters are counted
once per (name, server) at the requested key level; the exact loadout shared
by the most characters wins, ties broken by the smaller sorted entry list.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path


def loadout(character):
    return tuple(sorted((talent["talentID"], talent["points"]) for talent in character["talents"]))


def select(rankings, key_level):
    by_spec = collections.defaultdict(dict)
    for group, characters in sorted(rankings.items()):
        spec_id = int(group.split("-")[1])
        for character in characters:
            if character["hardModeLevel"] != key_level or not character["talents"]:
                continue
            server = (character.get("server") or {}).get("name")
            by_spec[spec_id].setdefault((character["name"], server), loadout(character))
    selected = {}
    for spec_id, characters in sorted(by_spec.items()):
        counts = collections.Counter(characters.values())
        best = min(counts.items(), key=lambda item: (-item[1], item[0]))
        selected[str(spec_id)] = {
            "characters": len(characters),
            "distinct_loadouts": len(counts),
            "loadout_characters": best[1],
            "talent_entries": [list(entry) for entry in best[0]],
        }
    return selected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("rankings", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--key-level", type=int, required=True)
    parser.add_argument("--source", required=True, help="human-readable WCL query description")
    args = parser.parse_args()
    payload = args.rankings.read_bytes()
    result = {
        "source": args.source,
        "key_level": args.key_level,
        "rankings_sha256": hashlib.sha256(payload).hexdigest(),
        "specs": select(json.loads(payload), args.key_level),
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
