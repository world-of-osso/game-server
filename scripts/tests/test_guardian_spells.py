"""Native binding precedence, explicit project policies and atomic import failure."""
import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "guardian_import", Path(__file__).parents[1] / "import-guardian-spells.py"
)
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


def read_bindings(conn):
    return conn.execute(
        "SELECT * FROM guardian_spells ORDER BY CreatureID,SpellIndex"
    ).fetchall()


class GuardianImportTest(unittest.TestCase):
    def test_native_rows_and_duplicate_rollback(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            conn = sqlite3.connect(":memory:")
            # Real TDB1210 rows: one spell at two distinct native indices.
            archive.write_text(
                "INSERT INTO `creature_template_spell` VALUES "
                "(416,0,3110,41031),(416,2,3110,44325);\n"
            )
            native_count, _ = IMPORTER.import_guardian_spells(conn, archive)
            self.assertEqual(native_count, 2)
            self.assertEqual(
                [row for row in read_bindings(conn) if row[0] == 416],
                [(416, 0, 3110, 41031), (416, 2, 3110, 44325)],
            )
            before = read_bindings(conn)
            archive.write_text(
                "INSERT INTO `creature_template_spell` VALUES "
                "(416,0,3110,41031),(416,0,3110,41031);\n"
            )
            with self.assertRaises(sqlite3.IntegrityError):
                IMPORTER.import_guardian_spells(conn, archive)
            self.assertEqual(read_bindings(conn), before)
            conn.close()

    def test_approved_missing_bindings_are_explicit_policy_rows(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            archive.write_text(
                "INSERT INTO `creature_template_spell` VALUES (416,0,3110,41031);\n"
            )
            with sqlite3.connect(":memory:") as conn:
                native_count, policies = IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(native_count, 1)
                self.assertEqual(
                    set(policies),
                    {
                        "SIMC_DREADSTALKER_ATTACK_BINDING",
                        "SIMC_ANTORAN_TYRANT_BURNING_CLEAVE_BINDING",
                    },
                )
                self.assertEqual(
                    [row for row in read_bindings(conn) if row[0] in (98035, 250289)],
                    [(98035, 0, 271971, 0), (250289, 0, 1264093, 0)],
                )

    def test_native_binding_list_wins_over_policy_and_reimport_is_stable(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            # Hypothetical future list using a qualified pair, not an observed
            # TDB1210 row. Its nonzero index must not acquire a policy index0.
            archive.write_text(
                "INSERT INTO `creature_template_spell` VALUES (98035,4,271971,69497);\n"
            )
            with sqlite3.connect(":memory:") as conn:
                native_count, policies = IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(native_count, 1)
                self.assertEqual(
                    policies, ["SIMC_ANTORAN_TYRANT_BURNING_CLEAVE_BINDING"]
                )
                expected = [(98035, 4, 271971, 69497), (250289, 0, 1264093, 0)]
                self.assertEqual(read_bindings(conn), expected)
                IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(read_bindings(conn), expected)

    def test_missing_archive_table_does_not_publish_policy_rows(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / "archive.sql"
            archive.write_text("-- no creature spell table\n")
            with sqlite3.connect(":memory:") as conn:
                with self.assertRaises(ValueError):
                    IMPORTER.import_guardian_spells(conn, archive)
                self.assertEqual(conn.execute("SELECT name FROM sqlite_master").fetchall(), [])


if __name__ == "__main__":
    unittest.main()
