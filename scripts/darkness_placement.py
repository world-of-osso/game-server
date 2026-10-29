#!/usr/bin/env python3
"""Move Darkflame Cleft's The Darkness (208747) to its logged fight position on a DB copy.

Source: WCL report WBYDm1N6JTZzCFnd fight 3 (pull 16), actor 51 (gameID 208747),
first boss sample at ENCOUNTER_START +975 ms: x=-2799, y=39193, facing=-746.
WCL (x, y) maps to WoW (y/100, -x/100); facing is centiradians.
Z is the baked map-2651 navmesh floor at that x/y (follower_route endpoint 87.26754),
reachable from Ol' Waxbeard's room. The Journal-UiMap placement at (188.5, -547.8)
is replaced. Never writes canonical world.db; the lead publishes.
"""
import argparse
import math
from contextlib import closing
from pathlib import Path
import sqlite3

POLICY = 'DARKNESS_PLACEMENT_FROM_WCL_ENCOUNTER_START_SAMPLE'
MAP = 2651
ENTRY = 208747
WCL_X, WCL_Y, WCL_FACING = -2799, 39193, -746
FLOOR_Z = 87.26754
COMMENT = (f'{POLICY}; wcl:WBYDm1N6JTZzCFnd fight3 pull16 actor51 ENCOUNTER_START+975ms '
           f'x={WCL_X} y={WCL_Y} facing={WCL_FACING}; floor=navmesh2651 {FLOOR_Z}')


def logged_pose():
    orientation = math.fmod(WCL_FACING / 100, 2 * math.pi) % (2 * math.pi)
    return WCL_Y / 100, -WCL_X / 100, FLOOR_Z, orientation


def place(connection):
    rows = connection.execute('SELECT guid FROM content_creature WHERE map=? AND id1=?',
                              (MAP, ENTRY)).fetchall()
    if len(rows) != 1:
        raise ValueError(f'expected one map{MAP} {ENTRY} placement, found {len(rows)}')
    x, y, z, o = logged_pose()
    connection.execute('UPDATE content_creature SET position_x=?,position_y=?,position_z=?,'
                       'orientation=?,Comment=? WHERE guid=?', (x, y, z, o, COMMENT, rows[0][0]))
    return rows[0][0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file():
        parser.error('use an existing independent database copy, not world.db')
    with closing(sqlite3.connect(args.db)) as connection, connection:
        guid = place(connection)
    print(f'{POLICY}: guid {guid} -> {logged_pose()}')


if __name__ == '__main__':
    main()
