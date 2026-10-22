"""Exact native portal coordinates must become explicitly derived entrances, COPY only."""
from contextlib import closing
import math
import os
from pathlib import Path
import sqlite3
import unittest

MAPS = (2651, 2652, 2660, 2661, 2662, 2773, 2825, 2830)
POLICY = 'FOLLOWER_ENTRANCE_FROM_EXIT_PORTAL'


@unittest.skipUnless('FOLLOWER_ENTRANCE_DB' in os.environ, 'requires an explicit database copy')
class FollowerEntranceAcceptance(unittest.TestCase):
    def test_eight_native_instance_portals_preserve_exact_entrance_xyz(self):
        path = Path(os.environ['FOLLOWER_ENTRANCE_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT ID,MapID,LocX,LocY,LocZ,Comment,Facing FROM tdb_world_safe_locs '
                              'WHERE Comment LIKE ? ORDER BY MapID', ('%' + POLICY + '%',)).fetchall()
            self.assertEqual([r[1] for r in rows], list(MAPS), 'eight sourced interior portals are not prepared entrances')
            self.assertEqual([r[0] for r in rows], [81_000_000 + map_id for map_id in MAPS])
            anchors = {2651: 10002585, 2652: 10002180, 2660: 10003760, 2661: 10002306,
                       2662: 10001505, 2773: 10006320, 2825: 11000473, 2830: 10006650}
            for safe_id, map_id, x, y, z, comment, facing in rows:
                with self.subTest(map_id=map_id):
                    portals = db.execute("SELECT g.guid,g.position_x,g.position_y,g.position_z FROM tdb_gameobject g "
                                         "JOIN tdb_gameobject_template t ON t.entry=g.id WHERE g.map=? AND t.name='Instance Portal'",
                                         (map_id,)).fetchall()
                    self.assertEqual(len(portals), 1)
                    self.assertEqual((x, y, z), portals[0][1:])
                    self.assertIn(str(portals[0][0]), comment)
                    anchor = db.execute('SELECT position_x,position_y FROM tdb_creature WHERE guid=? AND map=?',
                                        (anchors[map_id], map_id)).fetchone()
                    self.assertIsNotNone(anchor)
                    expected_facing = math.degrees(math.atan2(anchor[1] - y, anchor[0] - x)) % 360
                    self.assertAlmostEqual(facing, expected_facing, places=8)
                    if map_id == 2662:
                        self.assertIn('FOLLOWER_ENTRANCE_FACING_SOURCED_PATH_HELPER', comment)

    def test_sql_reapply_is_idempotent_and_refuses_reserved_id_collision(self):
        path = Path(os.environ['FOLLOWER_ENTRANCE_DB']).resolve(strict=True)
        sql = Path(os.environ['FOLLOWER_ENTRANCE_SQL']).read_text()
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as replay:
            replay.execute(source.execute("SELECT sql FROM sqlite_master WHERE name='tdb_world_safe_locs'").fetchone()[0])
            replay.executescript(sql)
            before = replay.execute('SELECT * FROM tdb_world_safe_locs ORDER BY ID').fetchall()
            replay.executescript(sql)
            self.assertEqual(replay.execute('SELECT * FROM tdb_world_safe_locs ORDER BY ID').fetchall(), before)
            self.assertEqual(len(before), 8)
            replay.execute('UPDATE tdb_world_safe_locs SET MapID=2519 WHERE ID=81002660')
            replay.commit()
            with self.assertRaises(sqlite3.IntegrityError):
                replay.executescript(sql)
            replay.rollback()
            self.assertEqual(replay.execute('SELECT MapID FROM tdb_world_safe_locs WHERE ID=81002660').fetchone(), (2519,))


if __name__ == '__main__':
    unittest.main()
