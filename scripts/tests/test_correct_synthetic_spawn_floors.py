import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/correct_synthetic_spawn_floors.py'
GEOMETRY = Path('/home/osso/Projects/world-of-osso/game-engine/data')
NAV = Path('/home/osso/Projects/world-of-osso/game-server/data/ground/navmesh')
WORLD = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')


class SyntheticFloorTest(unittest.TestCase):
    def test_real_windrunner_route_corrects_two_floor_spawns_without_self_anchor(self):
        resolver = os.environ['FLOOR_RESOLVER_BIN']
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            db_path = root / 'world.db'
            with sqlite3.connect(f'file:{WORLD}?mode=ro', uri=True) as original:
                wdt = original.execute('SELECT WdtFileDataID FROM map WHERE ID=2805').fetchone()[0]
            with sqlite3.connect(db_path) as db:
                db.executescript('''CREATE TABLE map (ID INTEGER, WdtFileDataID INTEGER);
                    CREATE TABLE tdb_creature (guid INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);
                    CREATE TABLE tdb_gameobject (guid INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL);
                    CREATE TABLE content_creature (guid INTEGER,id1 INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL,Comment TEXT);''')
                db.execute('INSERT INTO map VALUES (2805,?)', (wdt,))
                db.executemany('INSERT INTO content_creature VALUES (?,?,2805,5326.72,-3218.82,97.504312,?)', [
                    (63805110, 232071, 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS; sample'),
                    (63805178, 234673, 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS; sample'),
                    (63805222, 238099, 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS; sample'),
                ])
            route = root / 'route.json'
            route.write_text('[]')
            report = root / 'report.json'
            command = ['python3', str(SCRIPT), '--world-db', str(db_path), '--resolver', resolver,
                       '--geometry', str(GEOMETRY), '--navigation', str(NAV), '--routes', str(route),
                       '--output', str(report)]
            subprocess.run(command, check=True)
            without_route = json.loads(report.read_text())
            self.assertEqual(without_route['changes'], [])
            self.assertEqual(len(without_route['unresolved']), 3)
            route.write_text(json.dumps([dict(map=2805, x=5328.386, y=-3216.4063, z=101.156075)]))
            subprocess.run(command, check=True)
            result = json.loads(report.read_text())
            self.assertEqual(result['scanned'], 3)
            self.assertEqual([row['guid'] for row in result['changes']], [63805110, 63805178, 63805222])
            self.assertTrue(all(abs(row['new_z'] - 101.281906) < 0.001 for row in result['changes']))
            with sqlite3.connect(db_path) as db:
                self.assertEqual(db.execute('SELECT DISTINCT position_z FROM content_creature').fetchall(), [(97.504312,)])
            subprocess.run(['python3', str(SCRIPT), '--world-db', str(db_path),
                            '--output', str(report), '--apply'], check=True)
            with sqlite3.connect(db_path) as db:
                self.assertAlmostEqual(db.execute('SELECT MIN(position_z) FROM content_creature').fetchone()[0], 101.281906, places=4)
            subprocess.run(command, check=True)
            self.assertEqual(json.loads(report.read_text())['changes'], [])
