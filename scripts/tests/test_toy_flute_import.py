"""Native toy summon templates/model weights through the maintained importer."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyFluteImportTests(unittest.TestCase):
    def test_flute_preserves_all_native_model_choices_and_weights(self):
        self.copy_import_and_compare_native_models(71500, [3126, 4440, 35803, 36580], [1.0] * 4)

    def test_scarab_preserves_native_template_and_weighted_model_choices(self):
        self.copy_import_and_compare_native_models(51113, [36611, 36595], [1.0, 2.0])

    def test_bashiok_preserves_native_template_and_source_model(self):
        self.copy_import_and_compare_native_models(86916, [60051], [1.0])

    def copy_import_and_compare_native_models(self, entry, displays, weights):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                models = conn.execute("""SELECT * FROM tdb_creature_template_model
                    WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                self.assertEqual([row[2] for row in models], displays)
                self.assertEqual([row[4] for row in models], weights)
                template = conn.execute("""SELECT entry,name,faction,npcflag,AIName,
                    ScriptName FROM tdb_creature_template WHERE entry=?""", (entry,)).fetchall()
            for _ in range(2):
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall(), models)
                self.assertEqual(conn.execute("""SELECT entry,name,faction,npcflag,AIName,
                    ScriptName FROM content_creature_template WHERE entry=?""", (entry,)).fetchall(),
                                 template)


if __name__ == "__main__":
    unittest.main()
