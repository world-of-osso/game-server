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
            "region_min": [-10254.200195312, -1935.4200439453, -1000000],
            "region_max": [-7939.580078125, 1535.4200439453, 1000000],
        }
        x, y = wh.percent_to_world([41.78, 64.56], bounds)
        self.assertAlmostEqual(x, -9433.99, delta=0.3)
        self.assertAlmostEqual(y, 85.149, delta=0.3)
        with self.assertRaises(ValueError):
            wh.percent_to_world([101, 0], bounds)

    def test_npc_interaction_is_not_kill_credit(self):
        source = (FIXTURES / "quest-93836.html").read_text()
        quest = wh.normalize_quest(wh.parse_page(source, "quest", 93836))
        self.assertEqual(quest["objectives"][0]["type"], "interact")
        self.assertEqual(quest["objectives"][0]["count"], 1)

    def test_kill_credit_and_class_restriction_from_saved_pages(self):
        page = wh.parse_page(
            (FIXTURES / "quest-92461.html").read_text(), "quest", 92461
        )
        objective = wh.normalize_quest(page)["objectives"][0]
        self.assertEqual(
            (objective["type"], objective["target_id"], objective["count"]),
            ("kill", 250873, 8),
        )
        page = wh.parse_page(
            (FIXTURES / "quest-92481.html").read_text(), "quest", 92481
        )
        self.assertEqual(wh.normalize_quest(page)["classes"], [8])

    def test_partial_capture_exposes_zone_metadata_and_missing_pages(self):
        import json
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for kind, entry in [("zone", 16593), ("quest", 92471)]:
                page = wh.parse_page(
                    (FIXTURES / f"{kind}-{entry}.html").read_text(), kind, entry
                )
                wh.save_page(
                    root, page, f"https://www.wowhead.com/forever/{kind}={entry}"
                )
            content = wh.normalize(root)
            npc = next(n for n in content["npcs"] if n["id"] == 251361)
            self.assertEqual(npc["name"], "Rorian the Dayseeker")
            self.assertEqual(npc["coords"][0]["percent"], [42, 23.4])
            self.assertFalse(npc["page_captured"])
            self.assertEqual(npc["displayIds"], [])
            self.assertIn(251361, content["missing_pages"]["npc"])
            self.assertIn(92461, content["missing_pages"]["quest"])
            self.assertTrue((root / "zephras-content.json").exists())
            self.assertEqual(
                json.loads((root / "zephras-content.json").read_text())["map"], 2991
            )

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

    def test_js_loot_keys_and_class_quest_categories(self):
        source = "new Listview({template:'item',id:'skinning',data:[{\"id\":2318,count:157,stack:[1,1],name:\"count: not a key\"}]});"
        page = wh.parse_page(source, "npc", 250868)
        self.assertEqual(page["listviews"][0]["data"][0]["count"], 157)
        self.assertEqual(page["listviews"][0]["data"][0]["name"], "count: not a key")
        source = "new Listview({template:'quest',id:'quests',data:[{\"id\":92481,\"category\":-161}]});"
        self.assertEqual(
            wh.zone_ids(wh.parse_page(source, "zone", 16593), "quest"), [92481]
        )

    def test_unrelated_comment_listview_is_not_database_data(self):
        source = (FIXTURES / "quest-92471.html").read_text()
        source += "new Listview({template:'comment',id:'comments',data:lv_comments0});"
        page = wh.parse_page(source, "quest", 92471)
        self.assertEqual(wh.normalize_quest(page)["id"], 92471)

    def test_reward_markup_carries_spell_and_extra_item(self):
        page = wh.parse_page(
            (FIXTURES / "quest-97257.html").read_text(), "quest", 97257
        )
        rewards = wh.normalize_quest(page)["rewards"]
        self.assertEqual(rewards["spells"], [3599])
        self.assertIn([263412, 1], rewards["items"])
        page = wh.parse_page(
            (FIXTURES / "quest-92470.html").read_text(), "quest", 92470
        )
        rewards = wh.normalize_quest(page)["rewards"]
        self.assertEqual(len(rewards["choice_items"]), 5)
        self.assertNotIn([258882, 1], rewards["items"] or [])

    def test_https_transient_retry_and_forbidden_stop(self):
        import urllib.error
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
        from threading import Thread

        requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append(self.path)
                status = (
                    403
                    if self.path == "/blocked"
                    else 503
                    if len(requests) == 1
                    else 200
                )
                self.send_response(status)
                self.end_headers()
                self.wfile.write(b"captured page")

            def log_message(self, *args):
                return

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = Thread(target=server.serve_forever)
        thread.start()
        try:
            url = f"http://127.0.0.1:{server.server_port}"
            self.assertEqual(wh.fetch_source(url + "/retry"), "captured page")
            with self.assertRaises(urllib.error.HTTPError):
                wh.fetch_source(url + "/blocked")
            self.assertEqual(requests, ["/retry", "/retry", "/blocked"])
        finally:
            server.shutdown()
            thread.join()
            server.server_close()

    def test_cache_skips_fetch_and_preserves_timestamp(self):
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            page = wh.parse_page(
                (FIXTURES / "quest-92471.html").read_text(), "quest", 92471
            )
            original = wh.save_page(
                root, page, "https://www.wowhead.com/forever/quest=92471"
            )
            capture = wh.Capture(root)
            cached = capture.page("quest", 92471)
            self.assertEqual(cached["fetched_at"], original["fetched_at"])
            self.assertEqual(capture.fetched, [])
            self.assertEqual(capture.cached, [["quest", 92471]])

    def test_challenge_is_not_a_cached_page(self):
        with self.assertRaises(ValueError):
            wh.parse_page("<title>Just a moment...</title>", "quest", 92471)


if __name__ == "__main__":
    unittest.main()
