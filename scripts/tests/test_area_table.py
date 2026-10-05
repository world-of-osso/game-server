"""AreaTable-only import behavior with exact local DB2 CSV excerpts."""

import csv
import importlib.util
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("import_area_table", SCRIPTS / "import-area-table.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)
FIXTURE = Path(__file__).parent / "fixtures/city_rest_area_table.csv"


class AreaTableImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.db_path = Path(self.temp.name) / "world.db"
        with sqlite3.connect(self.db_path) as conn:
            conn.executescript(
                "CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT NOT NULL);"
                "CREATE TABLE creature_template (entry INTEGER PRIMARY KEY, name TEXT);"
                "INSERT INTO creature_template VALUES (295, 'Innkeeper Farley');"
            )

    def test_imports_real_capitals_and_parent_territory_without_replacing_other_data(self):
        self.assertEqual(importer.import_area_table(FIXTURE, self.db_path, "local-AreaTable.csv"), 5)
        with sqlite3.connect(self.db_path) as conn:
            self.assertEqual(conn.execute(
                "SELECT ID, ParentAreaID, Flags_0, FactionGroupMask FROM area_table ORDER BY ID"
            ).fetchall(), [
                (12, 0, 16448, 2), (1519, 0, 2113848, 2), (1637, 0, 2113848, 4),
                (5148, 1519, 1075855672, 0), (5170, 1637, 1075855672, 0),
            ])
            self.assertEqual(conn.execute("SELECT * FROM creature_template").fetchall(),
                             [(295, "Innkeeper Farley")])
            self.assertEqual(conn.execute("SELECT * FROM db2_build").fetchall(),
                             [("area_table", "local-AreaTable.csv")])

    def test_failed_reimport_preserves_previous_rows_and_provenance(self):
        importer.import_area_table(FIXTURE, self.db_path, "original")
        with FIXTURE.open(newline="") as handle:
            rows = list(csv.reader(handle))
        bad = Path(self.temp.name) / "bad.csv"
        with bad.open("w", newline="") as handle:
            csv.writer(handle).writerows(rows + [rows[1]])  # Duplicate primary key.
        with self.assertRaises(sqlite3.IntegrityError):
            importer.import_area_table(bad, self.db_path, "failed")
        with sqlite3.connect(self.db_path) as conn:
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM area_table").fetchone(), (5,))
            self.assertEqual(conn.execute("SELECT build FROM db2_build").fetchone(), ("original",))

    def test_invalid_header_does_not_modify_existing_import(self):
        importer.import_area_table(FIXTURE, self.db_path, "original")
        bad = Path(self.temp.name) / "bad.csv"
        bad.write_text("ID,Flags_0\n1519,256\n")
        with self.assertRaises(ValueError):
            importer.import_area_table(bad, self.db_path, "failed")
        with sqlite3.connect(self.db_path) as conn:
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM area_table").fetchone(), (5,))
            self.assertEqual(conn.execute("SELECT build FROM db2_build").fetchone(), ("original",))


if __name__ == "__main__":
    unittest.main()
