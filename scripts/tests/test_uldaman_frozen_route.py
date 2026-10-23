"""Behavioral data contract for the source-owned Uldaman frozen graph."""
import json
from pathlib import Path
import sqlite3
import struct
import unittest

ROOT = Path(__file__).resolve().parents[2]
GRAPH = ROOT / "content/dungeons/uldaman-2451.json"
WORLD = Path("/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db")


def f32_bytes(value):
    return struct.pack("<f", value)


class UldamanFrozenSourceAcceptance(unittest.TestCase):
    def read_graph(self):
        self.assertTrue(GRAPH.is_file(), "Uldaman2451 frozen source artifact is missing")
        return json.loads(GRAPH.read_text())

    def test_every_normal_source_identity_and_formation_pose_is_retained(self):
        graph = self.read_graph()
        with sqlite3.connect(WORLD.as_uri() + "?mode=ro", uri=True) as db:
            db.row_factory = sqlite3.Row
            rows = [dict(r) for r in db.execute(
                "SELECT * FROM content_creature WHERE map=2451 AND (spawnMask & 1)!=0 ORDER BY guid"
            )]
            formations = {r[0]: r[1] for r in db.execute(
                "SELECT f.memberGUID,f.leaderGUID FROM content_creature_formations f "
                "JOIN content_creature c ON c.guid=f.memberGUID WHERE c.map=2451"
            )}
        self.assertEqual(graph["map_id"], 2451)
        self.assertEqual(len(rows), 227)
        actual = {s["guid"]: (p, s) for p in graph["packs"] for s in p["spawns"]}
        self.assertEqual(set(actual), {r["guid"] for r in rows})
        self.assertEqual(sum(len(p["spawns"]) for p in graph["packs"]), len(rows))
        source = {r["guid"]: r for r in rows}
        for row in rows:
            pack, spawn = actual[row["guid"]]
            self.assertEqual(spawn["entry"], row["id1"])
            self.assertEqual(pack["id"], formations.get(row["guid"], row["guid"]))
            self.assertEqual([f32_bytes(v) for v in spawn["position"]],
                             [f32_bytes(row["position_x"]), f32_bytes(-row["position_y"])])
            leader = source[pack["id"]]
            self.assertEqual(f32_bytes(pack["height"]), f32_bytes(leader["position_z"]))
            self.assertEqual([f32_bytes(v) for v in pack["position"]],
                             [f32_bytes(leader["position_x"]), f32_bytes(-leader["position_y"])])

    def test_native_progression_keeps_encounter_order_and_only_frozen_walk_edges(self):
        graph = self.read_graph()
        path = graph["boss_path"]
        self.assertGreater(len(path), 0)
        edges = {frozenset((e["from"], e["to"])) for e in graph["edges"]}
        for left, right in zip(path, path[1:]):
            self.assertIn(frozenset((left, right)), edges)
        dwarves = {29004390, 29004392, 29004388}
        self.assertTrue(dwarves <= set(path))
        ordered = [29004320, 29004417, 29004496, 29004522]
        self.assertLess(max(path.index(guid) for guid in dwarves), path.index(ordered[0]))
        positions = [path.index(guid) for guid in ordered]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(graph["teleports"], [])
        self.assertFalse(set(dwarves | set(ordered)) & set(graph["unreachable"]))

    def test_frozen_pulls_keep_all_native_bosses_and_no_duplicate_pack(self):
        graph = self.read_graph()
        route = graph["route"]
        self.assertGreater(len(route), 0, "Native corridor inputs have not qualified a frozen pull route")
        self.assertEqual(graph["route_policy"], "FOLLOWER_ROUTE_BOSS_PATH_CORRIDOR")
        route_ids = [step["pack"] for step in route]
        self.assertEqual(len(route_ids), len(set(route_ids)))
        packs = {p["id"]: p for p in graph["packs"]}
        visited_entries = {s["entry"] for pid in route_ids for s in packs[pid]["spawns"]}
        self.assertTrue({184580, 184581, 184582, 184018, 184124, 184422, 184125} <= visited_entries)
        for step in route:
            self.assertEqual(len(step["anchor"]), 3)
            self.assertIn(step["pack"], packs)
        self.assertEqual(graph["teleports"], [])


if __name__ == "__main__":
    unittest.main()
