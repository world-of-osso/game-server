#!/usr/bin/env python3
"""Generate traceable follower profile candidates from selected WCL rankings.

Candidates preserve WCL gear and talents verbatim. They are not qualified for
runtime use until their native item levels and talent allocations are validated.
"""
import argparse
import hashlib
import json
from pathlib import Path


BANDS = {"2-5": (2, 5), "6-9": (6, 9), "10-14": (10, 14), "15-19": (15, 19), "20+": (20, None)}


def generate(source_bytes):
    selected = json.loads(source_bytes)
    profiles = {}
    for key, group in sorted(selected.items()):
        candidates = group["characters"]
        if not candidates:
            continue
        spec, band = key.rsplit("/", 1)
        low, high = BANDS[band]
        character = candidates[0]
        level = character["hardModeLevel"]
        if level < low or (high is not None and level > high):
            raise ValueError(f"{key}: actual key {level} is outside its band")
        if not character["gear"] or not character["talents"]:
            raise ValueError(f"{key}: ranking lacks gear or talents")
        race = group["race"]
        if not race.get("value") or not race.get("source"):
            raise ValueError(f"{key}: race provenance is missing")
        report = character["report"]
        if not report.get("code") or not report.get("fightID"):
            raise ValueError(f"{key}: report provenance is missing")
        profiles[key] = {
            "spec": spec,
            "key_band": band,
            "actual_key": level,
            "race": race,
            "character": character["name"],
            "server": character["server"],
            "report": report,
            "gear": character["gear"],
            "talents": character["talents"],
        }
    return {
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "source": "Warcraft Logs characterRankings, Midnight Season 2, Ruby Life Pools encounter 112521",
        "qualification": "candidate: native item levels and talent allocations not validated",
        "profiles": profiles,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("selected", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = generate(args.selected.read_bytes())
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
