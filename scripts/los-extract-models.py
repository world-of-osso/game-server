#!/usr/bin/env python3
"""Extract from local CASC the creature models the LOS eye table needs (docs/specs/line-of-sight.md).

For every display the bake covers (all creature template displays and spawn display overrides in
world.db, and the `ChrModel.csv` character displays), extract into game-engine `data/models/` whatever is missing of: the model
(`<fdid>.m2`), its primary skin (`<SFID>.skin`), its skeleton (`<fdid>.skel`, named after the
model as the client loads it) and the `.anim` files of the stand-state sequences
(`los::eye::STAND_STATE_ANIMS`). Uses `casc-local` (local CASC only, never the CDN), which runs
from game-engine (its `data/casc/` tables).

    scripts/los-extract-models.py --world-db data/world.db --engine ../game-engine
"""

import argparse
import csv
import shutil
import sqlite3
import struct
import subprocess
import tempfile
from pathlib import Path

# `los::eye::STAND_STATE_ANIMS` animation ids.
STAND_STATE_ANIMS = {0, 97, 100, 102, 103, 104, 6, 115, 202}
BATCH = 400


def model_fdids(world_db, data):
    db = sqlite3.connect(f"file:{world_db}?mode=ro", uri=True)
    db.execute("CREATE TEMP TABLE chr_displays (id INTEGER)")
    with (data / "ChrModel.csv").open() as chr_models:
        db.executemany("INSERT INTO chr_displays VALUES (?)", [(row["DisplayID"],) for row in csv.DictReader(chr_models)])
    runtime_displays = Path(__file__).with_name("los-runtime-displays.sql").read_text().strip()
    return sorted(
        fdid
        for (fdid,) in db.execute(
            f"""SELECT DISTINCT m.FileDataID FROM creature_display_info d
                 JOIN creature_model_data m ON m.ID = d.ModelID
                WHERE m.FileDataID != 0 AND d.ID IN (
                      {runtime_displays}
                      UNION SELECT id FROM chr_displays)"""
        )
    )


def chunks(data):
    offset = 0
    while offset + 8 <= len(data):
        tag = data[offset:offset + 4]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        yield tag, data[offset + 8:offset + 8 + size]
        offset += 8 + size


def chunk(data, tag):
    return next((body for found, body in chunks(data) if found == tag), None)


def stand_anims(data):
    afid = chunk(data, b"AFID") or b""
    return {
        fdid
        for anim, sub, fdid in struct.iter_unpack("<HHI", afid[: len(afid) // 8 * 8])
        if anim in STAND_STATE_ANIMS and sub == 0 and fdid
    }


def extract(engine, fdids, out, ext):
    """Extract `fdids` into `out` as `<fdid>.<ext>`: casc-local names a file the listfile
    lacks (newer FDIDs) `<fdid>.dat`."""
    fdids = sorted(set(fdids))
    for start in range(0, len(fdids), BATCH):
        batch = [str(fdid) for fdid in fdids[start:start + BATCH]]
        result = subprocess.run(["casc-local", *batch, "-o", str(out)], cwd=engine, capture_output=True, text=True)
        summary = [line for line in result.stdout.splitlines() if "extracted" in line and "failed" in line]
        print(f"casc-local {len(batch)} .{ext} files: {summary[-1] if summary else result.stderr.strip()[-300:]}", flush=True)
    for fdid in fdids:
        unnamed, named = out / f"{fdid}.dat", out / f"{fdid}.{ext}"
        if unnamed.is_file() and not named.is_file():
            unnamed.rename(named)


def companions(models, fdid):
    """Skin and anim FDIDs and the skeleton model `fdid` needs that are not extracted."""
    data = (models / f"{fdid}.m2").read_bytes()
    if data[:4] != b"MD21":
        return set(), set(), None
    skins = set()
    sfid = chunk(data, b"SFID")
    if sfid and len(sfid) >= 4:
        skin = struct.unpack_from("<I", sfid)[0]
        if not (models / f"{skin}.skin").is_file() and not (models / f"{fdid}00.skin").is_file():
            skins.add(skin)
    skid = chunk(data, b"SKID")
    skel = struct.unpack_from("<I", skid)[0] if skid and len(skid) >= 4 else None
    anim_source = data
    if skel is not None:
        if (models / f"{fdid}.skel").is_file():
            anim_source = (models / f"{fdid}.skel").read_bytes()
        else:
            return skins, set(), skel
    anims = {anim for anim in stand_anims(anim_source) if not (models / f"{anim}.anim").is_file()}
    return skins, anims, None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--engine", type=Path, required=True, help="game-engine checkout (data/models, data/casc)")
    args = parser.parse_args()
    engine = args.engine.resolve()
    models = engine / "data" / "models"
    fdids = model_fdids(args.world_db, engine / "data")
    missing = [fdid for fdid in fdids if not (models / f"{fdid}.m2").is_file()]
    print(f"{len(fdids)} models, {len(missing)} not extracted", flush=True)
    extract(engine, missing, models, "m2")
    # Two rounds: a skeleton extracted in the first names the anims of the second.
    for _ in range(2):
        skins, anims, skeletons = set(), set(), {}
        for fdid in fdids:
            if not (models / f"{fdid}.m2").is_file():
                continue
            model_skins, model_anims, skel = companions(models, fdid)
            skins |= model_skins
            anims |= model_anims
            if skel is not None:
                skeletons.setdefault(skel, []).append(fdid)
        print(f"{len(skins)} skins, {len(anims)} anims, {len(skeletons)} skeletons to extract", flush=True)
        extract(engine, skins, models, "skin")
        extract(engine, anims, models, "anim")
        with tempfile.TemporaryDirectory() as temporary:
            extract(engine, skeletons, Path(temporary), "skel")
            for skel, owners in skeletons.items():
                source = Path(temporary) / f"{skel}.skel"
                for owner in owners if source.is_file() else []:
                    shutil.copyfile(source, models / f"{owner}.skel")
    still = [fdid for fdid in fdids if not (models / f"{fdid}.m2").is_file()]
    print(f"models still not extracted: {len(still)} {still[:20]}")


if __name__ == "__main__":
    main()
