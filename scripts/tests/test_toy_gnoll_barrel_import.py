"""Gnoll Barrel81 authentic TDB identities through the maintained additive importer."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ToyGnollBarrelImportTests(unittest.TestCase):
    def test_source_templates_and_models_preserved_idempotently(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                source = conn.execute("""SELECT entry, name, faction, npcflag,
                    unit_flags, unit_flags2, AIName, ScriptName FROM tdb_creature_template
                    WHERE entry IN (143119,143120) ORDER BY entry""").fetchall()
                models = conn.execute("""SELECT * FROM tdb_creature_template_model
                    WHERE CreatureID IN (143119,143120) ORDER BY CreatureID,Idx""").fetchall()
                self.assertEqual(source, [
                    (143119, "Gnoll Target Dummy", 35, 0, 0, 0, "", ""),
                    (143120, "Target Barrel", 35, 0, 0, 0, "", ""),
                ])
                self.assertEqual(models, [
                    (143119, 0, 38854, 1.0, 1.0, 69497),
                    (143120, 0, 38857, 1.0, 1.0, 69497),
                ])
            for _ in range(2):
                subprocess.run([
                    "python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                    "--db", str(path),
                ], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn:
                self.assertTrue(before <= set(conn.execute("SELECT entry FROM content_creature_template")))
                self.assertEqual(conn.execute("""SELECT entry, name, faction, npcflag,
                    unit_flags, unit_flags2, AIName, ScriptName FROM content_creature_template
                    WHERE entry IN (143119,143120) ORDER BY entry""").fetchall(), source)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID IN (143119,143120) ORDER BY CreatureID,Idx""").fetchall(), models)


if __name__ == "__main__":
    unittest.main()
