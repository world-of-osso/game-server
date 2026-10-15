"""Authentic shrine import through the maintained CLI, on a disposable DB."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyShrineImportTests(unittest.TestCase):
    def test_shrine_preserves_source_model_and_idempotent_template(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                models = conn.execute("""SELECT CreatureID, Idx, CreatureDisplayID,
                    DisplayScale, Probability, VerifiedBuild
                    FROM tdb_creature_template_model WHERE CreatureID = 246037""").fetchall()
                template = conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM tdb_creature_template WHERE entry = 246037""").fetchall()
                self.assertEqual(models[0][2:5], (129958, 1.0, 1.0))
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID = 246037""").fetchall(), models)
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM content_creature_template WHERE entry = 246037""").fetchall(),
                                 template)


if __name__ == "__main__":
    unittest.main()
