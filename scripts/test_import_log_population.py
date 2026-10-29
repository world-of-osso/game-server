import importlib.util
import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('import_log_population', Path(__file__).with_name('import_log_population.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def record(key, x, npc=123):
    return {'source_key': key, 'map': 2521, 'npc_id': npc, 'x': x, 'y': 20.0, 'z': 30.0,
            'population_kind': 'likely_spawn', 'summoned': False, 'identity_ambiguous': False,
            'facing': 1.5, 'level': 90, 'z_method': 'single_walkable_floor',
            'observation_event': 'SPELL_DAMAGE', 'observation_time': '10/3/2026 20:23:32.000-5'}


class ImportTest(unittest.TestCase):
    def test_copy_only_import_preserves_authored_and_tags_sourced_row(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'world.db'
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(source) as db:
                db.executescript('''
                    CREATE TABLE content_creature_template(entry INTEGER PRIMARY KEY);
                    INSERT INTO content_creature_template VALUES(123);
                    CREATE TABLE tdb_creature(id INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);
                    INSERT INTO tdb_creature VALUES(123,2521,10,20,30);
                    CREATE TABLE content_creature(guid INTEGER PRIMARY KEY,id1 INTEGER,map INTEGER,
                        position_x REAL,position_y REAL,position_z REAL,orientation REAL,
                        spawnDifficulties TEXT,Comment TEXT);
                    INSERT INTO content_creature VALUES(5,123,2521,10,20,30,2,'1','authored');
                ''')
            shutil.copyfile(source, copy)
            with self.assertRaises(ValueError):
                module.import_population(source, [record('x', 100)])
            report = module.import_population(copy, [record('authored', 11), record('new', 100),
                                                     dict(record('summoned', 200), summoned=True),
                                                     dict(record('missing', 300), facing=None)])
            self.assertEqual((report['2521']['imported'], report['2521']['already_present']), (1, 1))
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT Comment,position_x FROM content_creature WHERE guid=5').fetchone(), ('authored', 10))
                self.assertEqual(db.execute('SELECT Comment,position_z,orientation FROM content_creature WHERE guid=6').fetchone(),
                                 ('LOG_DERIVED_POPULATION', 30, 1.5))
                self.assertEqual(db.execute('SELECT source_key,z_method,observed_level FROM log_derived_population').fetchone(),
                                 ('new', 'single_walkable_floor', 90))
            self.assertEqual(module.import_population(copy, [record('new', 100)])['2521']['already_present'], 1)
            with sqlite3.connect(source) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature').fetchone()[0], 1)


if __name__ == '__main__':
    unittest.main()
