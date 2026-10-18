"""Behavioral scratch overlay tests using captured Forever quests and calibrated NPCs."""

import copy
import json
import os
import sqlite3
import sys
import unittest
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])
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
        self.addCleanup(lambda: self.db.close())

    def apply(self, content=None):
        import zephras_content
        import zephras_creatures

        catalog, decoded = zephras_creatures.load_catalog(SOURCE)
        return zephras_content.apply_overlay(
            self.db, content or self.content, catalog, decoded
        )

    def test_collisions_preserve_every_non_overlay_row_on_import_and_removal(self):
        import zephras_content

        self.apply()
        records = []
        for table, encoded in self.db.execute(
            "SELECT table_name,row_key FROM zephras_row_source ORDER BY table_name,row_key"
        ):
            keys = json.loads(encoded)
            if table == "content_creature_template" and keys != {"entry": 251362}:
                continue
            if any(record[0] == table for record in records):
                continue
            where = " AND ".join(f'"{key}"=?' for key in keys)
            row = self.db.execute(
                f'SELECT * FROM "{table}" WHERE {where}', list(keys.values())
            ).fetchone()
            if table == "content_creature_template":
                columns = [
                    r[1] for r in self.db.execute(f'PRAGMA table_info("{table}")')
                ]
                row = list(row)
                for column, value in (
                    ("name", "Retail sentinel"),
                    ("minlevel", 99),
                    ("maxlevel", 99),
                    ("unit_class", 8),
                    ("HealthModifier", 3.5),
                ):
                    row[columns.index(column)] = value
                row = tuple(row)
            records.append((table, row))
        tag = self.db.execute(
            "SELECT * FROM zephras_row_source WHERE table_name='content_creature_template' AND row_key=?",
            (json.dumps({"entry": 251362}),),
        ).fetchone()
        records.append(("zephras_row_source", (*tag[:4], "retail", tag[5])))
        empty = copy.deepcopy(self.content)
        empty.update(npcs=[], quests=[])
        for table, row in records:
            with self.subTest(table=table):
                self.db.close()
                self.db = scratch_db()
                for ddl in zephras_content.DDL:
                    self.db.execute(ddl)
                placeholders = ",".join("?" for _ in row)
                self.db.execute(f'INSERT INTO "{table}" VALUES ({placeholders})', row)
                self.db.commit()
                before = "\n".join(self.db.iterdump())
                with self.assertRaisesRegex(ValueError, f"collision.*{table}"), self.db:
                    self.apply()
                self.assertEqual("\n".join(self.db.iterdump()), before)
                self.apply(empty)
                self.assertEqual("\n".join(self.db.iterdump()), before)

    def test_regeneration_removes_and_recreates_every_owned_table(self):
        first = self.apply()
        owned_tables = set(first["rows"]) | {"zephras_row_source"}
        before = {
            table: self.db.execute(f'SELECT * FROM "{table}" ORDER BY 1,2').fetchall()
            for table in owned_tables
        }
        empty = copy.deepcopy(self.content)
        empty.update(npcs=[], quests=[])
        self.apply(empty)
        for table in owned_tables:
            rows = self.db.execute(f'SELECT * FROM "{table}"').fetchall()
            self.assertEqual(
                len(rows), 1 if table == "content_creature_template" else 0, table
            )
        self.assertEqual(self.apply(), first)
        for table in owned_tables:
            self.assertEqual(
                self.db.execute(f'SELECT * FROM "{table}" ORDER BY 1,2').fetchall(),
                before[table],
                table,
            )

    def test_missing_npc_stats_are_labelled_estimates_with_captured_source_intact(self):
        self.db.execute(
            "UPDATE content_creature_template SET unit_class=2,HealthModifier=2.5 WHERE entry=823"
        )
        self.apply()
        for npc in self.content["npcs"]:
            with self.subTest(npc=npc["id"]):
                self.assertEqual(
                    self.db.execute(
                        "SELECT unit_class,HealthModifier,minlevel,maxlevel FROM content_creature_template WHERE entry=?",
                        (npc["id"],),
                    ).fetchone(),
                    (1, 1.0, npc["level"]["min"], npc["level"]["max"]),
                )
                url, kind, encoded = self.db.execute(
                    "SELECT source_url,kind,source_json FROM zephras_row_source WHERE table_name='content_creature_template' AND row_key=?",
                    (json.dumps({"entry": npc["id"]}),),
                ).fetchone()
                source = json.loads(encoded)
                estimate = source.pop("stat_estimate")
                self.assertEqual(source, npc)
                self.assertEqual((url, kind), (npc["source"], "reconstructed"))
                self.assertEqual(
                    estimate["fields"], {"unit_class": 1, "HealthModifier": 1.0}
                )
                self.assertEqual(estimate["reference_ids"], [713, 714, 23904])
                self.assertEqual(
                    estimate["basis"],
                    "User-approved coarse estimate from normal-rank exp-0 quest-giver analogues; not authored NPC stats",
                )
                self.assertEqual(
                    estimate["health_curve"],
                    {
                        "table": "creature_classlevelstats",
                        "class": 1,
                        "column": "basehp0",
                        "level_column": "minlevel",
                        "modifier": 1.0,
                        "reference_base_hp": {"1": 42, "5": 102},
                    },
                )
        self.assertEqual(
            self.db.execute(
                "SELECT unit_class,HealthModifier FROM content_creature_template WHERE entry=823"
            ).fetchone(),
            (2, 2.5),
        )

    def test_estimated_stats_preserve_existing_catalog_classification(self):
        import zephras_content
        import zephras_creatures

        catalog, decoded = zephras_creatures.load_catalog(SOURCE)
        record = catalog[2671]
        self.assertNotEqual(record["type"], 0)
        catalog[251362] = record
        zephras_content.apply_overlay(self.db, self.content, catalog, decoded)
        self.assertEqual(
            self.db.execute(
                "SELECT rank,type,family,unit_class,HealthModifier FROM content_creature_template WHERE entry=251362"
            ).fetchone(),
            (record["classification"], record["type"], record["family"], 1, 1.0),
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
        self.assertEqual(
            report["engine_display_ids"], [136536, 136966, 136968, 136971, 139694]
        )
        self.assertEqual(
            self.db.execute(
                "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=251362"
            ).fetchone(),
            (136968,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=251487"
            ).fetchone(),
            (139694,),
        )
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

    def test_page_displays_are_primary_and_full_list_keeps_ungrounded_models(self):
        import zephras_content
        import zephras_creatures

        content = copy.deepcopy(self.content)
        npc = next(n for n in content["npcs"] if n["id"] == 251487)
        npc["coords"][0]["world"]["z"] = None
        npc["coords"][0]["world"]["z_status"] = "missing-root"
        catalog, decoded = zephras_creatures.load_catalog(SOURCE)
        # Actual decoded pet appearance must not replace a captured NPC-page appearance.
        catalog[251362] = catalog[2671]
        report = zephras_content.apply_overlay(self.db, content, catalog, decoded)
        self.assertEqual(
            self.db.execute(
                "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=251362"
            ).fetchone(),
            (136968,),
        )
        self.assertIn(139694, report["engine_display_ids"])
        self.assertEqual(
            self.db.execute(
                "SELECT count(*) FROM content_creature WHERE id1=251487"
            ).fetchone(),
            (0,),
        )

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
                    "--source-dir",
                    str(SOURCE),
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
