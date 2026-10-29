import importlib.util
import json
import shutil
import sqlite3
import tempfile
import unittest
from contextlib import closing
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
    def test_affix_summons_are_not_static_spawns_and_prior_imports_are_retired(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'world-copy.db'
            with closing(sqlite3.connect(copy)) as db, db:
                for table, data in fixture.items():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {table} VALUES ({','.join('?' for _ in row)})", row)
                db.execute('INSERT INTO content_creature_template(entry) VALUES(229296)')
                db.execute('INSERT INTO content_creature_template(entry) VALUES(123)')
                db.execute("INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z,Comment) VALUES(1,229296,2521,10,20,30,'authored')")
            records = [record('affix', 100, npc=229296), record('ordinary', 200)]
            module.import_population(copy, records)
            with closing(sqlite3.connect(copy)) as db, db:
                db.execute("INSERT INTO keystone_affix VALUES(148,?)", ("Xal'atath's Bargain: Ascendant",))
                db.execute("INSERT INTO spell_name VALUES(461936,?)", ("Xal'atath's Bargain: Ascendant",))
                db.execute('INSERT INTO spell_effect VALUES(461936,28,229296)')
                db.execute("INSERT INTO content_creature(guid,id1,map,Comment) VALUES(99,229296,1762,?)", (module.POLICY,))
                db.execute("INSERT INTO log_derived_population VALUES(99,'other-map',1762,229296,'floor','SPELL_DAMAGE','time',90,?,'{}')", (module.POLICY,))
            report = module.import_population(copy, records)
            self.assertIn('AFFIX_SUMMON_RETIRED', [gap['reason'] for gap in report['2521']['gaps']])
            self.assertIn('AFFIX_SUMMON', [gap['reason'] for gap in report['2521']['gaps']])
            with closing(sqlite3.connect(copy)) as db, db:
                self.assertEqual(db.execute('SELECT id1,Comment FROM content_creature ORDER BY guid').fetchall(),
                                 [(229296, 'authored'), (123, module.POLICY), (229296, module.POLICY)])
                self.assertEqual(db.execute('SELECT source_key FROM log_derived_population ORDER BY guid').fetchall(),
                                 [('ordinary',), ('other-map',)])
            self.assertEqual(module.import_population(copy, records)['2521']['imported'], 0)

    def test_boss_deduplication_uses_authored_then_start_observation_then_placeholder(self):
        # Real October 2026 positions, GUIDs, and source timings from the four follower maps.
        with tempfile.TemporaryDirectory() as directory, closing(sqlite3.connect(':memory:')) as db, db:
            db.executescript('''
                CREATE TABLE content_creature(guid INTEGER PRIMARY KEY,id1 INTEGER,map INTEGER,Comment TEXT);
                CREATE TABLE tdb_creature(id INTEGER,map INTEGER);
                CREATE TABLE log_derived_population(guid INTEGER PRIMARY KEY,observation_json TEXT);
                CREATE TABLE raid_encounter_creature(map_id INTEGER,creature_entry INTEGER,encounter_id INTEGER);
                INSERT INTO raid_encounter_creature VALUES(2813,234649,3102),(2825,241812,3207),
                    (2923,239008,3286),(2923,239167,3287),(2993,259445,3456);
                INSERT INTO tdb_creature VALUES(241812,2825);
                INSERT INTO content_creature VALUES(31000640,241812,2825,NULL);
                INSERT INTO content_creature VALUES(3276416712,241812,2825,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(63813001,234649,2813,'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP');
                INSERT INTO content_creature VALUES(3276416398,234649,2813,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(63923000,239008,2923,'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP');
                INSERT INTO content_creature VALUES(3276417028,239008,2923,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(3276417029,239167,2923,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(3276417030,239167,2923,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(63993000,259445,2993,'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP');
                INSERT INTO content_creature VALUES(3276417173,259445,2993,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(3276417174,259445,2993,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(3276417175,259445,2993,'LOG_DERIVED_POPULATION');
                INSERT INTO content_creature VALUES(1,187969,2521,NULL),(2,187969,2521,'LOG_DERIVED_POPULATION');
            ''')
            samples = [
                (3276416398, '10/1/2026 22:32:55.772-5', '10/1/2026 22:41:55.410-5', '10/1/2026 22:41:55.778-5', 3102),
                (3276417028, '10/1/2026 21:14:10.071-5', '10/1/2026 21:28:36.765-5', '10/1/2026 21:28:37.162-5', 3286),
                (3276417029, '10/1/2026 21:14:10.071-5', '10/1/2026 21:37:55.099-5', '10/1/2026 21:37:55.103-5', 3287),
                (3276417030, '10/8/2026 03:05:41.731-5', '10/8/2026 03:27:49.903-5', '10/8/2026 03:27:50.659-5', 3287),
                (3276417173, '10/3/2026 03:13:00.925-5', '10/3/2026 03:18:43.191-5', '10/3/2026 03:18:43.199-5', 3456),
                (3276417174, '10/5/2026 22:03:43.503-5', '10/5/2026 22:10:10.980-5', '10/5/2026 22:10:10.995-5', 3456),
                (3276417175, '10/15/2026 20:27:06.584-5', '10/15/2026 20:34:56.914-5', '10/15/2026 20:34:56.918-5', 3456),
            ]
            for guid, run, start, observed, encounter in samples:
                name = f'{guid}.txt'
                (Path(directory) / name).write_text(f'{run}  CHALLENGE_MODE_START\n{start}  ENCOUNTER_START,{encounter},"boss"\n')
                db.execute('INSERT INTO log_derived_population VALUES (?,?)', (guid, json.dumps(
                    {'source_file': name, 'source_run_start': run, 'observation_time': observed})))
            db.execute('INSERT INTO log_derived_population VALUES (3276416712,?)', ('{}',))
            retired = module.retire_encounter_duplicates(db, Path(directory))
            self.assertEqual(retired, [(2813, 234649, 63813001), (2825, 241812, 3276416712),
                                       (2923, 239008, 63923000), (2923, 239167, 3276417030),
                                       (2993, 259445, 63993000),
                                       (2993, 259445, 3276417173), (2993, 259445, 3276417174)])
            self.assertEqual(db.execute('SELECT guid FROM content_creature ORDER BY guid').fetchall(),
                             [(1,), (2,), (31000640,), (3276416398,), (3276417028,), (3276417029,), (3276417175,)])
            self.assertEqual(module.retire_encounter_duplicates(db, Path(directory)), [])

    def test_import_keeps_near_start_boss_observation_over_nearby_placeholder(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'world-copy.sqlite'
            with sqlite3.connect(copy) as db:
                for data in fixture.values():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {data['sql'].split()[2]} VALUES ({','.join('?' for _ in row)})", row)
                db.execute('INSERT INTO content_creature_template(entry) VALUES (234649)')
                db.execute('INSERT INTO raid_encounter_creature VALUES (2813,234649,3102)')
                db.execute('''INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z,Comment)
                    VALUES (63813001,234649,2813,8924.3223,-4738.4488,30.3815,?)''', (module.JOURNAL_PLACEHOLDER,))
            run = '10/1/2026 22:32:55.772-5'
            (Path(directory) / 'source.txt').write_text(
                f'{run}  CHALLENGE_MODE_START\n10/1/2026 22:41:55.410-5  ENCOUNTER_START,3102,"Zaen Bladesorrow"\n')
            observation = dict(record('zaen', 8916.85, npc=234649), map=2813, y=-4738.5, z=29.995829,
                               source_file='source.txt', source_run_start=run,
                               observation_time='10/1/2026 22:41:55.778-5')
            module.import_population(copy, [observation], logs=Path(directory))
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT position_x,position_y FROM content_creature WHERE id1=234649').fetchall(),
                                 [(8916.85, -4738.5)])
            self.assertEqual(module.import_population(copy, [observation], logs=Path(directory))['2813']['already_present'], 1)

    def test_late_boss_observation_loses_to_placeholder_and_missing_log_fails(self):
        with tempfile.TemporaryDirectory() as directory, closing(sqlite3.connect(':memory:')) as db, db:
            db.executescript('''
                CREATE TABLE content_creature(guid INTEGER PRIMARY KEY,id1 INTEGER,map INTEGER,Comment TEXT);
                CREATE TABLE tdb_creature(id INTEGER,map INTEGER);
                CREATE TABLE log_derived_population(guid INTEGER PRIMARY KEY,observation_json TEXT);
                CREATE TABLE raid_encounter_creature(map_id INTEGER,creature_entry INTEGER,encounter_id INTEGER);
                INSERT INTO raid_encounter_creature VALUES(2825,241812,3207);
                INSERT INTO content_creature VALUES(100,241812,2825,'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP');
                INSERT INTO content_creature VALUES(200,241812,2825,'LOG_DERIVED_POPULATION');
            ''')
            record = {'source_file': 'pull.txt', 'source_run_start': '10/12/2026 23:31:28.760-5',
                      'observation_time': '10/12/2026 23:38:22.053-5'}
            db.execute('INSERT INTO log_derived_population VALUES (?,?)', (200, json.dumps(record)))
            with self.assertRaises(FileNotFoundError):
                module.retire_encounter_duplicates(db, Path(directory))
            (Path(directory) / 'pull.txt').write_text(
                '10/12/2026 23:31:28.760-5  CHALLENGE_MODE_START\n'
                '10/12/2026 23:38:20.189-5  ENCOUNTER_START,3207,"The Hoardmonger"\n')
            self.assertEqual(module.retire_encounter_duplicates(db, Path(directory)), [(2825, 241812, 200)])
            self.assertEqual(db.execute('SELECT guid FROM content_creature').fetchall(), [(100,)])

    def test_projects_tdb_template_and_models_on_copy(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'world.db'
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(source) as db:
                for table, data in fixture.items():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {table} VALUES ({','.join('?' for _ in row)})", row)
            shutil.copyfile(source, copy)
            report = module.import_population(copy, [record('tdb', 100, npc=189886), record('absent', 200, npc=99999999)])
            self.assertEqual(report['2521']['templates_from_tdb'], [189886])
            self.assertEqual(report['2521']['templates_absent'], [99999999])
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT entry FROM content_creature_template').fetchall(), [(189886,)])
                self.assertEqual(db.execute('SELECT CreatureID,CreatureDisplayID FROM content_creature_template_model').fetchall(),
                                 db.execute('SELECT CreatureID,CreatureDisplayID FROM tdb_creature_template_model').fetchall())
                self.assertEqual(db.execute('SELECT DifficultyID FROM content_creature_template_difficulty ORDER BY DifficultyID').fetchall(), [(1,), (2,), (23,)])
                self.assertEqual(db.execute('SELECT entry,policy FROM log_derived_templates').fetchall(), [(189886, module.POLICY)])
                self.assertEqual(db.execute('SELECT table_name,row_key FROM log_derived_template_rows ORDER BY table_name,row_key').fetchall(),
                    [('content_creature_template', '[189886]'),
                     ('content_creature_template_difficulty', '[189886,1]'),
                     ('content_creature_template_difficulty', '[189886,23]'),
                     ('content_creature_template_difficulty', '[189886,2]'),
                     ('content_creature_template_model', '[189886,0]')])
            with sqlite3.connect(source) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature_template').fetchone()[0], 0)

    def test_authored_matching_maximizes_matches_before_distance(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(copy) as db:
                for table, data in fixture.items():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {table} VALUES ({','.join('?' for _ in row)})", row)
                db.executemany('INSERT INTO tdb_creature(guid,id,map,position_x,position_y,position_z) VALUES(?,189886,2521,?,20,30)', [(1, 0), (2, 18)])
            report = module.import_population(copy, [record('flexible', 9, npc=189886), record('constrained', 0, npc=189886)])
            self.assertEqual((report['2521']['already_present'], report['2521']['imported']), (2, 0))
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature').fetchone()[0], 0)

    def test_assigned_authored_row_still_excludes_three_yard_duplicates(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(copy) as db:
                for table, data in fixture.items():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {table} VALUES ({','.join('?' for _ in row)})", row)
                db.executemany('INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z) VALUES(?,?,?,?,20,30)',
                               [(63813220, 236525, 2813, 0), (63993120, 261557, 2993, 0),
                                (63993117, 261557, 2993, 10)])
            inputs = []
            for name, npc, map_id, x in [('matched-a',236525,2813,0), ('duplicate-a',236525,2813,0.26),
                                         ('matched-b',261557,2993,0), ('duplicate-b',261557,2993,0.79),
                                         ('matched-c',261557,2993,10), ('duplicate-c',261557,2993,11.52)]:
                inputs.append(dict(record(name, x, npc=npc), map=map_id))
            report = module.import_population(copy, inputs)
            self.assertEqual((report['2813']['already_present'], report['2813']['imported']), (1, 0))
            self.assertEqual((report['2993']['already_present'], report['2993']['imported']), (2, 0))
            self.assertEqual([g['reason'] for g in report['2813']['gaps']], ['AUTHORED_SPAWN_NEARBY_AMBIGUOUS'])
            self.assertEqual([g['reason'] for g in report['2993']['gaps']], ['AUTHORED_SPAWN_NEARBY_AMBIGUOUS'] * 2)
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature').fetchone()[0], 3)

    def test_nearby_slots_are_not_collapsed_or_matched_twice(self):
        fixture = json.loads((Path(__file__).parent / 'tests/fixtures/log-population-tdb.json').read_text())
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(copy) as db:
                for table, data in fixture.items():
                    db.execute(data['sql'])
                    for row in data['rows']:
                        db.execute(f"INSERT INTO {table} VALUES ({','.join('?' for _ in row)})", row)
                db.execute('INSERT INTO tdb_creature(guid,id,map,position_x,position_y,position_z) VALUES(1,189886,2521,100,20,30)')
            centroid = dict(record('two', 101, npc=189886), identity_ambiguous=True,
                            position_method='candidate_centroid', candidate_spread_yards=4.0)
            report = module.import_population(copy, [record('one', 100, npc=189886), centroid,
                                                      record('three', 200, npc=189886), record('four', 201, npc=189886)])
            self.assertEqual((report['2521']['already_present'], report['2521']['imported']), (1, 2))
            self.assertIn('AUTHORED_SPAWN_NEARBY_AMBIGUOUS', [gap['reason'] for gap in report['2521']['gaps']])
            with sqlite3.connect(copy) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature').fetchone()[0], 2)

    def test_copy_only_import_preserves_authored_and_tags_sourced_row(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'world.db'
            copy = Path(directory) / 'world-copy.db'
            with sqlite3.connect(source) as db:
                db.executescript('''
                    CREATE TABLE keystone_affix(ID INTEGER, Name_lang TEXT);
                    CREATE TABLE raid_encounter_creature(map_id INTEGER, creature_entry INTEGER, encounter_id INTEGER);
                    CREATE TABLE spell_name(ID INTEGER, Name_lang TEXT);
                    CREATE TABLE spell_effect(SpellID INTEGER, Effect INTEGER, EffectMiscValue_0 INTEGER);
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
                self.assertEqual(json.loads(db.execute('SELECT observation_json FROM log_derived_population').fetchone()[0])['observation_event'], 'SPELL_DAMAGE')
            self.assertEqual(module.import_population(copy, [record('new', 100)])['2521']['already_present'], 1)
            with sqlite3.connect(source) as db:
                self.assertEqual(db.execute('SELECT COUNT(*) FROM content_creature').fetchone()[0], 1)


if __name__ == '__main__':
    unittest.main()
