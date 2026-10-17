"""Build selection and supported-layout import proofs; never downloads."""
import importlib.util
import csv
import hashlib
import json
from unittest.mock import patch
import shutil
import sqlite3
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from contextlib import closing

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("import_achievements", SCRIPTS / "import_achievements.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class AchievementImportTests(unittest.TestCase):
    def test_active_csv_import_preserves_named_tree_flags_and_order_without_raw_db2s(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / '12.1.5.70448'
            directory.mkdir()
            active = root / 'active'
            active.symlink_to(directory)
            info = root / '.build.info'
            info.write_text('Product!STRING:0|Active!DEC:1|Version!STRING:0\n'
                            'wow|1|12.1.5.70448\n')
            receipt = {'identity': {'product': 'wow', 'build': directory.name,
                                     'build_key': '88d5e8ef05ec5a8446cdec93d142d7c6'}, 'tables': {}}
            for name, (fdid, layout, _, columns) in importer.TABLES.items():
                path = directory / f'{name}.csv'
                names = [column for column, _ in columns]
                row = {column: '' if column.endswith('_lang') else '0' for column in names}
                row['ID'] = '4479'
                if name == 'CriteriaTree':
                    row.update(Flags='8193', OrderIndex='2', Amount='50', Operator='5')
                with path.open('w', newline='') as output:
                    writer = csv.DictWriter(output, fieldnames=list(reversed(names)))
                    writer.writeheader()
                    writer.writerow(row)
                receipt['tables'][name] = {'fdid': fdid, 'layout': f'{layout:08X}',
                    'rows': 1, 'csv_sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
            (directory / 'receipt.json').write_text(json.dumps(receipt))
            database = root / 'world.db'
            with closing(sqlite3.connect(database)) as db, db:
                db.execute('CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT)')
            error = None
            with patch.object(importer, 'ACTIVE_DB2', active), patch.object(importer, 'BUILD_INFO', info), patch.object(sys, 'argv', ['import_achievements.py', '--db2-dir', str(active), '--db', str(database)]):
                try:
                    importer.main()
                except FileNotFoundError as failure:
                    error = failure
            self.assertIsNone(error, 'active CSV import must not require obsolete raw DB2 exports')
            with closing(sqlite3.connect(database)) as db, db:
                self.assertEqual(db.execute('SELECT Flags,OrderIndex,Amount FROM criteria_tree WHERE ID=4479').fetchone(), (8193, 2, 50))
                self.assertEqual(dict(db.execute('SELECT table_name,build FROM db2_build')),
                                 dict.fromkeys(importer.SQL_NAMES.values(), directory.name))
            # Modified CSV bytes cannot be imported under the original receipt.
            with (directory / 'CriteriaTree.csv').open('a') as output:
                output.write('\n')
            with patch.object(importer, 'ACTIVE_DB2', active), patch.object(importer, 'BUILD_INFO', info), patch.object(sys, 'argv', ['import_achievements.py', '--db2-dir', str(active), '--db', str(database)]):
                with self.assertRaisesRegex(ValueError, 'CriteriaTree.*SHA256'):
                    importer.main()

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
            with closing(sqlite3.connect(database)) as db, db:
                db.execute("CREATE TABLE db2_build (table_name TEXT PRIMARY KEY, build TEXT)")
            importer.import_tables(database, tables, directory.name)
            with closing(sqlite3.connect(database)) as db, db:
                row = db.execute("SELECT a.Category, a.IconFileID, a.Points, c.Name_lang, c.Parent FROM achievement a JOIN achievement_category c ON c.ID=a.Category WHERE a.ID=633").fetchone()
                self.assertEqual(row, (14808, 134163, 10, "Classic", 168))
                self.assertEqual(db.execute("SELECT COUNT(*) FROM achievement_category").fetchone()[0], 243)
                self.assertEqual(db.execute("SELECT build FROM db2_build WHERE table_name='achievement_category'").fetchone()[0], directory.name)


if __name__ == "__main__":
    unittest.main()
