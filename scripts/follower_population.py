#!/usr/bin/env python3
"""Project authentic follower Normal spawns/entrances into an existing world DB COPY.

Sources: TDB1210 tdb_creature/tdb_world_safe_locs and authenticated local 70448
Map/MapDifficulty/LFGDungeons/AreaTrigger CSVs. No inferred portal destinations,
synthetic placements, new difficulty tags, template budgets or encounter scripts.
The copy must already contain prepared runtime templates and related content.
"""
import argparse
import csv
import hashlib
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from import_instance_population import BASE_PHASE, validate_copy
from retail_content import RETAIL_GUID_OFFSET

BUILD = "12.1.5.70448"
TABLES = ("Map", "MapDifficulty", "LFGDungeons", "AreaTrigger")
NORMAL = "(','||replace(spawnDifficulties,' ','')||',') LIKE '%,1,%'"
SPAWN_COLUMNS = (
    "guid", "id1", "map", "zoneId", "areaId", "equipment_id", "position_x",
    "position_y", "position_z", "orientation", "spawntimesecs", "wander_distance",
    "currentwaypoint", "MovementType", "unit_flags", "ScriptName", "VerifiedBuild",
    "modelid", "spawnDifficulties",
)


def read_db2_roster(directory):
    receipt = json.loads((directory / "receipt.json").read_text())
    identity = receipt["identity"]
    if identity["build"] != BUILD or identity["extraction_source"] != "authenticated-local-CASC-content-root":
        raise ValueError("require authenticated local CASC build12.1.5.70448")
    tables = {}
    hashes = {}
    for name in TABLES:
        path = directory / f"{name}.csv"
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        metadata = receipt["tables"][name]
        if not metadata["content_authenticated"] or metadata["unreadable_records"] != 0 or digest != metadata["csv_sha256"]:
            raise ValueError(f"{name} authentication/hash mismatch")
        with path.open(encoding="utf-8", newline="") as stream:
            tables[name] = list(csv.DictReader(stream))
        hashes[name] = digest
    maps = {int(row["ID"]) for row in tables["Map"] if row["InstanceType"] == "1"}
    follower_maps = {int(row["MapID"]) for row in tables["MapDifficulty"] if row["DifficultyID"] == "205"}
    roster = [row for row in tables["LFGDungeons"] if row["DifficultyID"] == "205"]
    for row in roster:
        if int(row["MapID"]) not in maps & follower_maps:
            raise ValueError(f"follower map{row['MapID']} lacks Map/MapDifficulty identity")
        if tuple(row[key] for key in ("Count_tank", "Count_healer", "Count_damage")) != ("1", "1", "3"):
            raise ValueError(f"follower row{row['ID']} is not1/1/3")
    if len(roster) != 27 or {int(row["MapID"]) for row in roster} != follower_maps:
        raise ValueError("expected all27 build70448 follower maps")
    return roster, hashes


def read_source_population(source, roster):
    result = []
    for row in sorted(roster, key=lambda value: int(value["MapID"])):
        map_id = int(row["MapID"])
        source.row_factory = sqlite3.Row
        spawns = source.execute(
            f"SELECT * FROM tdb_creature WHERE map=? AND {NORMAL} AND {BASE_PHASE} ORDER BY guid",
            (map_id,),
        ).fetchall()
        entrances = source.execute(
            "SELECT * FROM tdb_world_safe_locs WHERE MapID=? AND Comment LIKE '%Entrance%' ORDER BY ID",
            (map_id,),
        ).fetchall()
        result.append((row, spawns, entrances))
    return result


