#!/usr/bin/env python3
"""Prepare source-owned follower topology inputs; never infer missing bosses or walk links."""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import sqlite3
import struct


class SourceGap(ValueError):
    """Named refusal: incomplete source data is not a simulated encounter."""


def refuse(map_id, reason, detail):
    raise SourceGap(f"FOLLOWER_GRAPH_{reason}_{map_id}: {detail}")


def native_float(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def encounter_requirements(db, map_id, journal_rows):
    encounters = db.execute(
        "SELECT ID,OrderIndex FROM dungeon_encounter WHERE MapID=? AND DifficultyID=0 "
        "ORDER BY OrderIndex,ID", (map_id,),
    ).fetchall()
    if not encounters:
        refuse(map_id, "MISSING_ENCOUNTER_ORDER", "no default DungeonEncounter records")
    bindings = [dict(row) for row in db.execute(
        "SELECT * FROM raid_encounter_creature WHERE map_id=?", (map_id,),
    )]
    boss_order = []
    for encounter in encounters:
        actors = [row for row in bindings if row["encounter_id"] == encounter["ID"]]
        if not actors:
            refuse(map_id, "MISSING_ENCOUNTER_BOSS_SOURCE", f"encounter{encounter['ID']}")
        for actor in actors:
            journal = journal_rows.get(actor["journal_creature_id"])
            if journal is None or int(journal["JournalEncounterID"]) != actor["journal_encounter_id"]:
                refuse(map_id, "MISSING_JOURNAL_ORDER", f"creature{actor['creature_entry']}")
        actors.sort(key=lambda row: (int(journal_rows[row["journal_creature_id"]]["OrderIndex"]),
                                    row["journal_creature_id"]))
        boss_order.extend(row["creature_entry"] for row in actors)
    return {"difficulty_id": 205, "boss_order": boss_order, "gate_entries": {},
            "source": "Canonical raid_encounter_creature bindings in DungeonEncounter.OrderIndex "
                      "and DB2 70448 JournalEncounterCreature.OrderIndex order; "
                      "no unbound journal add or static prerequisite inferred."}


def normal_spawns(db, map_id):
    return {row["guid"]: dict(row) for row in db.execute(
        "SELECT * FROM content_creature WHERE map=? AND (spawnMask & 1)!=0 ORDER BY guid",
        (map_id,),
    )}


def source_packs(db, map_id, spawns):
    formations = dict(db.execute(
        "SELECT f.memberGUID,f.leaderGUID FROM content_creature_formations f "
        "JOIN content_creature c ON c.guid=f.memberGUID WHERE c.map=?", (map_id,),
    ))
    groups = {}
    for guid, spawn in spawns.items():
        leader = formations.get(guid, guid)
        if leader not in spawns:
            refuse(map_id, "MISSING_FORMATION_LEADER", f"source{guid} leader{leader}")
        groups.setdefault(leader, []).append({
            "guid": guid, "entry": spawn["id1"],
            "position": [native_float(spawn["position_x"]), native_float(-spawn["position_y"])],
        })
    return [{"id": leader,
             "position": [native_float(spawns[leader]["position_x"]),
                          native_float(-spawns[leader]["position_y"])],
             "height": native_float(spawns[leader]["position_z"]),
             "spawns": members}
            for leader, members in sorted(groups.items())]


def read_source_graph(db, map_id, nav_directory, journal_rows):
    db.row_factory = sqlite3.Row
    follower = db.execute(
        "SELECT id FROM follower_lfg_dungeons WHERE map_id=? AND difficulty_id=205",
        (map_id,),
    ).fetchone()
    difficulty = db.execute(
        "SELECT ContentTuningID FROM map_difficulty WHERE MapID=? AND DifficultyID=205",
        (map_id,),
    ).fetchone()
    if follower is None or difficulty is None:
        refuse(map_id, "MISSING_FOLLOWER_DIFFICULTY", "no follower205 map/catalog binding")
    entrances = db.execute(
        "SELECT * FROM tdb_world_safe_locs WHERE MapID=? AND Comment LIKE '%Entrance%' ORDER BY ID",
        (map_id,),
    ).fetchall()
    if len(entrances) != 1:
        refuse(map_id, "MISSING_OR_AMBIGUOUS_ENTRANCE", f"source IDs{[row['ID'] for row in entrances]}")
    requirements = encounter_requirements(db, map_id, journal_rows)
    spawns = normal_spawns(db, map_id)
    if not spawns:
        refuse(map_id, "MISSING_NORMAL_POPULATION", "no source Normal actors")
    for entry in requirements["boss_order"]:
        found = [row["guid"] for row in spawns.values() if row["id1"] == entry]
        if len(found) != 1:
            refuse(map_id, "MISSING_OR_AMBIGUOUS_BOSS_SPAWN", f"entry{entry} source GUIDs{found}")
    if not nav_directory.is_dir() or not any(nav_directory.glob("*.nav")):
        refuse(map_id, "MISSING_NAV_BAKE", str(nav_directory))
    packs = source_packs(db, map_id, spawns)
    entrance = entrances[0]
    origin = [native_float(entrance["LocX"]), native_float(entrance["LocZ"]),
              native_float(-entrance["LocY"])]
    nearest = min(packs, key=lambda pack: (
        math.dist([pack["position"][0], pack["height"], pack["position"][1]], origin), pack["id"],
    ))["id"]
    graph = {"map_id": map_id, "entrance": [origin[0], origin[2]], "entrance_height": origin[1],
             "entrance_pack": nearest, "packs": packs, "edges": [], "teleports": [],
             "unreachable": [pack["id"] for pack in packs if pack["id"] != nearest],
             "route": [], "boss_path": []}
    receipt = {"map_id": map_id, "follower_lfg_id": follower["id"], "difficulty_id": 205,
               "content_tuning_id": difficulty["ContentTuningID"], "entrance_id": entrance["ID"],
               "native_population_difficulty": 1, "boss_entries": requirements["boss_order"],
               "source_coverage": "explicit raid bindings only; unbound joint actors remain a source gap",
               "nav_directory": str(nav_directory), "pull_route_qualified": False}
    return graph, requirements, receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--map", type=int, required=True)
    parser.add_argument("--nav", type=Path, required=True)
    parser.add_argument("--journal-creatures", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    destinations = [args.output / name for name in [f"follower-{args.map}.json", "requirements.json",
                                                   "points.csv", "receipt.json", "refusal.json"]]
    if args.db.resolve() in [path.resolve() for path in destinations]:
        parser.error("outputs must not replace the read-only source database")
    args.output.mkdir(parents=True, exist_ok=True)
    with args.journal_creatures.open() as stream:
        journal_rows = {int(row["ID"]): row for row in csv.DictReader(stream)}
    with sqlite3.connect(args.db.resolve().as_uri() + "?mode=ro", uri=True) as db:
        try:
            graph, requirements, receipt = read_source_graph(db, args.map, args.nav, journal_rows)
        except SourceGap as error:
            (args.output / "refusal.json").write_text(json.dumps({"map_id": args.map,
                "status": "source_refused", "reason": str(error)}, indent=2) + "\n")
            parser.exit(2, str(error) + "\n")
    graph_path = args.output / f"follower-{args.map}.json"
    graph_path.write_text(json.dumps(graph, indent=2) + "\n")
    (args.output / "requirements.json").write_text(json.dumps({str(args.map): requirements}, indent=2) + "\n")
    points = "".join(f"{pack['id']},{pack['position'][0]},{pack['height']},{pack['position'][1]}\n"
                     for pack in graph["packs"])
    (args.output / "points.csv").write_text(points)
    receipt["graph_sha256"] = hashlib.sha256(graph_path.read_bytes()).hexdigest()
    (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"SOURCE_TOPOLOGY map{args.map} packs{len(graph['packs'])}; native edges/radii still unqualified")


if __name__ == "__main__":
    main()
