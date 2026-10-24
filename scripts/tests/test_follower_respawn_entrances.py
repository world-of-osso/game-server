"""Project admission from authored initial respawn POIs is not a native portal claim."""
from contextlib import closing
import copy
import json
import math
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

POLICY = 'FOLLOWER_ENTRANCE_FROM_INITIAL_RESPAWN_POI'


@unittest.skipUnless('FOLLOWER_RESPAWN_ENTRANCE_DB' in os.environ, 'requires an explicit prepared database copy')
class InitialRespawnEntranceAcceptance(unittest.TestCase):
    def test_two_project_admissions_keep_exact_initial_poi_and_native_portal_gaps(self):
        path = Path(os.environ['FOLLOWER_RESPAWN_ENTRANCE_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT ID,MapID,LocX,LocY,LocZ,Facing,Comment FROM tdb_world_safe_locs '
                              'WHERE Comment LIKE ? ORDER BY MapID', ('%' + POLICY + '%',)).fetchall()
            self.assertEqual([r[1] for r in rows], [2649, 2669], 'two initial respawn project admissions are missing')
            expected = {
                2649: (8073, (3022.30737, 983.28125, 514.286804), 10001096),
                2669: (8072, (-1600.50525, -807.380188, -1247.68042), 10002867),
            }
            for safe_id, map_id, x, y, z, facing, comment in rows:
                poi, xyz, anchor_guid = expected[map_id]
                self.assertEqual(safe_id, 81_000_000 + map_id)
                self.assertEqual((x, y, z), xyz)
                self.assertIn(str(poi), comment)
                self.assertIn(f'FOLLOWER_ENTRANCE_NATIVE_PORTAL_UNSOURCED_{map_id}', comment)
                anchor = db.execute('SELECT position_x,position_y FROM tdb_creature WHERE guid=? AND map=?',
                                    (anchor_guid, map_id)).fetchone()
                self.assertIsNotNone(anchor)
                self.assertAlmostEqual(facing, math.degrees(math.atan2(anchor[1] - y, anchor[0] - x)) % 360, places=8)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual(receipt['policy'], POLICY)
        self.assertFalse(receipt['exact_portal_xyz'])
        self.assertEqual(set(receipt['native_portal_gaps']), {'FOLLOWER_ENTRANCE_NATIVE_PORTAL_UNSOURCED_2649',
                                                           'FOLLOWER_ENTRANCE_NATIVE_PORTAL_UNSOURCED_2669'})
        self.assertTrue(receipt['db2_authentication']['AreaPOI']['content_authenticated'])
        self.assertEqual({r['source_poi']['ID'] for r in receipt['maps']}, {'8072', '8073'})
        for row in receipt['maps']:
            self.assertEqual((row['source_poi']['PlayerConditionID'], row['source_poi']['WorldStateID']), ('0', '0'))
        calibration = receipt['findings'][0]
        self.assertGreater(calibration['distance_min_yards'], 12)
        self.assertLess(calibration['distance_min_yards'], 14)
        self.assertGreater(calibration['distance_max_yards'], 50)
        self.assertLess(calibration['distance_max_yards'], 52)

    @unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ, 'requires authenticated AreaPOI CSV')
    def test_only_unique_unconditional_approved_initial_pois_are_eligible(self):
        from follower_entrances import read_respawn_pois, select_initial_respawn_poi
        pois, _, _ = read_respawn_pois(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        self.assertEqual(select_initial_respawn_poi(pois, 2649)['ID'], '8073')
        self.assertEqual(select_initial_respawn_poi(pois, 2669)['ID'], '8072')
        for field in ('PlayerConditionID', 'WorldStateID'):
            conditional = copy.deepcopy(pois)
            next(p for p in conditional if p['ID'] == '8073')[field] = '1'
            with self.assertRaisesRegex(ValueError, 'unique approved unconditional'):
                select_initial_respawn_poi(conditional, 2649)
        duplicate = copy.deepcopy(pois)
        duplicate.append(dict(next(p for p in pois if p['ID'] == '8072'), ID='999999'))
        with self.assertRaisesRegex(ValueError, 'unique approved unconditional'):
            select_initial_respawn_poi(duplicate, 2669)
        with self.assertRaisesRegex(ValueError, 'approved only'):
            select_initial_respawn_poi(pois, 2805)

    @unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ, 'requires authenticated AreaPOI CSV')
    def test_source_reader_refuses_changed_or_unauthenticated_poi_data(self):
        from follower_entrances import read_respawn_pois
        source = Path(os.environ['FOLLOWER_PLACEMENT_DB2'])
        receipt = json.loads((source / 'receipt.json').read_text())
        with tempfile.TemporaryDirectory(prefix='follower-poi-auth-') as directory:
            target = Path(directory)
            (target / 'receipt.json').write_text(json.dumps(receipt))
            raw = (source / 'AreaPOI.csv').read_bytes()
            (target / 'AreaPOI.csv').write_bytes(raw + b'\n')
            with self.assertRaisesRegex(ValueError, 'authentication/hash'):
                read_respawn_pois(target)
            (target / 'AreaPOI.csv').write_bytes(raw)
            receipt['tables']['AreaPOI']['content_authenticated'] = False
            (target / 'receipt.json').write_text(json.dumps(receipt))
            with self.assertRaisesRegex(ValueError, 'authentication/hash'):
                read_respawn_pois(target)

    def test_sql_reapply_keeps_two_rows_and_refuses_modified_project_pose(self):
        path = Path(os.environ['FOLLOWER_RESPAWN_ENTRANCE_DB']).resolve(strict=True)
        sql = (path.parent / 'delta.sql').read_text()
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as replay:
            replay.execute(source.execute("SELECT sql FROM sqlite_master WHERE name='tdb_world_safe_locs'").fetchone()[0])
            replay.executescript(sql)
            before = replay.execute('SELECT * FROM tdb_world_safe_locs ORDER BY ID').fetchall()
            self.assertEqual(len(before), 2)
            replay.executescript(sql)
            self.assertEqual(replay.execute('SELECT * FROM tdb_world_safe_locs ORDER BY ID').fetchall(), before)
            replay.execute('UPDATE tdb_world_safe_locs SET LocZ=LocZ+1 WHERE ID=81002649')
            replay.commit()
            with self.assertRaises(sqlite3.IntegrityError):
                replay.executescript(sql)
            replay.rollback()
            self.assertEqual(replay.execute('SELECT LocZ FROM tdb_world_safe_locs WHERE ID=81002649').fetchone(),
                             (514.286804 + 1,))


if __name__ == '__main__':
    unittest.main()
