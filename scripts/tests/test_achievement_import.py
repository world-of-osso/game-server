"""Concrete local DB2 import proof; requires the pinned CASC assets, never downloads."""
import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("import_achievements", SCRIPTS / "import_achievements.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class AchievementImportTests(unittest.TestCase):
    def test_dungeon2_local_categories_and_stockade_metadata(self):
        directory = SCRIPTS.parent / "data/db2" / importer.BUILD
        tables = importer.export_tables(directory)
        with tempfile.TemporaryDirectory() as temporary:
            database = Path(temporary) / "world.db"
            with sqlite3.connect(database) as db:
                db.execute("CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT)")
            importer.import_tables(database, tables)
            with sqlite3.connect(database) as db:
                row = db.execute("SELECT a.Category, a.IconFileID, a.Points, c.Name_lang, c.Parent FROM achievement a JOIN achievement_category c ON c.ID=a.Category WHERE a.ID=633").fetchone()
                self.assertEqual(row, (14808, 134163, 10, "Classic", 168))
                self.assertEqual(db.execute("SELECT COUNT(*) FROM achievement_category").fetchone()[0], 243)
                self.assertEqual(db.execute("SELECT build FROM db2_build WHERE table_name='achievement_category'").fetchone()[0], importer.BUILD)


if __name__ == "__main__":
    unittest.main()
