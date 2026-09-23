"""Exercise the real importer without touching the development world database."""

import csv
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2  # noqa: E402

PINNED_CSV_ROOT = Path(os.environ.get("DB2_CSV_ROOT", db2.DEFAULT_CSV_ROOT))
PINNED_BUILD_DIR = PINNED_CSV_ROOT / db2.DEFAULT_BUILD


def write_header_only_csvs(csv_dir):
    """Write one header-only CSV per DB2 table, headers taken from db2_schema.sql."""
    csv_dir.mkdir(parents=True)
    schema = sqlite3.connect(":memory:")
    schema.executescript(db2.SCHEMA_SQL.read_text())
    for name in db2.TABLES:
        columns = [row[1] for row in schema.execute(f"PRAGMA table_info({db2.table_name(name)})")]
        with (csv_dir / f"{name}.csv").open("w", newline="") as handle:
            csv.writer(handle).writerow(columns)


def run_db2_import(csv_root, db_path):
    return subprocess.run(
        [sys.executable, str(SCRIPTS / "db2.py"), "import", "--csv-root", str(csv_root), "--db", str(db_path)],
        capture_output=True,
        text=True,
        timeout=600,
        check=False,
    )


class CreatureEventImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        (self.root / "data").mkdir()
        for name in ("import-azerothcore.sh", "world_schema.sql", "db2.py", "db2_schema.sql"):
            shutil.copy2(SCRIPTS / name, scripts / name)
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
        write_header_only_csvs(self.csv / db2.DEFAULT_BUILD)
        self.db = self.root / "data/world.db"

    def run_import(self):
        return subprocess.run(
            [str(self.root / "scripts/import-azerothcore.sh"), str(self.sql)],
            env={**os.environ, "DB2_CSV_ROOT": str(self.csv)},
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

    def test_missing_db2_csv_does_not_destroy_existing_database(self):
        (self.csv / db2.DEFAULT_BUILD / "SkillLineAbility.csv").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SkillLineAbility", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)


class Db2ReimportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.csv_root = root / "csv"
        self.csv_dir = self.csv_root / db2.DEFAULT_BUILD
        write_header_only_csvs(self.csv_dir)
        with (self.csv_dir / "SpellName.csv").open("a", newline="") as handle:
            csv.writer(handle).writerow([35395, "Crusader Strike"])
        self.db = root / "world.db"
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE chr_races (ID INTEGER PRIMARY KEY, Name TEXT)")
            conn.execute("INSERT INTO chr_races VALUES (1, 'Human')")

    def test_reimport_replaces_db2_rows_and_keeps_other_tables(self):
        for _ in range(2):
            result = run_db2_import(self.csv_root, self.db)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute("SELECT ID, Name_lang FROM spell_name").fetchall(), [(35395, "Crusader Strike")])
            self.assertEqual(conn.execute("SELECT * FROM chr_races").fetchall(), [(1, "Human")])
            self.assertEqual(conn.execute("SELECT count(*) FROM db2_build").fetchone()[0], len(db2.TABLES))

    def test_header_mismatch_fails_and_leaves_previous_import_intact(self):
        self.assertEqual(run_db2_import(self.csv_root, self.db).returncode, 0)
        (self.csv_dir / "SpellName.csv").write_text("ID,Name_lang,Extra\n1,Renamed,0\n")
        result = run_db2_import(self.csv_root, self.db)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SpellName.csv header does not match spell_name", result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute("SELECT ID, Name_lang FROM spell_name").fetchall(), [(35395, "Crusader Strike")])


@unittest.skipUnless(PINNED_BUILD_DIR.is_dir(), f"pinned DB2 CSVs not present at {PINNED_BUILD_DIR}")
class PinnedBuildImportTests(unittest.TestCase):
    """Import the real pinned wago.tools exports and check known Retail rows."""

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.temp.name) / "world.db"
        result = run_db2_import(PINNED_CSV_ROOT, cls.db)
        if result.returncode != 0:
            raise AssertionError(result.stdout + result.stderr)
        cls.conn = sqlite3.connect(cls.db)

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.temp.cleanup()

    def one(self, sql, *params):
        return self.conn.execute(sql, params).fetchall()

    def test_every_table_records_the_pinned_build(self):
        self.assertEqual(self.one("SELECT DISTINCT build FROM db2_build"), [("12.1.0.69933",)])
        self.assertEqual(self.one("SELECT count(*) FROM db2_build"), [(len(db2.TABLES),)])

    def test_crusader_strike_is_a_paladin_class_skill_ability(self):
        self.assertEqual(
            self.one(
                "SELECT a.ID, a.ClassMask, s.DisplayName_lang FROM skill_line_ability a"
                " JOIN skill_line s ON s.ID = a.SkillLine WHERE a.Spell = 35395"
            ),
            [(43394, 2, "Paladin")],
        )

    def test_retribution_specialization_belongs_to_paladin(self):
        self.assertEqual(
            self.one("SELECT Name_lang, ClassID, MasterySpellID_0 FROM chr_specialization WHERE ID = 70"),
            [("Retribution", 2, 267316)],
        )

    def test_kick_interrupt_effect_is_68(self):
        self.assertEqual(
            self.one("SELECT EffectIndex, Effect FROM spell_effect WHERE SpellID = 1766 AND DifficultyID = 0"),
            [(0, 68)],
        )

    def test_selfless_healer_stacks_to_four(self):
        self.assertEqual(
            self.one("SELECT CumulativeAura, ProcChance FROM spell_aura_options WHERE SpellID = 114250"),
            [(4, 101)],
        )


if __name__ == "__main__":
    unittest.main()
