"""Behavioral checks for frozen boss-path corridor routes."""
import json
from pathlib import Path
import unittest
import sys

ROOT = Path(__file__).resolve().parents[2]


class FrozenAzureBossCorridor(unittest.TestCase):
    def test_leymor_precedes_the_lower_postboss_book_wing(self):
        graph = json.loads((ROOT / "content/dungeons/azure-vault-2515.json").read_text())
        packs = {pack["id"]: pack for pack in graph["packs"]}
        route = graph["route"]
        leymor = next(index for index, step in enumerate(route) if step["pack"] == 29001810)
        book_height = packs[29001868]["height"]
        premature = [step["pack"] for step in route[:leymor]
                     if packs[step["pack"]]["height"] <= book_height]
        self.assertEqual(premature, [],
                         f"Leymor is only reached after {leymor} planned steps; "
                         f"post-boss book-wing packs were visited first: {premature}")

    def test_azure_bosses_keep_sourced_encounter_order(self):
        graph = json.loads((ROOT / "content/dungeons/azure-vault-2515.json").read_text())
        bosses = {29001810, 29001975, 29001910, 29002017}
        actual = [step["pack"] for step in graph["route"] if step["pack"] in bosses]
        self.assertEqual(actual, [29001810, 29001975, 29001910, 29002017])


sys.path.insert(0, str(ROOT / "scripts"))
from freeze_follower_route import plan_route


def fixture():
    points = {1: (0, 0, 0), 2: (10, 0, 0), 3: (20, 0, 0),
              4: (10, 3, 0), 5: (10, 10, 0), 6: (10, 40, 0)}
    packs = [{"id": key, "position": [point[0], point[2]], "height": point[1],
              "spawns": [{"guid": key, "entry": 900 if key == 3 else key}]}
             for key, point in points.items()]
    graph = {"map_id": 99, "entrance": [0, 0], "entrance_height": 0,
             "packs": packs, "edges": [
                 {"from": a, "to": b, "yards": distance}
                 for a, b, distance in [(1, 2, 10), (2, 3, 10), (1, 4, 20),
                                        (4, 3, 20), (1, 5, 30), (5, 3, 30),
                                        (1, 6, 50), (6, 3, 50)]], "teleports": []}
    profile = {"map_id": 99, "sources": [
        {"guid": key, "position": list(point), "eligible": key not in (1, 3),
         "radius": 4 if key == 4 else 1}
        for key, point in points.items()]}
    return graph, profile, {"boss_order": [900], "gate_entries": {}}


class CorridorPlanning(unittest.TestCase):
    def test_native_spheres_intersect_walk_corridor_in_travel_order_not_nearest_pack_order(self):
        graph, profile, requirements = fixture()
        result = plan_route(graph, profile, requirements)
        self.assertEqual([step["pack"] for step in result["route"]], [4, 2, 3])
        self.assertTrue(all(len(step["anchor"]) == 3 for step in result["route"]))
        self.assertEqual(result, plan_route(graph, profile, requirements))

    def test_sourced_gate_is_visited_before_boss_even_outside_direct_corridor(self):
        graph, profile, requirements = fixture()
        requirements["gate_entries"] = {"900": [5]}
        result = plan_route(graph, profile, requirements)
        ids = [step["pack"] for step in result["route"]]
        self.assertIn(5, ids)
        self.assertLess(ids.index(5), ids.index(3))
        self.assertEqual(ids.count(5), 1)

    def test_unattackable_source_never_becomes_a_planned_pull(self):
        graph, profile, requirements = fixture()
        source = next(source for source in profile["sources"] if source["guid"] == 4)
        source["eligible"] = False
        source["radius"] = None
        result = plan_route(graph, profile, requirements)
        self.assertEqual([step["pack"] for step in result["route"]], [2, 3])

    def test_directed_translocation_does_not_expose_the_in_between_space(self):
        graph, profile, requirements = fixture()
        graph["packs"] = [
            {"id": key, "position": [x, 0], "height": 0,
             "spawns": [{"guid": key, "entry": 900 if key == 4 else key}]}
            for key, x in [(1, 0), (2, 10), (3, 1000), (4, 1010), (5, 500)]]
        graph["edges"] = [{"from": 1, "to": 2, "yards": 10},
                          {"from": 3, "to": 4, "yards": 10},
                          {"from": 2, "to": 5, "yards": 490},
                          {"from": 5, "to": 3, "yards": 500}]
        graph["teleports"] = [{"from": 2, "to": 3, "yards": 0}]
        profile["sources"] = [
            {"guid": key, "position": [x, 0, 0], "eligible": key == 5, "radius": 50}
            for key, x in [(1, 0), (2, 10), (3, 1000), (4, 1010), (5, 500)]]
        result = plan_route(graph, profile, requirements)
        self.assertEqual([step["pack"] for step in result["route"]], [4])
        graph["teleports"] = [{"from": 3, "to": 2, "yards": 0}]
        reverse = plan_route(graph, profile, requirements)
        self.assertIn(5, [step["pack"] for step in reverse["route"]])

    def test_disconnected_boss_and_unknown_gate_fail_instead_of_inventing_edges(self):
        graph, profile, requirements = fixture()
        graph["edges"] = []
        with self.assertRaises(ValueError):
            plan_route(graph, profile, requirements)
        graph, profile, requirements = fixture()
        requirements["gate_entries"] = {"900": [123456]}
        with self.assertRaises(ValueError):
            plan_route(graph, profile, requirements)


if __name__ == "__main__":
    unittest.main()
