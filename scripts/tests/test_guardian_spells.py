"""Import exact archive rows and prove failed imports preserve existing bindings."""
import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("guardian_import", Path(__file__).parents[1] / "import-guardian-spells.py")
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class GuardianImportTest(unittest.TestCase):
    def test_native_rows_and_duplicate_rollback(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            conn = sqlite3.connect(":memory:")
            # Real TDB1210 rows; the duplicate spell appears at two distinct indices.
            archive.write_text("INSERT INTO `creature_template_spell` VALUES (416,0,3110,41031),(416,2,3110,44325);\n")
            self.assertEqual(IMPORTER.import_guardian_spells(conn, archive), 2)
            self.assertEqual(conn.execute("SELECT * FROM guardian_spells ORDER BY SpellIndex").fetchall(), [(416, 0, 3110, 41031), (416, 2, 3110, 44325)])
            archive.write_text("INSERT INTO `creature_template_spell` VALUES (416,0,3110,41031),(416,0,3110,41031);\n")
            with self.assertRaises(sqlite3.IntegrityError):
                IMPORTER.import_guardian_spells(conn, archive)
            self.assertEqual(conn.execute("SELECT count(*) FROM guardian_spells").fetchone()[0], 2)
            conn.close()

    def test_no_binding_is_not_synthesized(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            archive.write_text("-- no creature spell table\n")
            with sqlite3.connect(":memory:") as conn:
                with self.assertRaises(ValueError):
                    IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(conn.execute("SELECT name FROM sqlite_master").fetchall(), [])


if __name__ == "__main__":
    unittest.main()
