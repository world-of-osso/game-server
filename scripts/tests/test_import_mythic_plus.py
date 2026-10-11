"""Targeted offline import tests using the extracted local Mainline DB2s."""

import importlib.util
import os
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "import_mythic_plus", SCRIPTS / "import_mythic_plus.py"
)
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)
SOURCE = Path(os.environ["MYTHIC_PLUS_DB2_SOURCE"])
WORLD = Path(os.environ["MYTHIC_PLUS_WORLD_SOURCE"])


class MythicPlusImportTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.copy = Path(self.temp.name) / "world.sqlite"
        with closing(sqlite3.connect(self.copy)) as conn, conn:
            conn.execute("CREATE TABLE sentinel (id INTEGER PRIMARY KEY, value TEXT)")
            conn.execute("INSERT INTO sentinel VALUES (7, 'preserve')")

    def test_local_catalog_preserves_timers_affixes_rewards_and_sources(self):
        report = IMPORTER.import_catalog(self.copy, SOURCE)
        self.assertIn("map_challenge_mode", report["tables"])
        with closing(sqlite3.connect(self.copy)) as conn:
            self.assertEqual(
                conn.execute("SELECT value FROM sentinel WHERE id=7").fetchone(),
                ("preserve",),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT Name_lang, MapID, CriteriaCount_0, CriteriaCount_1, CriteriaCount_2 FROM map_challenge_mode WHERE ID=401"
                ).fetchone(),
                ("The Azure Vault", 2515, 2250, 1800, 1350),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT Name_lang FROM keystone_affix WHERE ID=165"
                ).fetchone(),
                ("Lindormi's Guidance",),
            )
            self.assertIn(
                "15 seconds",
                conn.execute(
                    "SELECT Description_lang FROM keystone_affix WHERE ID=147"
                ).fetchone()[0],
            )
            self.assertEqual(
                conn.execute(
                    "SELECT CriteriatreeID, ScenarioID FROM scenario_step WHERE ID=5383"
                ).fetchone(),
                (138749, 2133),
            )
            self.assertGreater(
                conn.execute(
                    "SELECT COUNT(*) FROM challenge_mode_item_bonus_override"
                ).fetchone()[0],
                700,
            )
            self.assertEqual(
                conn.execute(
                    "SELECT COUNT(DISTINCT build) FROM mythic_plus_import_source"
                ).fetchone(),
                (1,),
            )
        self.assertEqual(report["tables"]["map_challenge_mode"]["rows"], 87)
        self.assertEqual(report["tables"]["keystone_affix"]["rows"], 50)
        self.assertEqual(report, IMPORTER.import_catalog(self.copy, SOURCE))

    def test_corrupt_layout_keeps_existing_catalog_unchanged(self):
        IMPORTER.import_catalog(self.copy, SOURCE)
        bad = Path(self.temp.name) / "bad"
        bad.mkdir()
        for fdid in IMPORTER.TABLES:
            data = bytearray((SOURCE / f"{fdid}.db2").read_bytes())
            if fdid == 801709:
                data[156:160] = bytes(4)
            (bad / f"{fdid}.db2").write_bytes(data)
        with self.assertRaisesRegex(ValueError, "layout"):
            IMPORTER.import_catalog(self.copy, bad)
        with closing(sqlite3.connect(self.copy)) as conn:
            self.assertEqual(
                conn.execute("SELECT COUNT(*) FROM map_challenge_mode").fetchone(),
                (87,),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT CriteriaCount_0 FROM map_challenge_mode WHERE ID=401"
                ).fetchone(),
                (2250,),
            )

    def test_sql_failure_rolls_back_every_table(self):
        IMPORTER.import_catalog(self.copy, SOURCE)
        with closing(sqlite3.connect(self.copy)) as conn, conn:
            conn.execute(
                "UPDATE map_challenge_mode SET CriteriaCount_0=99 WHERE ID=401"
            )
            conn.execute(
                "CREATE TRIGGER deny_affix_replacement BEFORE DELETE ON keystone_affix BEGIN SELECT RAISE(ABORT, 'test copy is locked'); END"
            )
        with self.assertRaisesRegex(sqlite3.IntegrityError, "test copy is locked"):
            IMPORTER.import_catalog(self.copy, SOURCE)
        with closing(sqlite3.connect(self.copy)) as conn:
            self.assertEqual(
                conn.execute(
                    "SELECT CriteriaCount_0 FROM map_challenge_mode WHERE ID=401"
                ).fetchone(),
                (99,),
            )
            self.assertEqual(
                conn.execute("SELECT COUNT(*) FROM keystone_affix").fetchone(), (50,)
            )

    def test_refuses_linked_destination_and_canonical_database(self):
        link = Path(self.temp.name) / "link.sqlite"
        link.symlink_to(self.copy)
        with self.assertRaisesRegex(ValueError, "symlink"):
            IMPORTER.import_catalog(link, SOURCE)
        hardlink = Path(self.temp.name) / "hard.sqlite"
        os.link(self.copy, hardlink)
        with self.assertRaisesRegex(ValueError, "hardlink"):
            IMPORTER.import_catalog(hardlink, SOURCE)
        with self.assertRaisesRegex(ValueError, "canonical"):
            IMPORTER.import_catalog(WORLD, SOURCE)

    def test_azure_vault_authored_population_and_forces(self):
        with closing(sqlite3.connect(f"file:{WORLD}?mode=ro", uri=True)) as conn:
            self.assertEqual(
                conn.execute(
                    "SELECT Amount FROM criteria_tree WHERE ID=138754"
                ).fetchone(),
                (410,),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT t.Amount FROM criteria_tree t JOIN criteria c ON c.ID=t.CriteriaID WHERE t.Parent=138754 AND c.Type=0 AND c.Asset=187160"
                ).fetchone(),
                (5,),
            )
            self.assertGreater(
                conn.execute(
                    "SELECT COUNT(*) FROM content_creature WHERE map=2515 AND ',' || spawnDifficulties || ',' LIKE '%,8,%'"
                ).fetchone()[0],
                0,
            )
            self.assertGreater(
                conn.execute(
                    "SELECT COUNT(*) FROM content_gameobject g JOIN content_gameobject_template t ON t.entry=g.id WHERE g.map=2515 AND t.type=49 AND ',' || g.spawnDifficulties || ',' LIKE '%,8,%'"
                ).fetchone()[0],
                0,
            )


if __name__ == "__main__":
    unittest.main()
