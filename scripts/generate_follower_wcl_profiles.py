#!/usr/bin/env python3
"""Generate traceable follower profile candidates from selected WCL rankings.

Candidates preserve WCL gear and talents verbatim. They are not qualified for
runtime use until their native item levels and talent allocations are validated.
"""
import argparse
import hashlib
import json
import sqlite3
from pathlib import Path


BANDS = {
    "2-5": (2, 5),
    "6-9": (6, 9),
    "10-14": (10, 14),
    "15-19": (15, 19),
    "20+": (20, None),
}


def normalize(value):
    return "".join(
        character.lower()
        for character in value
        if character.isascii() and character.isalnum()
    )


def read_spec_catalog(path):
    uri = path.resolve().as_uri() + "?mode=ro"
    with sqlite3.connect(uri, uri=True) as database:
        rows = database.execute(
            "SELECT c.ID, c.Name_lang, s.ID, s.Name_lang "
            "FROM chr_classes c JOIN chr_specialization s ON s.ClassID=c.ID"
        )
        catalog = {}
        for class_id, class_name, spec_id, spec_name in rows:
            key = (normalize(class_name), normalize(spec_name))
            if key in catalog:
                raise ValueError(f"ambiguous native specialization {class_name}/{spec_name}")
            catalog[key] = (class_id, spec_id)
        return catalog


def generate(source_bytes, catalog):
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
        identity = (normalize(character["class"]), normalize(character["spec"]))
        if identity not in catalog:
            raise ValueError(f"{key}: native class/spec {identity} is missing")
        class_id, spec_id = catalog[identity]
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
            "class_id": class_id,
            "spec_id": spec_id,
            "key_band": band,
            "actual_key": level,
            "race": race,
            "character": character["name"],
            "server": character["server"],
            "report": report,
            "gear": character["gear"],
            "talents": character["talents"],
        }
    catalog_rows = sorted(
        (class_name, spec_name, *ids)
        for (class_name, spec_name), ids in catalog.items()
    )
    catalog_bytes = json.dumps(catalog_rows, separators=(",", ":")).encode()
    return {
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "native_spec_catalog": "chr_classes and chr_specialization",
        "native_spec_catalog_sha256": hashlib.sha256(catalog_bytes).hexdigest(),
        "source": "Warcraft Logs characterRankings, Midnight Season 2, Ruby Life Pools encounter 112521",
        "qualification": "candidate: native item levels and talent allocations not validated",
        "profiles": profiles,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("selected", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--world-db", type=Path, required=True, help="native database, opened read-only"
    )
    args = parser.parse_args()
    result = generate(args.selected.read_bytes(), read_spec_catalog(args.world_db))
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
