"""Behavioral scratch overlay tests using captured Forever quests and calibrated NPCs."""

import copy
import json
import sqlite3
import sys
import unittest
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
FIXTURE = SCRIPTS / "tests/fixtures/zephras-content.json"
TABLES = [
    "creature_template",
    "creature_template_model",
    "creature",
    "quest_template",
    "quest_template_addon",
    "quest_offer_reward",
    "quest_request_items",
    "creature_queststarter",
    "creature_questender",
]


def scratch_db():
    db = sqlite3.connect(":memory:")
    db.executescript((SCRIPTS / "world_schema.sql").read_text())
    db.executescript((SCRIPTS / "db2_schema.sql").read_text())
    for table in TABLES:
        ddl = db.execute(
            "SELECT sql FROM sqlite_master WHERE name=?", (table,)
        ).fetchone()[0]
        db.execute(
            ddl.replace(f"CREATE TABLE {table}", f"CREATE TABLE content_{table}", 1)
        )
    db.execute("CREATE TABLE content_item (ID INTEGER PRIMARY KEY)")
    db.execute(
        "INSERT INTO content_creature_template(entry,name) VALUES(823,'Deputy Willem')"
    )
    return db


class ContentOverlayTest(unittest.TestCase):
    def setUp(self):
        self.content = json.loads(FIXTURE.read_text())
        self.db = scratch_db()
        self.addCleanup(self.db.close)

    def apply(self, content=None):
        import zephras_content
        import zephras_creatures

        catalog, decoded = zephras_creatures.load_catalog()
        return zephras_content.apply_overlay(
            self.db, content or self.content, catalog, decoded
        )

    def test_creatures_and_quests_are_server_readable_with_row_provenance(self):
        report = self.apply()
        self.assertEqual(
            self.db.execute(
                "SELECT name FROM content_creature_template WHERE entry=823"
            ).fetchone(),
            ("Deputy Willem",),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT name,minlevel,maxlevel,npcflag FROM content_creature_template WHERE entry=251362"
            ).fetchone(),
            ("Ailee Farheart", 1, 1, 2),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT map,position_x,position_y,position_z FROM content_creature WHERE id1=251362"
            ).fetchone(),
            (2991, 4088.500790283203, 1850.5, 976.1734898252165),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT quest FROM content_creature_queststarter WHERE id=251362"
            ).fetchall(),
            [(92460,)],
        )
        self.assertEqual(
            self.db.execute(
                "SELECT id FROM content_creature_questender WHERE quest=92460"
            ).fetchall(),
            [(251361,)],
        )
        self.assertEqual(
            self.db.execute(
                "SELECT MinLevel,AllowableRaces,LogTitle FROM content_quest_template WHERE ID=92460"
            ).fetchone(),
            (1, 1 << 32, "Coming of Age"),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT RequiredNpcOrGo1,RequiredNpcOrGoCount1 FROM content_quest_template WHERE ID=92461"
            ).fetchone(),
            (250873, 8),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT reward_xp,completion_blocked FROM zephras_quest_meta WHERE quest_id=92460"
            ).fetchone(),
            (40, 0),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT completion_blocked FROM zephras_quest_meta WHERE quest_id=92598"
            ).fetchone(),
            (1,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT AllowableRaces FROM content_quest_template WHERE ID=92598"
            ).fetchone(),
            (0,),
        )
        self.assertEqual(report["display_coverage"]["db2_npcs"], 0)
        self.assertEqual(report["engine_display_ids"], [136966, 136971])
        self.assertEqual(report["rows"]["content_quest_template"], 4)
        tags = self.db.execute(
            "SELECT scrape,source_url,kind FROM zephras_row_source"
        ).fetchall()
        self.assertTrue(tags)
        self.assertTrue(
            all(
                scrape == "forever-zephras-20261004"
                and url.startswith("https://www.wowhead.com/forever/")
                and kind == "reconstructed"
                for scrape, url, kind in tags
            )
        )
        self.assertEqual(len(tags), sum(report["rows"].values()))

    def test_unknown_z_and_unknown_levels_do_not_create_spawns(self):
        content = copy.deepcopy(self.content)
        npc = next(n for n in content["npcs"] if n["id"] == 251362)
        npc["coords"].append(
            {"world": {"x": 1, "y": 2, "z": None, "z_status": "missing_root"}}
        )
        bad_level = next(n for n in content["npcs"] if n["id"] == 250873)
        bad_level["level"] = {"min": 9999, "max": 9999}
        report = self.apply(content)
        self.assertEqual(
            self.db.execute(
                "SELECT count(*) FROM content_creature WHERE id1=251362"
            ).fetchone(),
            (1,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT count(*) FROM content_creature_template WHERE entry=250873"
            ).fetchone(),
            (0,),
        )
        self.assertEqual(report["skipped_positions"]["missing_root"], 1)
        self.assertEqual(report["skipped_npcs"], [250873])

    def test_item_rewards_require_retail_and_runtime_item_rows(self):
        report = self.apply()
        self.assertEqual(
            self.db.execute(
                "SELECT RewardItem1 FROM content_quest_template WHERE ID=92460"
            ).fetchone(),
            (0,),
        )
        self.assertIn(
            {
                "quest": 92460,
                "kind": "item",
                "id": 264908,
                "reason": "absent from retail/runtime item tables",
            },
            report["dropped_rewards"],
        )
        self.db.execute("INSERT INTO item_sparse(ID) VALUES(264908)")
        self.db.execute("INSERT INTO content_item(ID) VALUES(264908)")
        self.apply()
        self.assertEqual(
            self.db.execute(
                "SELECT RewardItem1,RewardAmount1 FROM content_quest_template WHERE ID=92460"
            ).fetchone(),
            (264908, 1),
        )

    def test_reapply_is_deterministic_and_removes_old_reconstructed_rows_only(self):
        first = self.apply()
        second = self.apply()
        self.assertEqual(first, second)
        self.assertEqual(
            self.db.execute("SELECT count(*) FROM content_quest_template").fetchone(),
            (4,),
        )
        reduced = copy.deepcopy(self.content)
        reduced["quests"] = [q for q in reduced["quests"] if q["id"] != 92465]
        self.apply(reduced)
        self.assertEqual(
            self.db.execute(
                "SELECT count(*) FROM content_quest_template WHERE ID=92465"
            ).fetchone(),
            (0,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT name FROM content_creature_template WHERE entry=823"
            ).fetchone(),
            ("Deputy Willem",),
        )

    def test_missing_giver_template_is_reported_not_silently_linked(self):
        content = copy.deepcopy(self.content)
        next(n for n in content["npcs"] if n["id"] == 251362)["level"] = None
        report = self.apply(content)
        self.assertIn(
            {"quest": 92460, "npc": 251362, "role": "starter"},
            report.get("unresolved_quest_links", []),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT count(*) FROM content_creature_queststarter WHERE id=251362"
            ).fetchone(),
            (0,),
        )

    def test_failed_cli_import_rolls_back_schema_and_content_rows(self):
        import subprocess
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "scratch.db"
            self.db.commit()
            with closing(sqlite3.connect(path)) as saved:
                self.db.backup(saved)
            content = copy.deepcopy(self.content)
            next(n for n in content["npcs"] if n["id"] == 251362)["coords"][0]["world"][
                "x"
            ] = float("nan")
            source = root / "invalid-content.json"
            source.write_text(json.dumps(content))
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "zephras_content.py"),
                    "--content",
                    str(source),
                    "--db",
                    str(path),
                    "--report",
                    str(root / "report.json"),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("non-finite", result.stderr)
            with closing(sqlite3.connect(path)) as saved:
                self.assertEqual(
                    saved.execute(
                        "SELECT entry FROM content_creature_template"
                    ).fetchall(),
                    [(823,)],
                )
                self.assertEqual(
                    saved.execute(
                        "SELECT count(*) FROM sqlite_master WHERE name LIKE 'zephras_%'"
                    ).fetchone(),
                    (0,),
                )

    def test_unknown_interactions_are_not_imported_as_kill_objectives(self):
        self.apply()
        self.assertEqual(
            self.db.execute(
                "SELECT RequiredNpcOrGo1,RequiredNpcOrGo2 FROM content_quest_template WHERE ID=92465"
            ).fetchone(),
            (251160, 0),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT completion_blocked FROM zephras_quest_meta WHERE quest_id=92465"
            ).fetchone(),
            (1,),
        )


if __name__ == "__main__":
    unittest.main()
