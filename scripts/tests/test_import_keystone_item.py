import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("keystone", SCRIPTS / "import_keystone_item.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class KeystoneItemTest(unittest.TestCase):
    def test_private_publication_preserves_unrelated_items_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as temp:
            db = Path(temp) / "world.sqlite"
            with closing(sqlite3.connect(db)) as conn, conn:
                conn.executescript("""
                    CREATE TABLE content_item(ID INTEGER PRIMARY KEY, Name TEXT, Quality INTEGER, ClassID INTEGER, Stackable INTEGER);
                    INSERT INTO content_item VALUES(2589,'Linen Cloth',1,7,200);
                    CREATE TABLE item_sparse(ID INTEGER PRIMARY KEY, Display_lang TEXT, OverallQualityID INTEGER, Stackable INTEGER);
                    INSERT INTO item_sparse VALUES(180653,'Mythic Keystone',4,1);
                    CREATE TABLE item(ID INTEGER PRIMARY KEY, ClassID INTEGER);
                    INSERT INTO item VALUES(180653,5);
                """)
            module.import_item(db)
            module.import_item(db)
            with closing(sqlite3.connect(db)) as conn:
                self.assertEqual(conn.execute("SELECT * FROM content_item WHERE ID=180653").fetchone(), (180653, "Mythic Keystone", 4, 5, 1))
                self.assertEqual(conn.execute("SELECT * FROM content_item WHERE ID=2589").fetchone(), (2589, "Linen Cloth", 1, 7, 200))


if __name__ == "__main__":
    unittest.main()
