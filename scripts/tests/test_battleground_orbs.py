import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import battleground_summons


class BattlegroundOrbTests(unittest.TestCase):
    def database(self):
        conn = sqlite3.connect(':memory:')
        conn.executescript('''
            CREATE TABLE tdb_gameobject_template (entry INTEGER PRIMARY KEY, type INTEGER,
                displayId INTEGER, name TEXT, size REAL, Data1 INTEGER);
            CREATE TABLE content_gameobject_template (entry INTEGER PRIMARY KEY, type INTEGER,
                displayId INTEGER, name TEXT, size REAL, Data1 INTEGER);
            CREATE TABLE game_object_display_info (ID INTEGER PRIMARY KEY);
            INSERT INTO game_object_display_info VALUES (11533);
            CREATE TABLE content_gameobject (guid INTEGER PRIMARY KEY, id INTEGER, map INTEGER,
                position_x REAL, position_y REAL, position_z REAL, orientation REAL,
                rotation3 REAL, spawntimesecs INTEGER, animprogress INTEGER, state INTEGER,
                spawnDifficulties TEXT, Comment TEXT);
            INSERT INTO content_gameobject (guid,id,map,position_x,position_y,position_z)
                VALUES (12,142075,0,-9462.29,42.087,56.9666);
        ''')
        # Exact TDB1210 template fields; positions are TC OnInit scripted summons,
        # not static TDB rows. Storage GUIDs must not claim to be source spawn IDs.
        conn.executemany("INSERT INTO tdb_gameobject_template VALUES (?,24,11533,'Orb of Power',1.0,?)",
                         [(212091,121164),(212092,121175),(212093,121176),(212094,121177)])
        return conn

    def test_source_orbs_materialize_idempotently_and_preserve_unrelated_rows(self):
        conn = self.database()
        before = conn.execute('SELECT * FROM content_gameobject WHERE map=0').fetchall()
        self.assertEqual(battleground_summons.import_kotmogu_orbs(conn), 4)
        expected = [
            (212091,1716.9479,1250.0173,13.3266),
            (212092,1850.217,1416.8229,13.3382),
            (212093,1716.8923,1416.618,13.2056),
            (212094,1850.1666,1250.118,13.2056),
        ]
        self.assertEqual(conn.execute('SELECT id,position_x,position_y,position_z FROM content_gameobject WHERE map=998 ORDER BY id').fetchall(),expected)
        self.assertEqual(conn.execute('SELECT entry,type,displayId,Data1 FROM content_gameobject_template ORDER BY entry').fetchall(),
                         [(212091,24,11533,121164),(212092,24,11533,121175),(212093,24,11533,121176),(212094,24,11533,121177)])
        rows = conn.execute('SELECT * FROM content_gameobject ORDER BY guid').fetchall()
        self.assertEqual(battleground_summons.import_kotmogu_orbs(conn),4)
        self.assertEqual(conn.execute('SELECT * FROM content_gameobject ORDER BY guid').fetchall(),rows)
        self.assertEqual(conn.execute('SELECT * FROM content_gameobject WHERE map=0').fetchall(),before)

    def test_missing_authored_template_fails_before_any_content_write(self):
        conn = self.database()
        conn.execute('DELETE FROM tdb_gameobject_template WHERE entry=212093')
        with self.assertRaisesRegex(ValueError,'212093'):
            battleground_summons.import_kotmogu_orbs(conn)
        self.assertEqual(conn.execute('SELECT COUNT(*) FROM content_gameobject WHERE map=998').fetchone()[0],0)
        self.assertEqual(conn.execute('SELECT COUNT(*) FROM content_gameobject_template').fetchone()[0],0)


if __name__ == '__main__':
    unittest.main()
