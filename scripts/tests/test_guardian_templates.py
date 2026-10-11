"""Real TDB-to-runtime preparation; never writes the source world.db."""
from contextlib import closing
import shutil
import sqlite3
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 103673


class GuardianTemplateTests(unittest.TestCase):
    def run_import(self, path):
        return subprocess.run(
            ["python3", str(ROOT / "scripts/add_guardian_templates.py"), "--db", str(path)],
            capture_output=True, text=True,
        )

    def test_darkglare_preserves_native_template_and_model_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "prepared.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            for _ in range(2):
                result = self.run_import(path)
                self.assertEqual(result.returncode, 0, result.stderr)
            with closing(sqlite3.connect(path)) as conn:
                source = conn.execute("SELECT entry,name,type,BaseAttackTime,AIName,ScriptName FROM tdb_creature_template WHERE entry=?", (ENTRY,)).fetchone()
                actual = conn.execute("SELECT entry,name,type,BaseAttackTime,AIName,ScriptName FROM content_creature_template WHERE entry=?", (ENTRY,)).fetchone()
                self.assertEqual(source, (103673, "Darkglare", 3, 0, "", ""))
                self.assertEqual(actual, source)
                models = conn.execute("SELECT * FROM tdb_creature_template_model WHERE CreatureID=?", (ENTRY,)).fetchall()
                self.assertEqual(models, [(103673, 0, 116130, 1.0, 0.0, 69497)])
                self.assertEqual(conn.execute("SELECT * FROM content_creature_template_model WHERE CreatureID=?", (ENTRY,)).fetchall(), models)

    def test_missing_native_model_rolls_back(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "prepared.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                conn.execute("DELETE FROM content_creature_template WHERE entry=?", (ENTRY,))
                conn.execute("DELETE FROM content_creature_template_model WHERE CreatureID=?", (ENTRY,))
                conn.execute("DELETE FROM tdb_creature_template_model WHERE CreatureID=?", (ENTRY,))
            result = self.run_import(path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing authentic guardian template/model: 103673", result.stderr)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute("SELECT count(*) FROM content_creature_template WHERE entry=?", (ENTRY,)).fetchone()[0], 0)

    def test_live_db_refused_before_connect(self):
        result = self.run_import(ROOT / "data/world.db")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("prepare a database copy", result.stderr)


if __name__ == "__main__":
    unittest.main()
