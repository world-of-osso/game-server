"""Approved spec-spell bridge, authentic roster/template/display round trip."""

import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
WORLD = Path("/home/osso/Projects/world-of-osso/game-server/data/world.db")


class FollowerProfileTests(unittest.TestCase):
    def test_roster_and_authored_displays_survive_reapplication(self):
        spec = importlib.util.spec_from_file_location(
            "follower_profiles", SCRIPTS / "follower_profiles.py"
        )
        self.assertTrue(
            spec.origin and Path(spec.origin).exists(),
            "approved follower profile importer missing",
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as folder:
            sql = Path(folder) / "profiles.sql"
            module.prepare_profiles(WORLD, sql)
            with (
                sqlite3.connect(WORLD.as_uri() + "?mode=ro", uri=True) as source,
                sqlite3.connect(":memory:") as db,
            ):
                tables = [
                    "content_creature_template",
                    "content_creature_template_model",
                    "content_creature_template_addon",
                    "content_creature_equip_template",
                    "tdb_creature_template",
                    "tdb_creature_template_difficulty",
                    "tdb_creature_template_model",
                    "tdb_creature_template_addon",
                    "tdb_creature_equip_template",
                    "content_tuning_level",
                ]
                for table in tables:
                    ddl = source.execute(
                        "SELECT sql FROM sqlite_master WHERE name=?", (table,)
                    ).fetchone()[0]
                    db.execute(ddl)
                    if table.startswith("tdb_") or table == "content_tuning_level":
                        rows = source.execute("SELECT * FROM " + table).fetchall()
                        db.executemany(
                            "INSERT INTO "
                            + table
                            + " VALUES ("
                            + ",".join("?" for _ in rows[0])
                            + ")",
                            rows,
                        )
                for _ in range(2):
                    db.executescript(sql.read_text())
                    roster = db.execute(
                        "SELECT entry,name,class_id,spec_id,role FROM follower_profiles ORDER BY entry"
                    ).fetchall()
                    self.assertEqual(len(roster), 11)
                    self.assertIn((209057, "Captain Garrick", 2, 66, 0), roster)
                    self.assertIn((215011, "Henry Garrick", 5, 257, 1), roster)
                    self.assertEqual(
                        {row[2] for row in roster}, {1, 2, 3, 5, 7, 8, 9, 10, 11, 13}
                    )
                    self.assertEqual(
                        db.execute(
                            "SELECT count(*) FROM content_creature_template"
                        ).fetchone()[0],
                        11,
                    )
                    self.assertEqual(
                        db.execute(
                            "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=209057 ORDER BY Idx"
                        ).fetchall(),
                        source.execute(
                            "SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=209057 ORDER BY Idx"
                        ).fetchall(),
                    )
                    self.assertEqual(
                        db.execute(
                            "SELECT attack_spell FROM follower_profiles WHERE entry=209059"
                        ).fetchone()[0],
                        133,
                    )
                    self.assertEqual(
                        db.execute(
                            "SELECT heal_spell FROM follower_profiles WHERE entry=209072"
                        ).fetchone()[0],
                        8936,
                    )

    def test_refuses_to_overwrite_database(self):
        spec = importlib.util.spec_from_file_location(
            "follower_profiles", SCRIPTS / "follower_profiles.py"
        )
        self.assertTrue(
            spec.origin and Path(spec.origin).exists(),
            "approved follower profile importer missing",
        )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with self.assertRaisesRegex(ValueError, "database"):
            module.prepare_profiles(WORLD, WORLD)


if __name__ == "__main__":
    unittest.main()
