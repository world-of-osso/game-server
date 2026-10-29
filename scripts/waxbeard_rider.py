#!/usr/bin/env python3
"""Remove the Journal-UiMap ground placement of Ol' Waxbeard (210149) on a DB copy.

TDB vehicle_template_accessory seats 210149 on its mount 210153 (seat 0); the rider
must come from the mount's accessory install, not a second standing actor.
Requires tdb_vehicle_template_accessory (import-vehicle-accessories.py) first.
Never writes canonical world.db; the lead publishes.
"""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3

MAP = 2651
RIDER = 210149
MOUNT = 210153
PLACEMENT_GUIDS = range(63651000, 63652000)


def remove_ground_rider(connection):
    seated = connection.execute('SELECT 1 FROM tdb_vehicle_template_accessory '
                                'WHERE entry=? AND accessory_entry=? AND seat_id=0',
                                (MOUNT, RIDER)).fetchone()
    if seated is None:
        raise ValueError(f'{RIDER} is not seat0 of {MOUNT}; import accessories first')
    rows = [guid for (guid,) in connection.execute(
        'SELECT guid FROM content_creature WHERE map=? AND id1=?', (MAP, RIDER))]
    if len(rows) != 1 or rows[0] not in PLACEMENT_GUIDS:
        raise ValueError(f'expected one generated map{MAP} {RIDER} placement, found {rows}')
    connection.execute('DELETE FROM content_creature WHERE guid=?', (rows[0],))
    return rows[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file():
        parser.error('use an existing independent database copy, not world.db')
    with closing(sqlite3.connect(args.db)) as connection, connection:
        guid = remove_ground_rider(connection)
    print(f'removed ground placement {guid}; {RIDER} rides {MOUNT}')


if __name__ == '__main__':
    main()
