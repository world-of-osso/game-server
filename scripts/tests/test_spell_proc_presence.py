"""Pinned-source provenance and atomic refresh survive the complete-row import."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

from test_spell_proc_values import COLUMNS

SCRIPT = Path(__file__).resolve().parents[1] / "import_spell_proc_presence.py"

# Exact audited rows4341/48484 and ordered update1259886.
ROWS = [
    dict(zip(COLUMNS, (4341, 0, 0, 0, 0, 0, 0, 4096, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0))),
    dict(zip(COLUMNS, (48484, 1, 7, 4096, 0, 0, 0, 16, 0, 1, 1, 1027, 0, 0, 0, 100, 0, 0))),
    dict(zip(COLUMNS, (1259886, 0, 5, 0, 272, 0, 0, 0, 0, 4, 1, 0, 0, 0, 0, 0, 0, 0))),
]


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
        base.write_text("INSERT INTO spell_proc VALUES " + ",".join(
            "(" + ",".join(str(row[column]) for column in COLUMNS) + ")" for row in ROWS[:2]
        ) + ";\n")
        patch = self.root / "patch.sql"
        patch.write_text("INSERT INTO spell_proc VALUES (" + ",".join(
            str(ROWS[2][column]) for column in COLUMNS
        ) + ");\n")
        self.audit = {
            "base": {"path": str(base), "sha256": self.digest(base), "rows": 2},
            "patches": [{"source": str(patch), "sha256": self.digest(patch),
                         "statements": [patch.read_text().strip()]}],
            "effective_rows": 3,
            "rows": [dict(row) for row in ROWS],
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

    def test_refresh_replaces_stale_values_for_retained_rows(self):
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY, "
                         "Chance REAL DEFAULT 0)")
            conn.executemany("INSERT INTO tdb_spell_proc VALUES (?, 99)", [(4341,), (14190,)])
        result = self.import_presence()
        self.assertEqual(result.returncode, 0, result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute(
                "SELECT SpellId, Chance FROM tdb_spell_proc ORDER BY SpellId").fetchall(),
                [(4341, 0), (48484, 100), (1259886, 0)])

    def test_database_insert_failure_rolls_back_table_replacement(self):
        spec = importlib.util.spec_from_file_location("spell_proc_import", SCRIPT)
        importer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(importer)
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY, Chance REAL)")
            conn.executemany("INSERT INTO tdb_spell_proc VALUES (?, 99)", [(4341,), (14190,)])
            conn.commit()

            def deny_proc_inserts(action, table, *_):
                if action == sqlite3.SQLITE_INSERT and table == "tdb_spell_proc":
                    return sqlite3.SQLITE_DENY
                return sqlite3.SQLITE_OK

            conn.set_authorizer(deny_proc_inserts)
            with self.assertRaises(sqlite3.DatabaseError):
                importer.publish_proc_values(conn, importer.collect_proc_rows(self.audit))
            conn.set_authorizer(None)
            self.assertEqual(conn.execute(
                "SELECT SpellId, Chance FROM tdb_spell_proc ORDER BY SpellId").fetchall(),
                [(4341, 99), (14190, 99)])

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
