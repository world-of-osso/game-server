"""Frozen graph coordinates must preserve actual source heights, including stacked floors."""
import json
import sqlite3
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class NativeGraphHeights(unittest.TestCase):
    def test_both_frozen_graphs_preserve_native_leader_and_entrance_heights(self):
        db = sqlite3.connect((ROOT / "data/world.db").resolve().as_uri() + "?mode=ro", uri=True)
        self.addCleanup(db.close)
        for filename, safe_loc in [("stockade-34.json", 3599), ("azure-vault-2515.json", 100022)]:
            with self.subTest(graph=filename):
                graph = json.loads((ROOT / "content/dungeons" / filename).read_text())
                entrance = db.execute("SELECT LocZ FROM tdb_world_safe_locs WHERE ID=?", (safe_loc,)).fetchone()[0]
                self.assertAlmostEqual(graph["entrance_height"], entrance, places=3)
                for pack in graph["packs"]:
                    if pack["id"] == 7116:
                        self.assertAlmostEqual(pack["height"], 157.716995, places=3)
                        continue
                    native = db.execute("SELECT position_z FROM content_creature WHERE guid=?", (pack["id"],)).fetchone()
                    self.assertIsNotNone(native, f"real source leader{pack['id']}")
                    self.assertAlmostEqual(pack["height"], native[0], places=3)


if __name__ == "__main__":
    unittest.main()
