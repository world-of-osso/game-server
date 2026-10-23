"""Concrete native item-bonus rows and transactional failure boundaries."""
from contextlib import closing
import csv
import importlib.util
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "item_instance_data", Path(__file__).resolve().parents[1] / "item_instance_data.py")
DATA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DATA)


def publish_native_catalogs(path, source):
    return subprocess.run(
        [sys.executable, str(DATA.__file__), "--db2-dir", str(source), "--db", str(path),
         "--publish-producer-catalogs"], capture_output=True, text=True, check=False)


class ItemInstanceImportTests(unittest.TestCase):
    def test_import_preserves_signed_values_and_replaces_only_named_table(self):
        with tempfile.TemporaryDirectory() as root, closing(sqlite3.connect(":memory:")) as conn:
            path = Path(root) / "ItemBonus.csv"
            path.write_text("ID,Value_0,Value_1,Value_2,Value_3,ParentItemBonusListID,Type,OrderIndex\n"
                            "2,7,15789,0,0,1,2,0\n3,-10,0,0,0,2,1,0\n")
            conn.execute("CREATE TABLE untouched (ID INTEGER)")
            conn.execute("INSERT INTO untouched VALUES (70448)")
            self.assertEqual(DATA.import_table(conn, path, "item_bonus"), 2)
            self.assertEqual(conn.execute("SELECT Value_0 FROM item_bonus ORDER BY ID").fetchall(),
                             [(7,), (-10,)])
            self.assertEqual(conn.execute("SELECT * FROM untouched").fetchall(), [(70448,)])
            self.assertEqual(DATA.import_table(conn, path, "item_bonus"), 2)
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM item_bonus").fetchone()[0], 2)

    def test_native_equipped_producer_set_and_role_sources_are_imported(self):
        source = Path(__file__).resolve().parents[2] / "data/db2/12.1.5.70448"
        with closing(sqlite3.connect(":memory:")) as conn:
            counts = DATA.import_catalog(conn, source)
            for table in ("item_set", "item_set_spell", "spell_description_variables",
                          "spell_x_description_variables"):
                self.assertIn(table, counts, f"native producer input missing: {table}")
                self.assertGreater(counts[table], 0)
            self.assertEqual(conn.execute(
                "SELECT ChrSpecID,Threshold,ItemSetID FROM item_set_spell WHERE SpellID=1296648"
            ).fetchall(), [(73, 4, 2067)])
            self.assertEqual(conn.execute(
                "SELECT ChrSpecID,Threshold,ItemSetID FROM item_set_spell WHERE SpellID=1291726"
            ).fetchall(), [(0, 2, 2070)])
            self.assertEqual(conn.execute(
                "SELECT ItemID_0,ItemID_1,ItemID_2 FROM item_set WHERE ID=2070"
            ).fetchone(), (270173, 268209, 268213))
            self.assertEqual(conn.execute(
                "SELECT SpellID,SpellDescriptionVariablesID FROM spell_x_description_variables "
                "WHERE SpellID IN (1291718,1291728) ORDER BY SpellID"
            ).fetchall(), [(1291718, 884), (1291728, 884)])
            variables = conn.execute(
                "SELECT Variables FROM spell_description_variables WHERE ID=884"
            ).fetchone()[0]
            with (source / "SpellDescriptionVariables.csv").open() as handle:
                native = next(row["Variables"] for row in csv.DictReader(handle)
                              if row["ID"] == "884")
            self.assertEqual(variables, native)
            self.assertEqual(conn.execute(
                "SELECT build FROM item_instance_source WHERE table_name='item_set_spell'"
            ).fetchone(), ("12.1.5.70448",))

    def test_producer_publication_adds_only_new_catalogs_and_refuses_existing_drift(self):
        source = Path(__file__).resolve().parents[2] / "data/db2/12.1.5.70448"
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "canonical-copy.db"
            with closing(sqlite3.connect(path)) as conn:
                conn.execute("CREATE TABLE item_bonus (ID INTEGER PRIMARY KEY, Value_0 INTEGER)")
                conn.execute("INSERT INTO item_bonus VALUES (2,7)")
                conn.execute("CREATE TABLE item_instance_source "
                             "(table_name TEXT PRIMARY KEY, build TEXT NOT NULL, sha256 TEXT NOT NULL)")
                conn.execute("INSERT INTO item_instance_source VALUES (?,?,?)",
                             ("item_bonus", "prior-build", "preserve-this-source-stamp"))
                conn.commit()
                result = publish_native_catalogs(path, source)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(conn.execute("SELECT * FROM item_bonus").fetchall(), [(2, 7)])
                self.assertEqual(conn.execute(
                    "SELECT build,sha256 FROM item_instance_source WHERE table_name='item_bonus'"
                ).fetchone(), ("prior-build", "preserve-this-source-stamp"))
                self.assertEqual(conn.execute(
                    "SELECT Threshold FROM item_set_spell WHERE SpellID=1296648"
                ).fetchone(), (4,))
                self.assertEqual(conn.execute(
                    "SELECT COUNT(*) FROM item_instance_source"
                ).fetchone()[0], 5)
                version = conn.execute("PRAGMA data_version").fetchone()[0]
                result = publish_native_catalogs(path, source)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(conn.execute("PRAGMA data_version").fetchone()[0], version,
                                 "identical repeat publication must not write database pages")
                conn.execute("UPDATE item_set_spell SET Threshold=3 WHERE SpellID=1296648")
                conn.commit()
                version = conn.execute("PRAGMA data_version").fetchone()[0]
                result = publish_native_catalogs(path, source)
                self.assertNotEqual(result.returncode, 0, "drift must refuse, not overwrite")
                self.assertEqual(conn.execute("PRAGMA data_version").fetchone()[0], version)
                self.assertEqual(conn.execute(
                    "SELECT Threshold FROM item_set_spell WHERE SpellID=1296648"
                ).fetchone(), (3,))
                self.assertEqual(conn.execute("SELECT * FROM item_bonus").fetchall(), [(2, 7)])

    def test_bad_row_does_not_replace_existing_table(self):
        with tempfile.TemporaryDirectory() as root, closing(sqlite3.connect(":memory:")) as conn:
            conn.execute("CREATE TABLE item_bonus (ID INTEGER PRIMARY KEY, Value_0 INTEGER)")
            conn.execute("INSERT INTO item_bonus VALUES (2,7)")
            path = Path(root) / "ItemBonus.csv"
            path.write_text("ID,Value_0\n3,9\n3,11\n")
            with self.assertRaises(sqlite3.IntegrityError):
                DATA.import_table(conn, path, "item_bonus")
            self.assertEqual(conn.execute("SELECT * FROM item_bonus").fetchall(), [(2, 7)])


if __name__ == "__main__":
    unittest.main()
