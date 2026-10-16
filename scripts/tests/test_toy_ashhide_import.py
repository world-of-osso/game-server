"""Source104329 needs its real MOUNTED78 creature despite having no summon28."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyAshhideImportTests(unittest.TestCase):
    def test_authentic_mount_body_is_imported_without_rewriting_source_model(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                models = conn.execute("""SELECT * FROM tdb_creature_template_model
                    WHERE CreatureID = 73689""").fetchall()
                self.assertEqual(models, [(73689, 0, 51484, 1.0, 0.0, 69497)])
                template = conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM tdb_creature_template WHERE entry = 73689""").fetchall()
                self.assertEqual(template, [(73689, "Ashhide Mushan Beast", 35, 0, "", "")])
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                after = set(conn.execute("SELECT entry FROM content_creature_template"))
                self.assertIn((73689,), after)
                self.assertTrue(before <= after)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID = 73689""").fetchall(), models)
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM content_creature_template WHERE entry = 73689""").fetchall(),
                                 template)


if __name__ == "__main__":
    unittest.main()
