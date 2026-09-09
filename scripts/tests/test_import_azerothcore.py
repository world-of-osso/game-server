"""Exercise the real importer without touching the development world database."""

import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest


class CreatureEventImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        (self.root / "data").mkdir()
        source = Path(__file__).resolve().parents[1]
        for name in ("import-azerothcore.sh", "world_schema.sql"):
            shutil.copy2(source / name, scripts / name)
        self.sql = self.root / "source"
        self.sql.mkdir()
        for name in ("creature_template", "creature_template_model", "waypoint_data"):
            (self.sql / f"{name}.sql").write_text("-- empty fixture\n")
        (self.sql / "creature.sql").write_text(
            "INSERT INTO `creature` (guid,id1,position_z) VALUES\n"
            "(96194,26012,84.2136),(79968,952,113.24),(76,1,30);\n"
        )
        (self.sql / "game_event_creature.sql").write_text(
            "INSERT INTO `game_event_creature` VALUES\n"
            "(31,96194),(-72,76),(31,76);\n"
        )
        self.csv = self.root / "csv"
        self.csv.mkdir()
        for name in (
            "SpellName", "Spell", "SpellMisc", "SpellEffect", "SpellPower",
            "SpellCastTimes", "SpellRange", "SpellCooldowns", "SpellDuration",
        ):
            (self.csv / f"{name}.csv").write_text("ID\n")
        self.db = self.root / "data/world.db"

    def run_import(self):
        return subprocess.run(
            [str(self.root / "scripts/import-azerothcore.sh"), str(self.sql)],
            env={**os.environ, "SPELL_CSV_DIR": str(self.csv)},
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def test_preserves_signed_memberships_and_spawn_coordinates(self):
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            memberships = conn.execute(
                "SELECT eventEntry,guid FROM game_event_creature ORDER BY guid,eventEntry"
            ).fetchall()
            self.assertEqual(memberships, [(-72, 76), (31, 76), (31, 96194)])
            self.assertEqual(conn.execute("SELECT count(*) FROM creature").fetchone()[0], 3)
            height = conn.execute(
                "SELECT position_z FROM creature WHERE guid=79968"
            ).fetchone()[0]
            self.assertEqual(height, 113.24)

    def test_missing_membership_source_does_not_destroy_existing_database(self):
        (self.sql / "game_event_creature.sql").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("game_event_creature.sql", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
