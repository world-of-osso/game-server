"""Exact native portal coordinates must become explicitly derived entrances, COPY only."""
from contextlib import closing
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
            rows = db.execute('SELECT ID,MapID,LocX,LocY,LocZ,Comment FROM tdb_world_safe_locs '
                              'WHERE Comment LIKE ? ORDER BY MapID', ('%' + POLICY + '%',)).fetchall()
            self.assertEqual([r[1] for r in rows], list(MAPS), 'eight sourced interior portals are not prepared entrances')
            for safe_id, map_id, x, y, z, comment in rows:
                with self.subTest(map_id=map_id):
                    portals = db.execute("SELECT g.guid,g.position_x,g.position_y,g.position_z FROM tdb_gameobject g "
                                         "JOIN tdb_gameobject_template t ON t.entry=g.id WHERE g.map=? AND t.name='Instance Portal'",
                                         (map_id,)).fetchall()
                    self.assertEqual(len(portals), 1)
                    self.assertEqual((x, y, z), portals[0][1:])
                    self.assertIn(str(portals[0][0]), comment)


if __name__ == '__main__':
    unittest.main()
