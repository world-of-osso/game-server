"""Build selection and supported-layout import proofs; never downloads."""
import importlib.util
import shutil
import sqlite3
import struct
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
    def test_active_build_reads_retail_install_and_resolved_active_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "13.0.0.90001"
            directory.mkdir()
            active = root / "active"
            active.symlink_to(directory)
            info = root / ".build.info"
            info.write_text("Product!STRING:0|Active!DEC:1|Version!STRING:0\n"
                            "wow_classic|1|5.5.4.70268\n"
                            "wow|0|12.1.0.69933\n"
                            "wow|1|13.0.0.90001\n")
            self.assertEqual(importer.read_active_build(active, info), "13.0.0.90001")
            info.write_text("Product!STRING:0|Active!DEC:1|Version!STRING:0\n"
                            "wow|1|13.0.0.90002\n")
            with self.assertRaisesRegex(ValueError, "installed wow build 13.0.0.90002"):
                importer.read_active_build(active, info)
            info.write_text("Product!STRING:0|Active!DEC:1|Version!STRING:0\n"
                            "wow_classic|1|5.5.4.70268\n")
            with self.assertRaisesRegex(ValueError, "expected one active wow build"):
                importer.read_active_build(active, info)

    def test_unknown_actual_file_layout_fails_before_export(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            data = bytearray(204)
            data[:4] = b"WDC5"
            struct.pack_into("<I", data, 156, 0xDEADBEEF)
            (directory / "1260179.db2").write_bytes(data)
            with self.assertRaisesRegex(ValueError, "Achievement: layout DEADBEEF"):
                importer.export_tables(directory)
            self.assertFalse((directory / "Achievement.csv").exists())

    def test_dungeon2_local_categories_and_stockade_metadata(self):
        # Original native golden fixture proves this unchanged supported layout's mapping.
        source = SCRIPTS.parent / "data/db2/12.1.0.69933"
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / source.name
            directory.mkdir()
            for fdid, _, _, _ in importer.TABLES.values():
                shutil.copyfile(source / f"{fdid}.db2", directory / f"{fdid}.db2")
            tables = importer.export_tables(directory)
            database = Path(temporary) / "world.db"
            with sqlite3.connect(database) as db:
                db.execute("CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT)")
            importer.import_tables(database, tables, directory.name)
            with sqlite3.connect(database) as db:
                row = db.execute("SELECT a.Category, a.IconFileID, a.Points, c.Name_lang, c.Parent FROM achievement a JOIN achievement_category c ON c.ID=a.Category WHERE a.ID=633").fetchone()
                self.assertEqual(row, (14808, 134163, 10, "Classic", 168))
                self.assertEqual(db.execute("SELECT COUNT(*) FROM achievement_category").fetchone()[0], 243)
                self.assertEqual(db.execute("SELECT build FROM db2_build WHERE table_name='achievement_category'").fetchone()[0], directory.name)


if __name__ == "__main__":
    unittest.main()
