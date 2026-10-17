"""Katy's authentic template/model through the maintained disposable importer."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyStampwhistleImportTests(unittest.TestCase):
    def test_katy_template_and_mailbox_flags_preserved_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                source = conn.execute("""SELECT entry, name, faction, npcflag, unit_flags,
                    unit_flags2, AIName, ScriptName FROM tdb_creature_template
                    WHERE entry=132969""").fetchall()
                models = conn.execute("""SELECT * FROM tdb_creature_template_model
                    WHERE CreatureID=132969""").fetchall()
                self.assertEqual(source, [(132969, "Katy Stampwhistle", 35, 67108865,
                                          768, 2048, "", "")])
                self.assertEqual(models, [(132969, 0, 68512, 1.0, 1.0, 69497)])
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                self.assertTrue(before <= set(conn.execute("SELECT entry FROM content_creature_template")))
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag,
                    unit_flags, unit_flags2, AIName, ScriptName FROM content_creature_template
                    WHERE entry=132969""").fetchall(), source)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=132969""").fetchall(), models)


if __name__ == "__main__":
    unittest.main()
