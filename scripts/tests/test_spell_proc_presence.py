"""Audited public TC rows become explicit provenance, never proc values."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "import_spell_proc_presence.py"


class SpellProcPresenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.db = self.root / "world.db"
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE untouched (Build INTEGER)")
            conn.execute("INSERT INTO untouched VALUES (70448)")
        base = self.root / "base.sql"
        base.write_text("INSERT INTO spell_proc (SpellId) VALUES (4341),(48484);\n")
        patch = self.root / "patch.sql"
        patch.write_text("INSERT INTO spell_proc (SpellId) VALUES (1259886);\n")
        self.audit = {
            "base": {"path": str(base), "sha256": self.digest(base), "rows": 2},
            "patches": [{"source": str(patch), "sha256": self.digest(patch),
                         "statements": [patch.read_text().strip()]}],
            "effective_rows": 3,
            "rows": [{"SpellId": 4341}, {"SpellId": 48484}, {"SpellId": 1259886}],
        }
        self.manifest = self.root / "audit.json"

    @staticmethod
    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def import_presence(self):
        self.manifest.write_text(json.dumps(self.audit))
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--manifest", str(self.manifest),
             "--db", str(self.db)], capture_output=True, text=True, check=False)

    def test_imports_exact_effective_ids_and_is_idempotent(self):
        for _ in range(2):
            result = self.import_presence()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["rows"], 3)
            with sqlite3.connect(self.db) as conn:
                self.assertEqual(conn.execute(
                    "SELECT SpellId FROM tdb_spell_proc ORDER BY SpellId").fetchall(),
                    [(4341,), (48484,), (1259886,)])
                self.assertEqual(conn.execute("SELECT * FROM untouched").fetchall(), [(70448,)])

    def test_refresh_preserves_later_value_columns_for_retained_rows(self):
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY, "
                         "Chance REAL DEFAULT 0)")
            conn.executemany("INSERT INTO tdb_spell_proc VALUES (?, 0)", [(4341,), (14190,)])
        result = self.import_presence()
        self.assertEqual(result.returncode, 0, result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute(
                "SELECT SpellId, Chance FROM tdb_spell_proc ORDER BY SpellId").fetchall(),
                [(4341, 0), (48484, 0), (1259886, 0)])

    def test_value_column_constraint_failure_rolls_back_removed_rows(self):
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY, "
                         "Chance REAL NOT NULL)")
            conn.executemany("INSERT INTO tdb_spell_proc VALUES (?, 0)", [(4341,), (14190,)])
        result = self.import_presence()
        self.assertNotEqual(result.returncode, 0)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute(
                "SELECT SpellId, Chance FROM tdb_spell_proc ORDER BY SpellId").fetchall(),
                [(4341, 0), (14190, 0)])

    def test_changed_pinned_source_rejects_before_database_mutation(self):
        Path(self.audit["patches"][0]["source"]).write_text("-- changed after audit\n")
        result = self.import_presence()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA-256", result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute(
                "SELECT name FROM sqlite_master WHERE name='tdb_spell_proc'").fetchall(), [])
            self.assertEqual(conn.execute("SELECT * FROM untouched").fetchall(), [(70448,)])

    def test_duplicate_or_incomplete_rows_reject_before_mutation(self):
        for rows in [[{"SpellId": 4341}] * 3, [{"SpellId": 48484}]]:
            with self.subTest(rows=rows):
                self.audit["rows"] = rows
                result = self.import_presence()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("audited rows", result.stderr)
                with sqlite3.connect(self.db) as conn:
                    self.assertEqual(conn.execute(
                        "SELECT name FROM sqlite_master WHERE name='tdb_spell_proc'").fetchall(), [])


if __name__ == "__main__":
    unittest.main()
