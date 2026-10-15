#!/usr/bin/env python3
"""Generate Toy facing flags from current canonical CSVs and source TDB model radii.

TC a352b1fa Spell::CheckRange, Unit::IsWithinBoundaryRadius and
Position::IsInDist: 180 degrees, strict exact3D radius, never CombatReach.
Reads source files only; writes an inspectable runtime manifest, not world.db.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path

from generate_toy_area_requirements import read_toy_roots
from encounter_model_geometry import COLUMNS
import mysql_dump


def read_facing_flags(path, roots):
    with path.open(newline="") as handle:
        rows = list(csv.DictReader(handle))
    return {int(row["SpellID"]): int(row["FacingCasterFlags"])
            for row in rows if int(row["SpellID"]) in roots and int(row["FacingCasterFlags"]) & 1}


def read_model_radii(path):
    radii = {}
    with path.open(encoding="utf-8") as handle:
        for statement in handle:
            if not statement.startswith("INSERT INTO `creature_model_info`"):
                continue
            for columns, row in mysql_dump.dump_rows(statement, "creature_model_info", path):
                if columns is not None and tuple(columns) != COLUMNS:
                    raise ValueError(f"{path}: unexpected model-info column order {columns}")
                if len(row) != len(COLUMNS):
                    raise ValueError(f"{path}: incompatible model-info row")
                radii[int(row[0])] = float(row[1])
    if not radii:
        raise ValueError(f"{path}: no source bounding radii")
    return radii


def write_manifest(database, requirements, dump, output, build):
    flags = read_facing_flags(requirements, read_toy_roots(database))
    radii = read_model_radii(dump)
    if not build.strip():
        raise ValueError("source CSV export build is empty")
    manifest = {"build": build, "source": "TCa352b1fa/" + dump.name,
                "requirements_sha256": hashlib.sha256(requirements.read_bytes()).hexdigest(),
                "spells": flags, "model_radii": radii}
    output.write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    return len(flags), len(radii)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("database", "requirements", "dump", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--build", required=True, help="Build of the current canonical CSV export, supplied by its apply step")
    args = parser.parse_args()
    print(write_manifest(args.database, args.requirements, args.dump, args.output, args.build))


if __name__ == "__main__":
    main()
