"""Native set identities and threshold/spec rows survive the normal importer."""
from contextlib import closing
from pathlib import Path
import sqlite3
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2
import skyborne_kit_fixture
from scripts.tests.fixture_env import requires_env


class ItemSetImportTests(unittest.TestCase):
    @requires_env('FOREVER_DB2_SOURCE')
    def test_owned_kit_projection_retains_required_native_set_catalogs(self):
        with closing(skyborne_kit_fixture.fixture_database(Path(os.environ['FOREVER_DB2_SOURCE']))) as conn:
            tables = {row[0] for row in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
            self.assertTrue({'item_set', 'item_set_spell'}.issubset(tables),
                            'owned runtime kit projection must retain native set catalogs')
            # This captured starting kit contains no Retail set pieces.
            self.assertEqual(conn.execute('SELECT count(*) FROM item_sparse WHERE ItemSet!=0').fetchone(), (0,))
            self.assertEqual(conn.execute('SELECT count(*) FROM item_set').fetchone(), (0,))
            self.assertEqual(conn.execute('SELECT count(*) FROM item_set_spell').fetchone(), (0,))

    def test_native_ret_and_elemental_sets_survive_normal_import(self):
        tables = [name for name in db2.TABLES if name in {'ItemSet', 'ItemSetSpell'}]
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / 'world.db'
            with closing(sqlite3.connect(database)) as conn, conn:
                conn.execute('CREATE TABLE unrelated(value TEXT)')
                conn.execute("INSERT INTO unrelated VALUES('preserved')")
            with patch.object(db2, 'TABLES', tables), \
                    patch.object(db2.item_instance_data, 'import_catalog', return_value={}), \
                    patch.object(db2.skyborne_data, 'load_rows', return_value={}), \
                    patch.object(db2.skyborne_data, 'import_rows'):
                db2.import_tables('12.1.5.70448', db2.DEFAULT_CSV_ROOT, database, Path(directory))
            with closing(sqlite3.connect(database)) as conn:
                imported = {row[0] for row in conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='table'"
                )}
                self.assertTrue({'item_set', 'item_set_spell'}.issubset(imported),
                                'normal importer must publish native set catalogs')
                self.assertEqual(conn.execute(
                    'SELECT ID,SetFlags,RequiredSkill,ItemID_0,ItemID_1,ItemID_2,ItemID_3,ItemID_4 '
                    'FROM item_set WHERE ID=2062'
                ).fetchone(), (2062, 0, 0, 271468, 271466, 271465, 271464, 271463))
                self.assertEqual(conn.execute(
                    'SELECT ChrSpecID,SpellID,TraitSubTreeID,Threshold FROM item_set_spell '
                    'WHERE ItemSetID=2062 AND ChrSpecID=70 ORDER BY Threshold'
                ).fetchall(), [(70, 1296660, 0, 2), (70, 1296661, 0, 4)])
                self.assertEqual(conn.execute(
                    'SELECT SpellID,Threshold FROM item_set_spell '
                    'WHERE ItemSetID=2065 AND ChrSpecID=262 ORDER BY Threshold'
                ).fetchall(), [(1296625, 2), (1296626, 4)])
                self.assertEqual(conn.execute(
                    'SELECT table_name,build FROM db2_build ORDER BY table_name'
                ).fetchall(), [('item_set', '12.1.5.70448'), ('item_set_spell', '12.1.5.70448')])
                self.assertEqual(conn.execute('SELECT value FROM unrelated').fetchone(), ('preserved',))


if __name__ == '__main__':
    unittest.main()
