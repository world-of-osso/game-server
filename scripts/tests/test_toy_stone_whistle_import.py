"""Source Stone Whistle import through the real CLI on a disposable database."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyStoneWhistleImportTests(unittest.TestCase):
    def test_source_petrified_piper_model_and_template_are_preserved_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                models = conn.execute("""SELECT CreatureID, Idx, CreatureDisplayID,
                    DisplayScale, Probability, VerifiedBuild FROM tdb_creature_template_model
                    WHERE CreatureID = 98300""").fetchall()
                template = conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM tdb_creature_template WHERE entry = 98300""").fetchall()
                self.assertEqual(models, [(98300, 0, 71056, 1.0, 1.0, 69497)])
                self.assertEqual(template[0][1], "Petrified Sand Piper")
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                after = set(conn.execute("SELECT entry FROM content_creature_template"))
                self.assertIn((98300,), after)
                self.assertTrue(before <= after)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID = 98300""").fetchall(), models)
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag, AIName,
                    ScriptName FROM content_creature_template WHERE entry = 98300""").fetchall(),
                                 template)


if __name__ == "__main__":
    unittest.main()
