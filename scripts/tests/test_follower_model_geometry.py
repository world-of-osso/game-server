import importlib.util
from pathlib import Path
import sqlite3
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location('follower_model_geometry', SCRIPTS / 'follower_model_geometry.py')
producer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(producer)

# Exact local TDB1210 rows, not invented model reaches.
GARRICK = (113753, 0.208, 1.5, 0, 66384)
SHUJA = (113752, 0.236, 1.5, 0, 66384)


class GeometryProducerTests(unittest.TestCase):
    def database(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE content_creature_template_model (CreatureID, CreatureDisplayID)')
        db.executemany('INSERT INTO content_creature_template_model VALUES (?,?)', [(209057, 113753), (227734, 121289)])
        db.execute(producer.SCHEMA)
        return db

    def test_sql_adds_only_missing_used_source_rows_and_keeps_an_unknown_display_unresolved(self):
        db = self.database()
        rows, gaps = producer.plan(db, [GARRICK, SHUJA])
        self.assertEqual(rows, [GARRICK])
        self.assertEqual(gaps, [121289])
        db.executescript(producer.render_sql(rows))
        self.assertEqual(db.execute('SELECT * FROM encounter_creature_model_info').fetchall(), [GARRICK])
        self.assertEqual(producer.plan(db, [GARRICK, SHUJA]), ([], [121289]))

    def test_existing_model_geometry_is_not_overwritten(self):
        db = self.database()
        db.execute('INSERT INTO encounter_creature_model_info VALUES (?,?,?,?,?)', GARRICK)
        rows, gaps = producer.plan(db, [GARRICK, SHUJA])
        self.assertEqual(rows, [])
        self.assertEqual(gaps, [121289])
        db.executescript(producer.render_sql(rows))
        self.assertEqual(db.execute('SELECT * FROM encounter_creature_model_info').fetchall(), [GARRICK])


if __name__ == '__main__':
    unittest.main()
