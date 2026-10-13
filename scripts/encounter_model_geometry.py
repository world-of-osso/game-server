#!/usr/bin/env python3
"""Prepare source-selected encounter model geometry in independent copies.

TC a352b1fa Golden Serpent follows at both units' actual CombatReach plus its
authored range, then uses invisible display11686 on absorption; Slagmaw61463's
boundary check measures `GetDistance`, which subtracts both units' CombatReach. Import the TDB1210 model-info
rows for the source templates and that display, without activating movement,
changing templates or substituting a guessed reach. Input is the local full
TDB mysqldump (its INSERT records are single-line), never a CDN download.
"""
import argparse
import sqlite3
from contextlib import closing
from pathlib import Path

import mysql_dump

ACTORS = (135322, 135406, 61463)
INVISIBLE_DISPLAY = 11686
COLUMNS = ('DisplayID', 'BoundingRadius', 'CombatReach', 'DisplayID_Other_Gender', 'VerifiedBuild')
SCHEMA = '''CREATE TABLE IF NOT EXISTS encounter_creature_model_info (
    DisplayID INTEGER PRIMARY KEY,
    BoundingRadius REAL NOT NULL,
    CombatReach REAL NOT NULL,
    DisplayID_Other_Gender INTEGER NOT NULL,
    VerifiedBuild INTEGER NOT NULL
)'''


def select_source_displays(connection):
    models = connection.execute(
        'SELECT CreatureID,CreatureDisplayID FROM tdb_creature_template_model '
        f'WHERE CreatureID IN ({",".join("?" * len(ACTORS))})', ACTORS,
    ).fetchall()
    actors = {entry for entry, _ in models}
    if actors != set(ACTORS):
        raise ValueError(f'Missing authored encounter model rows for {set(ACTORS) - actors}')
    return {display for _, display in models} | {INVISIBLE_DISPLAY}


def read_model_info(dump, displays):
    selected = {}
    with dump.open(encoding='utf-8') as source:
        for statement in source:
            if not statement.startswith('INSERT INTO `creature_model_info`'):
                continue
            for columns, row in mysql_dump.dump_rows(statement, 'creature_model_info', dump):
                if columns is not None and tuple(columns) != COLUMNS:
                    raise ValueError(f'{dump}: unexpected model-info column order {columns}')
                if len(row) != len(COLUMNS):
                    raise ValueError(f'{dump}: incompatible model-info row with {len(row)} fields')
                if row[0] in displays:
                    selected[row[0]] = row
    missing = displays - selected.keys()
    if missing:
        raise ValueError(f'{dump}: missing authored model-info displays {sorted(missing)}')
    return [selected[display] for display in sorted(displays)]


def import_geometry(connection, dump):
    displays = select_source_displays(connection)
    rows = read_model_info(dump, displays)
    connection.execute(SCHEMA)
    connection.executemany(
        'INSERT OR REPLACE INTO encounter_creature_model_info VALUES (?,?,?,?,?)', rows,
    )
    return [row[0] for row in rows]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    parser.add_argument('--source-dump', type=Path, required=True)
    args = parser.parse_args()
    canonical = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
    if args.db.name == 'world.db' or args.db.resolve() == canonical.resolve():
        parser.error('use an independent database copy, not canonical world.db')
    if args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('copy must be an existing independent regular file')
    if not args.source_dump.is_file():
        parser.error(f'missing local source dump: {args.source_dump}')
    with closing(sqlite3.connect(args.db)) as connection:
        with connection:
            displays = import_geometry(connection, args.source_dump)
    print(f'Imported source model geometry for {displays}; no runtime behavior activated')


if __name__ == '__main__':
    main()
