"""Behavioral tests using trimmed, captured Forever page fragments."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wowhead_forever as wh

FIXTURES = Path(__file__).parent / "fixtures/wowhead_forever"


class ForeverTests(unittest.TestCase):
    def test_quest_capture_and_normalization(self):
        page = wh.parse_page(
            (FIXTURES / "quest-92471.html").read_text(), "quest", 92471
        )
        quest = wh.normalize_quest(page)
        self.assertEqual(quest["name"], "Aetheen of the Gales")
        self.assertEqual(quest["races"], [95])
        self.assertEqual(quest["starter_npc_ids"], [251361])
        self.assertEqual(quest["ender_npc_ids"], [251366])
        self.assertEqual(quest["rewards"]["xp"], 35)
        self.assertIn("trial of sorts", quest["text"]["description"])
        self.assertIn("Council of Elders", quest["text"]["completion"])
        self.assertIn(92470, quest["series"]["ids"])

    def test_npc_source_coords_and_display_are_preserved(self):
        page = wh.parse_page((FIXTURES / "npc-251361.html").read_text(), "npc", 251361)
        npc = wh.normalize_npc(page)
        self.assertEqual(npc["displayIds"], [136966])
        self.assertEqual(
            npc["coords"][0], {"zone": 16593, "uiMap": 2521, "percent": [42, 23.4]}
        )
        self.assertEqual(npc["react"], [1, 1])
        self.assertIn(92461, npc["starts_quests"])

    def test_zone_discovery_excludes_foreign_rows(self):
        page = wh.parse_page((FIXTURES / "zone-16593.html").read_text(), "zone", 16593)
        self.assertIn(92471, wh.zone_ids(page, "quest"))
        self.assertIn(251361, wh.zone_ids(page, "npc"))
        self.assertNotIn(6491, wh.zone_ids(page, "npc"))

    def test_inverts_retail_goldshire_mapping(self):
        bounds = {
            "ui_min": [0, 0],
            "ui_max": [1, 1],
            "region_min": [-10254.166, -1935.4166, -10000],
            "region_max": [-7939.583, 1535.4166, 10000],
        }
        x, y = wh.percent_to_world([41.78, 64.56], bounds)
        self.assertAlmostEqual(x, -9433.99, delta=0.3)
        self.assertAlmostEqual(y, 85.149, delta=0.3)
        with self.assertRaises(ValueError):
            wh.percent_to_world([101, 0], bounds)

    def test_terrain_inner_vertex_and_holes(self):
        import struct

        header = bytearray(128)
        struct.pack_into("<3f", header, 0x68, 100, 100, 20)
        heights = [0.0] * 145
        heights[9] = 12
        payload = bytes(header) + b"TVCM" + struct.pack("<I145f", 580, *heights)
        data = b"KNCM" + struct.pack("<I", len(payload)) + payload
        step = (1600 / 3) / 128
        self.assertAlmostEqual(wh.sample_adt(data, 100 - step / 2, 100 - step / 2), 32)
        struct.pack_into("<I", header, 0x3C, 1)
        payload = bytes(header) + payload[128:]
        data = b"KNCM" + struct.pack("<I", len(payload)) + payload
        self.assertIsNone(wh.sample_adt(data, 100 - step / 2, 100 - step / 2))

    def test_unrelated_comment_listview_is_not_database_data(self):
        source = (FIXTURES / "quest-92471.html").read_text()
        source += "new Listview({template:'comment',id:'comments',data:lv_comments0});"
        page = wh.parse_page(source, "quest", 92471)
        self.assertEqual(wh.normalize_quest(page)["id"], 92471)

    def test_challenge_is_not_a_cached_page(self):
        with self.assertRaises(ValueError):
            wh.parse_page("<title>Just a moment...</title>", "quest", 92471)


if __name__ == "__main__":
    unittest.main()
