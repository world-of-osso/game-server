"""Full proc rows are the importer's persisted external contract, not only IDs."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "import_spell_proc_presence.py"
COLUMNS = (
    "SpellId", "SchoolMask", "SpellFamilyName", "SpellFamilyMask0", "SpellFamilyMask1",
    "SpellFamilyMask2", "SpellFamilyMask3", "ProcFlags", "ProcFlags2", "SpellTypeMask",
    "SpellPhaseMask", "HitMask", "AttributesMask", "DisableEffectsMask", "ProcsPerMinute",
    "Chance", "Cooldown", "Charges",
)
# Exact effective audited TDB rows, not fabricated chance/family values.
ROWS = [
    dict(zip(COLUMNS, (64129, 0, 6, 1, 0, 524288, 0, 0, 0, 4, 2, 3, 0, 0, 0, 0, 0, 0))),
    dict(zip(COLUMNS, (383344, 0, 10, 0, 0, 0, 1073741824, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0))),
]


class SpellProcValuesTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.db = self.root / "world.db"
        with sqlite3.connect(self.db) as db:
            db.execute("CREATE TABLE untouched (Build INTEGER)")
            db.execute("INSERT INTO untouched VALUES (70448)")
        source = self.root / "spell_proc.sql"
        values = ",".join("(" + ",".join(str(row[key]) for key in COLUMNS) + ")" for row in ROWS)
        source.write_text("INSERT INTO spell_proc VALUES " + values + ";\n")
        self.audit = {
            "base": {"path": str(source), "sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "rows": 2},
            "patches": [], "effective_rows": 2, "rows": [dict(row) for row in ROWS],
        }
        self.manifest = self.root / "audit.json"

    def import_values(self):
        self.manifest.write_text(json.dumps(self.audit))
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--manifest", str(self.manifest), "--db", str(self.db)],
            text=True, capture_output=True, check=False,
        )

    def read_rows(self):
        with sqlite3.connect(self.db) as db:
            db.row_factory = sqlite3.Row
            return [dict(row) for row in db.execute("SELECT * FROM tdb_spell_proc ORDER BY SpellId")]

    def test_import_persists_every_pinned_value_and_is_idempotent(self):
        for _ in range(2):
            result = self.import_values()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(self.read_rows(), ROWS)
            with sqlite3.connect(self.db) as db:
                self.assertEqual(db.execute("SELECT * FROM untouched").fetchall(), [(70448,)])

    def test_existing_presence_table_becomes_authoritative_complete_rows(self):
        with sqlite3.connect(self.db) as db:
            db.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY)")
            db.executemany("INSERT INTO tdb_spell_proc VALUES (?)", [(64129,), (4341,)])
        result = self.import_values()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read_rows(), ROWS)

    def test_missing_value_rejects_before_changing_existing_rows(self):
        with sqlite3.connect(self.db) as db:
            db.execute("CREATE TABLE tdb_spell_proc (SpellId INTEGER PRIMARY KEY)")
            db.execute("INSERT INTO tdb_spell_proc VALUES (4341)")
        del self.audit["rows"][0]["SpellPhaseMask"]
        result = self.import_values()
        self.assertNotEqual(result.returncode, 0, "missing native proc value must not become an implicit zero")
        self.assertEqual(self.read_rows(), [{"SpellId": 4341}])


if __name__ == "__main__":
    unittest.main()
