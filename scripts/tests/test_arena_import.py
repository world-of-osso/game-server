import importlib.util
import sqlite3
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'import-arenas.py'
sys.path.insert(0, str(SCRIPT.parent))


class ArenaImportTest(unittest.TestCase):
    def test_source_join_does_not_guess_map_ids_or_start_coordinates(self):
        spec = importlib.util.spec_from_file_location('arena_import', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with sqlite3.connect(':memory:') as db:
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
        with sqlite3.connect(':memory:') as db:
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


if __name__ == '__main__':
    unittest.main()
