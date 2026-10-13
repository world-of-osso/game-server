#!/usr/bin/env python3
"""Pin Toy facing flags (local69933) and target bounding radii (TDB1210).

TC a352b1fa Spell::CheckRange, Unit::IsWithinBoundaryRadius and
Position::IsInDist: 180 degrees, strict exact3D radius, never CombatReach.
Reads source files only; writes an inspectable runtime manifest, not world.db.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path

from generate_toy_area_requirements import BUILD, read_toy_roots
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


def write_manifest(database, requirements, dump, output):
    flags = read_facing_flags(requirements, read_toy_roots(database))
    radii = read_model_radii(dump)
    manifest = {"build": BUILD, "source": "TCa352b1fa/TDB1210.26091",
                "requirements_sha256": hashlib.sha256(requirements.read_bytes()).hexdigest(),
                "spells": flags, "model_radii": radii}
    output.write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    return len(flags), len(radii)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("database", "requirements", "dump", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    print(write_manifest(args.database, args.requirements, args.dump, args.output))


if __name__ == "__main__":
    main()
