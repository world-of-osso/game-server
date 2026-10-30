#!/usr/bin/env python3
"""Place a log-correlated Darkflame cart boarding point on a database copy.

DARKFLAME_CART_BOARDING_LOG_CORRELATED: TaxiPath9458 node0 supplies the position;
WCL WBYDm1N6JTZzCFnd fight3 pull14 follows that path while players cast Throw
Dynamite423374. The exact boarding trigger and click association are unsourced.
TDB supplies NPC209439 and display118139; the placement and spellclick flag are
inferred for the approved taxi-mount approximation. Never writes canonical world.db.
"""
import argparse
from contextlib import closing
import math
from pathlib import Path
import sqlite3

from encounter_summons import import_base_template

POLICY = 'DARKFLAME_CART_BOARDING_LOG_CORRELATED'
ENTRY, MAP, GUID, PATH = 209439, 2651, 63651999, 9458
SPELLCLICK = 0x01000000
COMMENT = (f'{POLICY}; db2:12.1.5.70448 TaxiPath9458 node0; '
           'wcl:WBYDm1N6JTZzCFnd fight3 pull14 Throw Dynamite423374; '
           'exact boarding trigger and spellclick association unsourced')


def add_cart(db):
    rows = db.execute('SELECT Loc_0,Loc_1,Loc_2 FROM taxi_path_node '
                      'WHERE PathID=? ORDER BY NodeIndex', (PATH,)).fetchall()
    if len(rows) != 32:
        raise ValueError(f'TaxiPath {PATH} expected 32 nodes, found {len(rows)}')
    if db.execute('SELECT 1 FROM content_creature WHERE guid=? OR (map=? AND id1=?)',
                  (GUID, MAP, ENTRY)).fetchone():
        raise ValueError('cart GUID or placement already exists')
    import_base_template(db, ENTRY, '')
    db.execute('UPDATE content_creature_template SET npcflag=? WHERE entry=?', (SPELLCLICK, ENTRY))
    x, y, z = rows[0]
    facing = math.atan2(rows[1][1] - y, rows[1][0] - x) % (2 * math.pi)
    db.execute('''INSERT INTO content_creature
        (guid,id1,map,zoneId,areaId,spawnMask,phaseMask,position_x,position_y,position_z,
         orientation,spawntimesecs,curhealth,VerifiedBuild,Comment,spawnDifficulties)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)''',
        (GUID, ENTRY, MAP, 14882, 15023, 1, 1, x, y, z, facing, 7200, 1, 0, COMMENT, '1'))
    return x, y, z


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file():
        parser.error('use an existing independent database copy, not world.db')
    with closing(sqlite3.connect(args.db)) as db, db:
        location = add_cart(db)
    print(f'{POLICY}: {GUID} at {location}')


if __name__ == '__main__':
    main()
