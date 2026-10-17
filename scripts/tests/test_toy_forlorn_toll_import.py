"""Forlorn Toll85 preserves the authentic ten-choice Wandering Soul identity."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
DISPLAYS = [81277, 88427, 88428, 88429, 81278, 88430, 88431, 88432, 81205, 88433]


class ToyForlornTollImportTests(unittest.TestCase):
    def test_source_soul_and_all_ten_models_import_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                source = conn.execute("""SELECT entry,name,faction,npcflag,unit_flags,
                    unit_flags2,AIName,ScriptName FROM tdb_creature_template WHERE entry=143222""").fetchall()
                models = conn.execute("""SELECT * FROM tdb_creature_template_model
                    WHERE CreatureID=143222 ORDER BY Idx""").fetchall()
                self.assertEqual(source, [(143222, "Wandering Soul", 35, 0, 0, 0, "", "")])
                self.assertEqual(models, [
                    (143222, index, display, 1.0, 1.0, 69497)
                    for index, display in enumerate(DISPLAYS)
                ])
            for _ in range(2):
                subprocess.run([
                    "python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                    "--db", str(path),
                ], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                self.assertTrue(before <= set(conn.execute("SELECT entry FROM content_creature_template")))
                self.assertEqual(conn.execute("""SELECT entry,name,faction,npcflag,unit_flags,
                    unit_flags2,AIName,ScriptName FROM content_creature_template WHERE entry=143222""").fetchall(), source)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=143222 ORDER BY Idx""").fetchall(), models)


if __name__ == "__main__":
    unittest.main()
