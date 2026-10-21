"""Concrete native item-bonus rows and transactional failure boundaries."""
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "item_instance_data", Path(__file__).resolve().parents[1] / "item_instance_data.py")
DATA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DATA)


class ItemInstanceImportTests(unittest.TestCase):
    def test_import_preserves_signed_values_and_replaces_only_named_table(self):
        with tempfile.TemporaryDirectory() as root, sqlite3.connect(":memory:") as conn:
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

    def test_bad_row_does_not_replace_existing_table(self):
        with tempfile.TemporaryDirectory() as root, sqlite3.connect(":memory:") as conn:
            conn.execute("CREATE TABLE item_bonus (ID INTEGER PRIMARY KEY, Value_0 INTEGER)")
            conn.execute("INSERT INTO item_bonus VALUES (2,7)")
            path = Path(root) / "ItemBonus.csv"
            path.write_text("ID,Value_0\n3,9\n3,11\n")
            with self.assertRaises(sqlite3.IntegrityError):
                DATA.import_table(conn, path, "item_bonus")
            self.assertEqual(conn.execute("SELECT * FROM item_bonus").fetchall(), [(2, 7)])


if __name__ == "__main__":
    unittest.main()
