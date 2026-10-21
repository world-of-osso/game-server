"""Behavioral checks for frozen boss-path corridor routes."""
import json
from pathlib import Path
import unittest

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


if __name__ == "__main__":
    unittest.main()
