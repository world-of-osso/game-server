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

    def test_approved_missing_bindings_are_explicit_policy_rows(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            archive.write_text("INSERT INTO `creature_template_spell` VALUES (416,0,3110,41031);\n")
            with sqlite3.connect(":memory:") as conn:
                IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(
                    conn.execute("SELECT * FROM guardian_spells WHERE CreatureID IN (98035,250289) ORDER BY CreatureID").fetchall(),
                    [(98035, 0, 271971, 0), (250289, 0, 1264093, 0)],
                )

    def test_native_binding_list_wins_over_policy_and_reimport_is_stable(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            # Hypothetical future native list using the source-qualified pair;
            # this does not claim that TDB1210 currently contains this row.
            archive.write_text("INSERT INTO `creature_template_spell` VALUES (98035,4,271971,69497);\n")
            with sqlite3.connect(":memory:") as conn:
                IMPORTER.import_guardian_spells(conn, archive)
                expected = [(98035, 4, 271971, 69497), (250289, 0, 1264093, 0)]
                self.assertEqual(conn.execute("SELECT * FROM guardian_spells ORDER BY CreatureID,SpellIndex").fetchall(), expected)
                IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(conn.execute("SELECT * FROM guardian_spells ORDER BY CreatureID,SpellIndex").fetchall(), expected)

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