def write_spawns(target, spawns):
    names = ",".join(SPAWN_COLUMNS)
    updates = ",".join(f"{name}=excluded.{name}" for name in SPAWN_COLUMNS if name != "guid")
    for spawn in spawns:
        if not target.execute("SELECT 1 FROM content_creature_template WHERE entry=?", (spawn["id"],)).fetchone():
            raise ValueError(f"prepare runtime template{spawn['id']} before importing spawn{spawn['guid']}")
        guid = spawn["guid"] + RETAIL_GUID_OFFSET
        existing = target.execute("SELECT id1,map FROM content_creature WHERE guid=?", (guid,)).fetchone()
        if existing and tuple(existing) != (spawn["id"], spawn["map"]):
            raise ValueError(f"source GUID namespace collision at{guid}")
        values = dict(spawn)
        values.update(guid=guid, id1=spawn["id"], unit_flags=spawn["unit_flags"] or 0)
        target.execute(
            f"INSERT INTO content_creature ({names}) VALUES ({','.join('?' for _ in SPAWN_COLUMNS)}) "
            f"ON CONFLICT(guid) DO UPDATE SET {updates}",
            tuple(values[name] for name in SPAWN_COLUMNS),
        )


def write_entrances(target, entrances):
    for entrance in entrances:
        columns = tuple(entrance.keys())
        existing = target.execute("SELECT MapID FROM tdb_world_safe_locs WHERE ID=?", (entrance["ID"],)).fetchone()
        if existing and existing[0] != entrance["MapID"]:
            raise ValueError(f"safe location ID collision at{entrance['ID']}")
        target.execute(
            f"INSERT OR REPLACE INTO tdb_world_safe_locs ({','.join(columns)}) "
            f"VALUES ({','.join('?' for _ in columns)})", tuple(entrance),
        )


def apply_population(source_db, destination, db2_directory):
    destination = validate_copy(destination)
    if destination.samefile(source_db):
        raise ValueError("source and destination must be distinct files")
    roster, hashes = read_db2_roster(db2_directory)
    with closing(sqlite3.connect(source_db.resolve().as_uri() + "?mode=ro", uri=True)) as source:
        source.execute("BEGIN")
        population = read_source_population(source, roster)
    report = []
    with closing(sqlite3.connect(destination)) as target, target:
        target.execute("BEGIN IMMEDIATE")
        for row, spawns, entrances in population:
            prepared = target.execute(
                "SELECT map_id,tanks,healers,damage FROM follower_lfg_dungeons WHERE id=? AND difficulty_id=205",
                (int(row["ID"]),),
            ).fetchone()
            if prepared != (int(row["MapID"]), 1, 1, 3):
                raise ValueError(f"prepared follower roster mismatch for{row['ID']}")
            write_spawns(target, spawns)
            write_entrances(target, entrances)
            gaps = []
            if not spawns:
                gaps.append(f"FOLLOWER_NORMAL_POPULATION_MISSING_MAP_{row['MapID']}")
            if not entrances:
                gaps.append(f"FOLLOWER_ENTRANCE_MISSING_MAP_{row['MapID']}")
            report.append(dict(map_id=int(row["MapID"]), id=int(row["ID"]), name=row["Name_lang"],
                               normal_spawns=len(spawns), entrance_ids=[loc["ID"] for loc in entrances],
                               source_complete=bool(spawns and entrances), gaps=gaps))
    return dict(build=BUILD, db2_csv_sha256=hashes, maps=report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-db", required=True, type=Path)
    parser.add_argument("--db", required=True, type=Path, help="existing prepared database COPY")
    parser.add_argument("--db2-dir", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    inputs = [args.source_db, args.db, args.db2_dir / "receipt.json"]
    inputs.extend(args.db2_dir / f"{table}.csv" for table in TABLES)
    aliases_input = args.report.resolve() in {path.resolve() for path in inputs}
    if args.report.exists():
        aliases_input = aliases_input or any(args.report.samefile(path) for path in inputs)
    if aliases_input:
        parser.error("report must not overwrite a database or DB2 input")
    result = apply_population(args.source_db, args.db, args.db2_dir)
    args.report.write_text(json.dumps(result, indent=2) + "\n")
    complete = sum(row["source_complete"] for row in result["maps"])
    print(f"{complete}/27 source-complete follower maps; remaining maps retain named gaps")


if __name__ == "__main__":
    main()
