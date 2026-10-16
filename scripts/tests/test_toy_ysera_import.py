"""Source Ysera admission through the maintained importer on disposable data."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyYseraImportTests(unittest.TestCase):
    def test_source_vision_template_and_display_are_preserved_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                models = conn.execute("""SELECT CreatureID, Idx, CreatureDisplayID,
                    DisplayScale, Probability, VerifiedBuild FROM tdb_creature_template_model
                    WHERE CreatureID = 99518""").fetchall()
                template = conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM tdb_creature_template WHERE entry = 99518""").fetchall()
                self.assertEqual(models, [(99518, 0, 66207, 1.0, 1.0, 69497)])
                self.assertEqual(template[0][1], "Vision of Ysera")
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                after = set(conn.execute("SELECT entry FROM content_creature_template"))
                self.assertEqual(conn.execute("""SELECT count(*) FROM content_creature_template
                    WHERE entry = 99518""").fetchone()[0], 1)
                self.assertTrue(before <= after)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID = 99518""").fetchall(), models)
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM content_creature_template WHERE entry = 99518""").fetchall(),
                                 template)


if __name__ == "__main__":
    unittest.main()
