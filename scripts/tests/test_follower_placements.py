"""Actual Neltharus placement/route acceptance on an explicitly supplied DB copy.

FOLLOWER_PLACEMENT_DB must name a fresh canonical-patched copy. Route proof uses
FOLLOWER_PLACEMENT_ROUTE (navigation's follower_route executable) and the staged
FOLLOWER_PLACEMENT_NAV directory; neither synthetic geometry nor straight-line
reachability stands in for the server NavigationMap query.
"""
import math
import os
from pathlib import Path
import sqlite3
import subprocess
import unittest
from contextlib import closing


class NeltharusPlacementAcceptance(unittest.TestCase):
    def test_all_four_primary_bosses_have_normal_spawns_reachable_from_source_entrance(self):
        database = Path(os.environ["FOLLOWER_PLACEMENT_DB"]).resolve(strict=True)
        with closing(sqlite3.connect(database.as_uri() + "?mode=ro", uri=True)) as db:
            entrance = db.execute(
                "SELECT LocX,LocY,LocZ FROM tdb_world_safe_locs WHERE ID=100028 AND MapID=2519"
            ).fetchone()
            self.assertIsNotNone(entrance, "TDB1210 Neltharus entrance100028 missing")
            bosses = db.execute(
                "SELECT id1,position_x,position_y,position_z FROM content_creature "
                "WHERE map=2519 AND id1 IN(181861,189901,189478,189340) "
                "AND (','||replace(spawnDifficulties,' ','')||',') LIKE '%,1,%' ORDER BY id1,guid"
            ).fetchall()
        self.assertEqual(
            {row[0] for row in bosses}, {181861, 189901, 189478, 189340},
            "Neltharus missing one or more LittleWigs2777a9d/DB2 primary placements",
        )
        route = Path(os.environ["FOLLOWER_PLACEMENT_ROUTE"]).resolve(strict=True)
        nav = Path(os.environ["FOLLOWER_PLACEMENT_NAV"]).resolve(strict=True)
        start = [entrance[0], entrance[2], -entrance[1]]
        for entry, x, y, z in bosses:
            with self.subTest(entry=entry):
                self.assertTrue(all(math.isfinite(v) for v in (x, y, z)))
                end = [x, z, -y]
                result = subprocess.run(
                    [str(route), str(nav), *map(str, start), *map(str, end)],
                    capture_output=True, text=True, check=False, timeout=120,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                corners = [tuple(map(float, line.split(','))) for line in result.stdout.splitlines()]
                self.assertTrue(corners, "server navigation returned no corners")
                self.assertTrue(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in corners))
                # Same nominal nearest-polygon horizontal extent as NavigationMap.
                self.assertLessEqual(abs(corners[-1][0] - end[0]), 0.8)
                self.assertLessEqual(abs(corners[-1][2] - end[2]), 0.8)
                self.assertLessEqual(abs(corners[-1][1] - end[1]), 5.0)


if __name__ == "__main__":
    unittest.main()
