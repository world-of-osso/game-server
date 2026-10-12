import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import retail_content
from content_common import Report


class BattlegroundContentTests(unittest.TestCase):
    def test_mainline_warsong_spawns_at_authored_none_difficulty(self):
        conn = sqlite3.connect(':memory:')
        conn.execute('CREATE TABLE map (ID INTEGER, InstanceType INTEGER)')
        conn.executemany('INSERT INTO map VALUES (?, ?)', [(0, 0), (1, 0), (34, 1), (670, 1), (2106, 3), (2107, 3), (761, 3), (726, 3), (566, 3), (998, 3), (727, 3)])
        conn.execute('CREATE TABLE map_difficulty (MapID INTEGER, DifficultyID INTEGER)')
        conn.executemany('INSERT INTO map_difficulty VALUES (?, ?)', [(34, 1), (670, 1)])
        retail_content.create_retail_map_table(conn)
        for map_id in (2106, 2107, 761, 726, 566, 998, 727):
            self.assertEqual(conn.execute('SELECT difficulty FROM temp.retail_map WHERE map_id=?', (map_id,)).fetchall(), [(0,)])

    def test_mainline_warsong_creature_keeps_authored_coordinates(self):
        conn = sqlite3.connect(':memory:')
        conn.executescript('''
            CREATE TEMP TABLE retail_map (map_id INTEGER, difficulty INTEGER);
            INSERT INTO retail_map VALUES (2106, 0);
            CREATE TABLE creature (guid INTEGER PRIMARY KEY, id1 INTEGER, map INTEGER, zoneId INTEGER,
                areaId INTEGER, equipment_id INTEGER, position_x REAL, position_y REAL, position_z REAL,
                orientation REAL, spawntimesecs INTEGER, wander_distance REAL, currentwaypoint INTEGER,
                MovementType INTEGER, unit_flags INTEGER, ScriptName TEXT, VerifiedBuild INTEGER);
            CREATE TABLE tdb_creature (guid INTEGER, id INTEGER, map INTEGER, zoneId INTEGER, areaId INTEGER,
                equipment_id INTEGER, position_x REAL, position_y REAL, position_z REAL, orientation REAL,
                spawntimesecs INTEGER, wander_distance REAL, currentwaypoint INTEGER, MovementType INTEGER,
                unit_flags INTEGER, ScriptName TEXT, VerifiedBuild INTEGER, modelid INTEGER,
                spawnDifficulties TEXT, PhaseId INTEGER, PhaseGroup INTEGER, terrainSwapMap INTEGER);
            CREATE TABLE creature_display_info (ID INTEGER);
        ''')
        # Exact TDB1210 creature row7000000; no generated placements.
        conn.execute("INSERT INTO tdb_creature VALUES (7000000,61750,2106,10218,12835,0,1403,1345.57,329.395,2.34634,7200,10,0,1,NULL,'',50000,0,'0',0,0,-1)")
        retail_content.build_spawns(conn, Report())
        self.assertEqual(conn.execute('SELECT id1,map,position_x,position_y,position_z,spawnDifficulties FROM content_creature').fetchall(),
                         [(61750,2106,1403,1345.57,329.395,'0')])


if __name__ == '__main__':
    unittest.main()
