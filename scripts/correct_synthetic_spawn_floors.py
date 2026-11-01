#!/usr/bin/env python3
"""Resolve Wowhead-derived spawn floors against real anchors and recorded routes."""
import argparse
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile

POLICY = 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS'
# navigation::AGENT_CLIMB: changes within one walkable step are not floor changes.
AGENT_CLIMB = 1.6


def read_spawns(db):
    return db.execute('''SELECT guid,id1,map,position_x,position_y,position_z,Comment
        FROM content_creature WHERE Comment LIKE ? ORDER BY guid''', (POLICY + '%',)).fetchall()


def write_anchor_database(source, path, routes):
    with sqlite3.connect(path) as target:
        target.execute('CREATE TABLE map (ID INTEGER, WdtFileDataID INTEGER)')
        target.executemany('INSERT INTO map VALUES (?,?)', source.execute('SELECT ID,WdtFileDataID FROM map'))
        for table in ('tdb_creature', 'tdb_gameobject', 'content_creature'):
            target.execute(f'''CREATE TABLE {table} (guid INTEGER, map INTEGER,
                position_x REAL, position_y REAL, position_z REAL, Comment TEXT)''')
            if table != 'content_creature':
                target.executemany(f'INSERT INTO {table} VALUES (?,?,?,?,?,NULL)',
                    source.execute(f'SELECT guid,map,position_x,position_y,position_z FROM {table}'))
        if target.execute('SELECT 1 FROM tdb_gameobject WHERE guid < 0 LIMIT 1').fetchone():
            raise ValueError('negative gameobject GUIDs conflict with route anchor IDs')
        target.executemany('INSERT INTO tdb_gameobject VALUES (?,?,?,?,?,NULL)',
            ((-index-1, row['map'], row['x'], row['y'], row['z'])
             for index, row in enumerate(routes)))


def resolve_spawns(source, executable, geometry, navigation, routes):
    rows = read_spawns(source)
    with tempfile.TemporaryDirectory(prefix='spawn-floors-') as directory:
        root = Path(directory)
        write_anchor_database(source, root / 'anchors.db', routes)
        requests = [dict(guid=str(row[0]), map=row[2], x=row[3], y=row[4], ui_map_id=None) for row in rows]
        (root / 'input.json').write_text(json.dumps(requests))
        subprocess.run([str(executable), str(root / 'input.json'), str(root / 'output.json'),
                        str(geometry), str(navigation), str(root / 'anchors.db')], check=True)
        results = json.loads((root / 'output.json').read_text())
    if len(results) != len(rows):
        raise ValueError('floor resolver returned a different number of rows')
    changes = []
    gaps = []
    for row, result in zip(rows, results):
        if result.get('method') != 'authored_neighbor_anchor':
            gaps.append(dict(guid=row[0], result=result))
            continue
        if abs(result['z'] - row[5]) > AGENT_CLIMB:
            anchors = [dict(table='route', index=-anchor['guid']-1)
                       if anchor['table'] == 'tdb_gameobject' and anchor['guid'] < 0 else anchor
                       for anchor in result['anchor_rows']]
            changes.append(dict(guid=row[0], entry=row[1], map=row[2], x=row[3], y=row[4],
                                old_z=row[5], new_z=result['z'], comment=row[6], anchor_rows=anchors))
    return dict(scanned=len(rows), changes=changes, unresolved=gaps)


def apply_changes(db, changes):
    with db:
        for row in changes:
            cursor = db.execute('''UPDATE content_creature SET position_z=?
                WHERE guid=? AND id1=? AND map=? AND position_x=? AND position_y=?
                  AND position_z=? AND Comment=?''', (row['new_z'], row['guid'], row['entry'],
                row['map'], row['x'], row['y'], row['old_z'], row['comment']))
            if cursor.rowcount != 1:
                raise ValueError(f"spawn {row['guid']} changed since resolution")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('world-db', 'resolver', 'geometry', 'navigation', 'routes', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--apply', action='store_true', help='apply guarded updates to world-db')
    args = parser.parse_args()
    mode = 'rw' if args.apply else 'ro'
    with sqlite3.connect(f'file:{args.world_db.resolve()}?mode={mode}', uri=True) as db:
        if args.apply:
            report = json.loads(args.output.read_text())
            apply_changes(db, report['changes'])
        else:
            routes = json.loads(args.routes.read_text())
            report = resolve_spawns(db, args.resolver, args.geometry, args.navigation, routes)
            args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(f"scanned {report['scanned']}, changes {len(report['changes'])}, unresolved {len(report['unresolved'])}")


if __name__ == '__main__':
    main()
