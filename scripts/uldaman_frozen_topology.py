#!/usr/bin/env python3
"""Prepare native Uldaman graph inputs and freeze qualified offline walk edges."""
import argparse
import csv
import json
import math
from pathlib import Path
import sqlite3
import struct

MAP_ID = 2451
ENTRANCE_ID = 100032
NEIGHBOURS = 4


def native_float(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def pack_point(pack):
    return [pack["position"][0], pack["height"], pack["position"][1]]


def read_native_graph(database):
    with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True) as db:
        db.row_factory = sqlite3.Row
        db.execute("BEGIN")
        spawns = {row["guid"]: dict(row) for row in db.execute(
            "SELECT * FROM content_creature WHERE map=? AND (spawnMask & 1)!=0 ORDER BY guid",
            (MAP_ID,),
        )}
        formations = dict(db.execute(
            "SELECT f.memberGUID,f.leaderGUID FROM content_creature_formations f "
            "JOIN content_creature c ON c.guid=f.memberGUID WHERE c.map=?", (MAP_ID,),
        ).fetchall())
        entrance = dict(db.execute(
            "SELECT * FROM tdb_world_safe_locs WHERE ID=? AND MapID=?", (ENTRANCE_ID, MAP_ID),
        ).fetchone())
        db.rollback()
    groups = {}
    for guid, spawn in spawns.items():
        leader = formations.get(guid, guid)
        groups.setdefault(leader, []).append({
            "guid": guid, "entry": spawn["id1"],
            "position": [native_float(spawn["position_x"]), native_float(-spawn["position_y"])],
        })
    packs = []
    for identity, members in sorted(groups.items()):
        leader = spawns[identity]
        packs.append({
            "id": identity,
            "position": [native_float(leader["position_x"]), native_float(-leader["position_y"])],
            "height": native_float(leader["position_z"]),
            "spawns": sorted(members, key=lambda row: row["guid"]),
        })
    origin = [native_float(entrance["LocX"]), native_float(entrance["LocZ"]),
              native_float(-entrance["LocY"])]
    nearest = min(packs, key=lambda pack: (math.dist(pack_point(pack), origin), pack["id"]))["id"]
    return {
        "map_id": MAP_ID, "entrance": [origin[0], origin[2]], "entrance_height": origin[1],
        "entrance_pack": nearest, "packs": packs, "edges": [], "teleports": [],
        "unreachable": [pack["id"] for pack in packs if pack["id"] != nearest],
        "route": [], "boss_path": [],
    }


def read_native_edges(path, packs):
    identities = {pack["id"] for pack in packs}
    expected = len(identities) * (len(identities) - 1) // 2
    observed, edges = set(), []
    with path.open() as stream:
        for row in csv.DictReader(stream):
            pair = (int(row["from"]), int(row["to"]))
            if pair[0] >= pair[1] or not set(pair) <= identities or pair in observed:
                raise ValueError(f"invalid or duplicate native pair {pair}")
            observed.add(pair)
            if row["status"] == "walk":
                yards = float(row["yards"])
                if not math.isfinite(yards) or yards <= 0:
                    raise ValueError(f"invalid native distance for {pair}")
                edges.append({"from": pair[0], "to": pair[1], "yards": yards})
            elif row["status"] == "coincident":
                if float(row["yards"]) != 0:
                    raise ValueError(f"nonzero coincident pair {pair}")
            elif row["status"] != "blocked" or row["yards"]:
                raise ValueError(f"unknown native pair result {row}")
    if len(observed) != expected:
        raise ValueError(f"native query coverage {len(observed)} != {expected}")
    return sorted(edges, key=lambda edge: (edge["yards"], edge["from"], edge["to"]))


def select_native_edges(packs, candidates):
    """Same minimum-spanning-forest plus four-neighbour rule as graph.rs."""
    roots = {pack["id"]: pack["id"] for pack in packs}
    counts = dict.fromkeys(roots, 0)
    selected = []
    for edge in candidates:
        a, b = edge["from"], edge["to"]
        left, right = roots[a], roots[b]
        connects = left != right
        neighbour = counts[a] < NEIGHBOURS or counts[b] < NEIGHBOURS
        if connects:
            roots = {node: left if root == right else root for node, root in roots.items()}
        if connects or neighbour:
            counts[a] += 1
            counts[b] += 1
            selected.append(edge)
    return sorted(selected, key=lambda edge: (edge["from"], edge["to"]))


def unreachable_packs(graph):
    adjacency = {pack["id"]: [] for pack in graph["packs"]}
    for edge in graph["edges"]:
        adjacency[edge["from"]].append(edge["to"])
        adjacency[edge["to"]].append(edge["from"])
    reached = {graph["entrance_pack"]}
    pending = [graph["entrance_pack"]]
    while pending:
        for node in adjacency[pending.pop()]:
            if node not in reached:
                reached.add(node)
                pending.append(node)
    return sorted(set(adjacency) - reached)


def write_graph(path, graph):
    path.write_text(json.dumps(graph, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="action", required=True)
    prepare = subcommands.add_parser("prepare")
    prepare.add_argument("--db", type=Path, required=True)
    prepare.add_argument("--graph", type=Path, required=True)
    prepare.add_argument("--points", type=Path, required=True)
    freeze = subcommands.add_parser("freeze-edges")
    freeze.add_argument("--graph", type=Path, required=True)
    freeze.add_argument("--edges", type=Path, required=True)
    args = parser.parse_args()
    if args.action == "prepare":
        destinations = [args.graph.resolve(), args.points.resolve()]
        if args.db.resolve() in destinations or len(set(destinations)) != 2:
            raise ValueError("graph/points must be distinct and never replace the source database")
        graph = read_native_graph(args.db)
        write_graph(args.graph, graph)
        args.points.write_text("".join(
            ",".join(str(value) for value in [pack["id"], *pack_point(pack)]) + "\n"
            for pack in graph["packs"]
        ))
    else:
        graph = json.loads(args.graph.read_text())
        if graph["map_id"] != MAP_ID or graph["route"]:
            raise ValueError("expected un-authored Uldaman source topology")
        candidates = read_native_edges(args.edges, graph["packs"])
        graph["edges"] = select_native_edges(graph["packs"], candidates)
        graph["unreachable"] = unreachable_packs(graph)
        write_graph(args.graph, graph)
    print(f"Uldaman native topology packs={len(graph['packs'])} edges={len(graph['edges'])}; "
          "pull route remains unqualified until real native corridor inputs")


if __name__ == "__main__":
    main()
