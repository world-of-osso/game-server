"""Native70448 trait modifiers and their curves through the normal DB2 importer."""
from contextlib import closing
from pathlib import Path
import gc
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch
import warnings

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2


class TraitEffectPointsImportTests(unittest.TestCase):
    def test_native_points_and_referenced_curves_survive_normal_import(self):
        tables = [name for name in db2.TABLES if name in {
            'TraitDefinitionEffectPoints', 'Curve', 'CurvePoint',
        }]
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / 'world.db'
            with closing(sqlite3.connect(database)) as conn, conn:
                conn.execute('CREATE TABLE unrelated(value TEXT)')
                conn.execute("INSERT INTO unrelated VALUES('preserved')")
            # Scope the importer to this feature; unrelated item/Forever catalogs
            # are external inputs, not mocked trait rows or curve calculations.
            with warnings.catch_warnings(record=True) as resource_warnings, \
                    patch.object(db2, 'TABLES', tables), \
                    patch.object(db2.item_instance_data, 'import_catalog', return_value={}), \
                    patch.object(db2.skyborne_data, 'load_rows', return_value={}), \
                    patch.object(db2.skyborne_data, 'import_rows'):
                warnings.simplefilter('always', ResourceWarning)
                db2.import_tables('12.1.5.70448', db2.DEFAULT_CSV_ROOT, database, Path(directory))
                gc.collect()
            self.assertEqual([
                str(warning.message) for warning in resource_warnings
                if issubclass(warning.category, ResourceWarning)
            ], [])
            with closing(sqlite3.connect(database)) as conn:
                self.assertEqual(conn.execute(
                    'SELECT OperationType,count(*) FROM trait_definition_effect_points '
                    'GROUP BY OperationType ORDER BY OperationType'
                ).fetchall(), [(-1, 7), (0, 909), (1, 492)])
                self.assertEqual(conn.execute(
                    'SELECT TraitDefinitionID,EffectIndex,OperationType,CurveID '
                    'FROM trait_definition_effect_points WHERE ID=22216'
                ).fetchone(), (120041, 0, 1, 64686))
                self.assertEqual(conn.execute(
                    'SELECT Pos_0,Pos_1 FROM curve_point WHERE CurveID=64686 ORDER BY OrderIndex'
                ).fetchall(), [(1.0, 1.0), (2.0, 2.0)])
                self.assertEqual(conn.execute(
                    'SELECT count(*) FROM trait_definition_effect_points p '
                    'WHERE NOT EXISTS(SELECT 1 FROM curve c WHERE c.ID=p.CurveID) '
                    'OR NOT EXISTS(SELECT 1 FROM curve_point q WHERE q.CurveID=p.CurveID)'
                ).fetchone(), (0,))
                self.assertEqual(conn.execute(
                    'SELECT table_name,build FROM db2_build ORDER BY table_name'
                ).fetchall(), [('curve', '12.1.5.70448'), ('curve_point', '12.1.5.70448'),
                               ('trait_definition_effect_points', '12.1.5.70448')])
                self.assertEqual(conn.execute('SELECT value FROM unrelated').fetchone(), ('preserved',))


if __name__ == '__main__':
    unittest.main()
