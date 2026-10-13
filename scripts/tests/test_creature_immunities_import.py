"""Process-boundary import proof against authentic TDB1210 rows, owned SQLite only."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/import-creature-immunities.py"
SOURCE = ROOT / "data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql"
FINGERPRINT = "b3794d194a12d7f7b4122dccb156987d2579394b6d568f11de2e9fc474c71ecb"


class CreatureImmunitiesImportTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.db = Path(self.directory.name) / "owned.db"
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE unrelated (ID INTEGER PRIMARY KEY, value TEXT)")
            conn.execute("INSERT INTO unrelated VALUES (93, 'preserve me')")

    def run_import(self, source=SOURCE):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--db", str(self.db), "--source", str(source)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_native_rows_import_idempotently_without_replacing_other_data(self):
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["rows"], 235)
        self.assertEqual(receipt["fingerprint"], FINGERPRINT)
        with sqlite3.connect(self.db) as conn:
            rows = conn.execute("SELECT * FROM creature_immunities ORDER BY ID").fetchall()
            self.assertEqual(
                conn.execute("SELECT ID, MechanicsMask, Effects FROM creature_immunities WHERE ID IN (1733,1808,1887) ORDER BY ID").fetchall(),
                [(1733, 1234599078, "98,124,144,145"),
                 (1808, 0, "98,124,144,145"),
                 (1887, 0, "98,124,144,145")],
            )
            self.assertEqual(conn.execute("SELECT * FROM unrelated").fetchall(), [(93, "preserve me")])
        encoded = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(encoded).hexdigest(), FINGERPRINT)
        repeated = self.run_import()
        self.assertEqual(repeated.returncode, 0, repeated.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute("SELECT * FROM creature_immunities ORDER BY ID").fetchall(), rows)

    def test_missing_source_table_fails_before_any_database_change(self):
        source = Path(self.directory.name) / "absent-table.sql"
        source.write_text("-- no creature_immunities table\n")
        before = self.db.read_bytes()
        result = self.run_import(source)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("creature_immunities", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
