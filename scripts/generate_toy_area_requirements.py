#!/usr/bin/env python3
"""Generate Toy root area limits from the current canonical local-CASC CSV exports.

Source world.db is read-only. Root selection matches toy_catalog::load; hashes retain
CSV identity. This producer handles RequiredAreasID only, not facing/procedural effects.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import sqlite3

ROOTS_SQL = """
    SELECT t.ItemID,e.SpellID FROM toy t LEFT JOIN (
        SELECT x.ItemID,e.SpellID,ROW_NUMBER() OVER (
            PARTITION BY x.ItemID ORDER BY e.LegacySlotIndex,e.ID) AS slot
        FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID
        WHERE e.TriggerType=0 AND e.SpellID>0
    ) e ON e.ItemID=t.ItemID AND e.slot=1
"""


def read_toy_roots(database):
    with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True) as conn:
        return {spell for _, spell in conn.execute(ROOTS_SQL) if spell is not None}


def read_cast_area_groups(path, roots):
    groups = {}
    with path.open(newline="") as handle:
        for row in csv.DictReader(handle):
            spell = int(row["SpellID"])
            group = int(row["RequiredAreasID"])
            if spell not in roots or group == 0:
                continue
            if spell in groups and groups[spell] != group:
                raise ValueError(f"spell{spell} has conflicting source area groups")
            groups[spell] = group
    return groups


def read_group_members(path, needed):
    groups = {group: set() for group in needed}
    with path.open(newline="") as handle:
        for row in csv.DictReader(handle):
            group = int(row["AreaGroupID"])
            if group in groups:
                groups[group].add(int(row["AreaID"]))
    for group, members in groups.items():
        if not members:
            raise ValueError(f"area group{group} has no source members")
    return {group: sorted(members) for group, members in groups.items()}


def assemble_manifest(spells, groups, sources, build):
    if not build.strip():
        raise ValueError("source CSV export build is empty")
    return {"build": build, "sources": sources, "spells": spells, "groups": groups}


def write_toy_area_manifest(database, requirements, members, output, build):
    roots = read_toy_roots(database)
    spells = read_cast_area_groups(requirements, roots)
    groups = read_group_members(members, set(spells.values()))
    sources = {
        "SpellCastingRequirements": hashlib.sha256(requirements.read_bytes()).hexdigest(),
        "AreaGroupMember": hashlib.sha256(members.read_bytes()).hexdigest(),
    }
    manifest = assemble_manifest(spells, groups, sources, build)
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return {"spells": len(spells), "groups": len(groups)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--casting-requirements", type=Path, required=True)
    parser.add_argument("--area-group-members", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--build", required=True, help="Build of the current canonical CSV export, supplied by its apply step")
    args = parser.parse_args()
    print(json.dumps(write_toy_area_manifest(
        args.db, args.casting_requirements, args.area_group_members, args.output, args.build,
    )))


if __name__ == "__main__":
    main()
