import csv
import importlib.util
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location('import_item_spec', SCRIPTS / 'import_item_spec.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ImportItemSpecTest(unittest.TestCase):
    def test_additive_import_and_existing_table_refusal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            db = root / 'world.db'
            with sqlite3.connect(db) as conn:
                conn.execute('CREATE TABLE existing (value INTEGER)')
                conn.execute('INSERT INTO existing VALUES (7)')
            fixtures = {
                'ItemSpec.csv': (['ID', 'MinLevel', 'MaxLevel', 'ItemType', 'PrimaryStat', 'SecondaryStat', 'SpecializationID'], [1, 1, 123, 5, 1, 28, 581]),
                'ItemSpecOverride.csv': (['ID', 'SpecID', 'ItemID'], [2, 581, 258045]),
            }
            for filename, (header, row) in fixtures.items():
                with (root / filename).open('w', newline='') as handle:
                    writer = csv.writer(handle)
                    writer.writerow(header)
                    writer.writerow(row)
            self.assertEqual(module.import_item_specs(db, root), {'item_spec': 1, 'item_spec_override': 1})
            with sqlite3.connect(db) as conn:
                self.assertEqual(conn.execute('SELECT * FROM existing').fetchall(), [(7,)])
                self.assertEqual(conn.execute('SELECT SpecializationID FROM item_spec').fetchall(), [(581,)])
                self.assertEqual(conn.execute('SELECT SpecID,ItemID FROM item_spec_override').fetchall(), [(581, 258045)])
                schema = sqlite3.connect(':memory:')
                schema.executescript(module.db2.SCHEMA_SQL.read_text())
                for table in ('item_spec', 'item_spec_override'):
                    self.assertEqual(conn.execute(f'PRAGMA table_info({table})').fetchall(), schema.execute(f'PRAGMA table_info({table})').fetchall())
                schema.close()
            with self.assertRaisesRegex(ValueError, 'already exists'):
                module.import_item_specs(db, root)
            with sqlite3.connect(db) as conn:
                self.assertEqual(conn.execute('SELECT COUNT(*) FROM item_spec').fetchone(), (1,))


if __name__ == '__main__':
    unittest.main()
