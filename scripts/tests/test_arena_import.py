import importlib.util
import sqlite3
import sys
import unittest
from contextlib import closing
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'import-arenas.py'
sys.path.insert(0, str(SCRIPT.parent))


class ArenaImportTest(unittest.TestCase):
    def test_source_join_does_not_guess_map_ids_or_start_coordinates(self):
        spec = importlib.util.spec_from_file_location('arena_import', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('''
                CREATE TABLE map(ID INTEGER,InstanceType INTEGER,WdtFileDataID INTEGER);
                CREATE TABLE tdb_battleground_template(ID INTEGER,AllianceStartLoc INTEGER,HordeStartLoc INTEGER,Weight INTEGER);
                CREATE TABLE tdb_world_safe_locs(ID INTEGER,MapID INTEGER);
                CREATE TABLE tdb_gameobject(id INTEGER,map INTEGER);
                INSERT INTO tdb_gameobject VALUES(185918,572),(185917,572),(192642,617),(192643,617);
                INSERT INTO map VALUES(572,4,789984),(617,4,780309),(2106,3,790517);
                INSERT INTO tdb_world_safe_locs VALUES(1258,572),(1259,572),(1362,617),(1363,617);
                INSERT INTO tdb_battleground_template VALUES(8,1258,1259,1),(10,1362,1363,1),(1014,7051,7050,1);
            ''')
            rows = module.import_catalog(db, SCRIPT.parent / 'reference/arena-12.1.0.69933/BattlemasterList.csv', {789984})
            self.assertEqual(rows, 1)
            module.import_gates(db, SCRIPT.parent / 'reference/arena-12.1.0.69933/arena-gates.json')
            self.assertEqual(db.execute('SELECT map_id,entry,action,despawn_secs FROM arena_gate ORDER BY entry').fetchall(), [(572,185917,'open',5),(572,185918,'open',5)])
            self.assertEqual(db.execute('SELECT battlemaster_id,map_id,start_a,start_b,min_level,max_level,weight FROM arena_map').fetchall(), [(8,572,1258,1259,10,90,1)])
            self.assertEqual(module.import_catalog(db, SCRIPT.parent / 'reference/arena-12.1.0.69933/BattlemasterList.csv', {789984,780309}), 2)
            db.execute('UPDATE tdb_world_safe_locs SET MapID=572 WHERE ID=1363')
            with self.assertRaisesRegex(ValueError, 'start maps disagree'):
                module.import_catalog(db, SCRIPT.parent / 'reference/arena-12.1.0.69933/BattlemasterList.csv', {789984,780309})
            self.assertEqual(db.execute('SELECT count(*) FROM arena_map').fetchone()[0], 2)


class ArenaContentTest(unittest.TestCase):
    def test_promotion_preserves_unrelated_rows_and_copies_source_doors(self):
        spec = importlib.util.spec_from_file_location('arena_import_content', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('''
                CREATE TABLE arena_map(map_id INTEGER);
                INSERT INTO arena_map VALUES(572);
                CREATE TABLE tdb_gameobject(guid INTEGER,id INTEGER,map INTEGER,terrainSwapMap INTEGER,phaseUseFlags INTEGER,PhaseId INTEGER,PhaseGroup INTEGER,spawnDifficulties TEXT,position_x REAL);
                INSERT INTO tdb_gameobject VALUES(1,185918,572,-1,0,0,0,'0',1294.94),(2,185917,572,-1,0,0,0,'0',1278.65);
                CREATE TABLE tdb_gameobject_template(entry INTEGER PRIMARY KEY,type INTEGER,name TEXT);
                INSERT INTO tdb_gameobject_template VALUES(185918,0,'Doodad_PVP_Lordaeron_Door02'),(185917,0,'Doodad_PVP_Lordaeron_Door01');
                CREATE TABLE content_gameobject(guid INTEGER PRIMARY KEY,id INTEGER,map INTEGER,spawnDifficulties TEXT,position_x REAL);
                INSERT INTO content_gameobject VALUES(123,456,0,'0',5.5);
                CREATE TABLE content_gameobject_template(entry INTEGER PRIMARY KEY,type INTEGER,name TEXT);
                INSERT INTO content_gameobject_template VALUES(456,5,'Keep this');
                CREATE TABLE content_reference_loot_template(Entry INTEGER,Item INTEGER);
                INSERT INTO content_reference_loot_template VALUES(77,88);
            ''')
            module.promote_content(db)
            self.assertEqual(db.execute('SELECT * FROM content_gameobject WHERE map=0').fetchall(),[(123,456,0,'0',5.5)])
            self.assertEqual(db.execute('SELECT * FROM content_gameobject_template WHERE entry=456').fetchall(),[(456,5,'Keep this')])
            self.assertEqual(db.execute('SELECT * FROM content_reference_loot_template').fetchall(),[(77,88)])
            self.assertEqual(db.execute('SELECT id,map,spawnDifficulties,position_x FROM content_gameobject WHERE map=572 ORDER BY id').fetchall(),[(185917,572,'0',1278.65),(185918,572,'0',1294.94)])


class DalaranSpoutImportTest(unittest.TestCase):
    def test_only_source_functional_spouts_and_their_flags_models_are_promoted(self):
        spec = importlib.util.spec_from_file_location('arena_spouts', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('''
                CREATE TABLE arena_map(map_id INTEGER); INSERT INTO arena_map VALUES(617);
                CREATE TABLE tdb_creature(guid INTEGER,id INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL,StringId TEXT,terrainSwapMap INTEGER,phaseUseFlags INTEGER,PhaseId INTEGER,PhaseGroup INTEGER);
                INSERT INTO tdb_creature VALUES
                (2000002,28567,617,1369.98,817.288,16.0872,'arena_dalaran_sewers_water_spout_pipe',-1,0,0,0),
                (2000004,28567,617,1292.59,790.221,7.19796,'arena_dalaran_sewers_water_spout_center',-1,0,0,0),
                (2000005,28567,617,1212.83,765.387,16.0948,'arena_dalaran_sewers_water_spout_pipe',-1,0,0,0),
                (7,15214,617,1,2,3,'decorative',-1,0,0,0);
                CREATE TABLE content_creature(guid INTEGER PRIMARY KEY,id1 INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);
                INSERT INTO content_creature VALUES(123,456,0,5,6,7),(124,15214,617,1,2,3);
                CREATE TABLE tdb_creature_template(entry INTEGER PRIMARY KEY,name TEXT,unit_flags INTEGER);
                INSERT INTO tdb_creature_template VALUES(28567,'Water Spout',33817344);
                CREATE TABLE content_creature_template(entry INTEGER PRIMARY KEY,name TEXT,unit_flags INTEGER);
                INSERT INTO content_creature_template VALUES(28567,'Water Spout',0),(456,'Keep this',99);
                CREATE TABLE tdb_creature_template_model(CreatureID INTEGER,Idx INTEGER,CreatureDisplayID INTEGER,DisplayScale REAL,Probability REAL,VerifiedBuild INTEGER);
                INSERT INTO tdb_creature_template_model VALUES(28567,0,169,1,0,69497),(28567,1,11686,1,1,69497);
                CREATE TABLE content_creature_template_model(CreatureID INTEGER,Idx INTEGER,CreatureDisplayID INTEGER,DisplayScale REAL,Probability REAL,VerifiedBuild INTEGER);
                INSERT INTO content_creature_template_model VALUES(456,0,99,1,1,1);
            ''')
            # TDB NULL override means inherit template; the target encodes that sentinel as0.
            db.executescript('''ALTER TABLE tdb_creature ADD COLUMN npcflag INTEGER;
                ALTER TABLE tdb_creature ADD COLUMN unit_flags INTEGER;
                ALTER TABLE content_creature ADD COLUMN npcflag INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE content_creature ADD COLUMN unit_flags INTEGER NOT NULL DEFAULT 0;''')
            self.assertEqual(module.import_spouts(db), 3)
            self.assertEqual(module.promote_dalaran_spouts(db), 3)
            self.assertEqual(db.execute('SELECT guid,id1,map,position_x,position_y,position_z FROM content_creature WHERE guid IN(123,124) ORDER BY guid').fetchall(), [(123,456,0,5,6,7),(124,15214,617,1,2,3)])
            self.assertEqual(db.execute('SELECT guid,id1,position_z FROM content_creature WHERE id1=28567 ORDER BY guid').fetchall(), [(22000002,28567,16.0872),(22000004,28567,7.19796),(22000005,28567,16.0948)])
            self.assertEqual(db.execute('SELECT npcflag,unit_flags FROM content_creature WHERE id1=28567').fetchall(), [(0,0)]*3)
            self.assertEqual(db.execute('SELECT unit_flags FROM content_creature_template WHERE entry=28567').fetchone()[0], 33817344)
            self.assertEqual(db.execute('SELECT CreatureDisplayID,Probability FROM content_creature_template_model WHERE CreatureID=28567 ORDER BY Idx').fetchall(), [(169,0),(11686,1)])
            self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=456').fetchall(), [(456,0,99,1,1,1)])
            self.assertEqual(module.promote_dalaran_spouts(db), 3)
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE id1=28567').fetchone()[0], 3)


if __name__ == '__main__':
    unittest.main()
