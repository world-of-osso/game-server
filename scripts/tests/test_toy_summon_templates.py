"""Exercise the real TDB1210-to-runtime import on a disposable database copy."""
import shutil
import sqlite3
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENTRIES = (65298, 65387)  # Silversage Incense / Totem of Harmony.


class ToySceneryImportTests(unittest.TestCase):
    def test_source_models_templates_and_idempotent_import(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with sqlite3.connect(path) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                source_models = conn.execute("""SELECT CreatureID, Idx, CreatureDisplayID,
                    DisplayScale, Probability, VerifiedBuild FROM tdb_creature_template_model
                    WHERE CreatureID IN (?, ?) ORDER BY CreatureID, Idx""", ENTRIES).fetchall()
            for _ in range(2):  # Rerunning the actual CLI must not duplicate rows.
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with sqlite3.connect(path) as conn:
                after = set(conn.execute("SELECT entry FROM content_creature_template"))
                self.assertEqual(after - before, {(entry,) for entry in ENTRIES} - before)
                actual_models = conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID IN (?, ?) ORDER BY CreatureID, Idx""", ENTRIES).fetchall()
                self.assertEqual(actual_models, source_models)
                actual = conn.execute("""SELECT entry, name, faction, npcflag, AIName, ScriptName
                    FROM content_creature_template WHERE entry IN (?, ?) ORDER BY entry""", ENTRIES).fetchall()
                source = conn.execute("""SELECT entry, name, faction, npcflag, AIName, ScriptName
                    FROM tdb_creature_template WHERE entry IN (?, ?) ORDER BY entry""", ENTRIES).fetchall()
                self.assertEqual(actual, source)
                self.assertEqual(len(actual), 2)

    def test_missing_source_models_rolls_back_new_templates(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with sqlite3.connect(path) as conn:
                conn.execute("DELETE FROM content_creature_template WHERE entry IN (?, ?)", ENTRIES)
                conn.execute("DELETE FROM content_creature_template_model WHERE CreatureID IN (?, ?)", ENTRIES)
                conn.execute("DELETE FROM tdb_creature_template_model WHERE CreatureID = 65387")
            result = subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                     "--db", str(path)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing authentic TDB templates/models: [65387]", result.stderr)
            with sqlite3.connect(path) as conn:
                self.assertEqual(conn.execute("""SELECT count(*) FROM content_creature_template
                    WHERE entry IN (?, ?)""", ENTRIES).fetchone()[0], 0)
                self.assertEqual(conn.execute("""SELECT count(*) FROM content_creature_template_model
                    WHERE CreatureID IN (?, ?)""", ENTRIES).fetchone()[0], 0)


if __name__ == "__main__":
    unittest.main()
