#!/usr/bin/env python3
"""Derive interior admissions from sourced portals or approved initial respawn POIs, COPY only."""
import argparse
from contextlib import closing
import csv
import json
import math
from pathlib import Path
import sqlite3

from follower_placements import backup, open_readonly
from follower_placement_sources import BUILD, digest
from import_instance_population import BASE_PHASE

POLICY = 'FOLLOWER_ENTRANCE_FROM_EXIT_PORTAL'
RESPAWN_POLICY = 'FOLLOWER_ENTRANCE_FROM_INITIAL_RESPAWN_POI'
INITIAL_RESPAWN_POIS = {2649: 8073, 2669: 8072}
SAFE_LOCATION_BASE = 81_000_000
NORMAL = "(','||replace(spawnDifficulties,' ','')||',') LIKE '%,1,%'"
COLUMNS = ('ID', 'MapID', 'LocX', 'LocY', 'LocZ', 'Facing', 'TransportSpawnId', 'Comment')


def read_portal(source, map_id):
    rows = source.execute(
        f"SELECT g.*,t.name FROM tdb_gameobject g JOIN tdb_gameobject_template t ON t.entry=g.id "
        f"WHERE g.map=? AND t.name='Instance Portal' AND {NORMAL} AND {BASE_PHASE}", (map_id,)
    ).fetchall()
    if len(rows) != 1:
        raise ValueError(f'map{map_id} requires one sourced Normal/base-phase Instance Portal, found{len(rows)}')
    return dict(rows[0])


def read_facing_anchor(source, portal):
    if portal['map'] == 2662:
        row = source.execute("SELECT c.guid,c.id,c.position_x,c.position_y,c.position_z,t.name FROM tdb_creature c "
                             "JOIN tdb_creature_template t ON t.entry=c.id WHERE c.guid=10001505 AND c.id=211151 "
                             "AND c.map=2662 AND t.name='Path Helper'").fetchone()
        policy = 'FOLLOWER_ENTRANCE_FACING_SOURCED_PATH_HELPER'
    else:
        row = source.execute(
            f"SELECT c.guid,c.id,c.position_x,c.position_y,c.position_z,t.name FROM tdb_creature c "
            f"JOIN tdb_creature_template t ON t.entry=c.id WHERE c.map=? AND t.faction=16 "
            f"AND {NORMAL} AND {BASE_PHASE} ORDER BY "
            "(c.position_x-?)*(c.position_x-?)+(c.position_y-?)*(c.position_y-?),c.guid LIMIT 1",
            (portal['map'], portal['position_x'], portal['position_x'], portal['position_y'], portal['position_y'])
        ).fetchone()
        policy = 'FOLLOWER_ENTRANCE_FACING_FIRST_SOURCED_TRASH'
    if row is None:
        raise ValueError(f"map{portal['map']} has no approved sourced facing anchor")
    anchor = dict(row)
    dx, dy = anchor['position_x'] - portal['position_x'], anchor['position_y'] - portal['position_y']
    if dx == 0 and dy == 0:
        raise ValueError('facing anchor coincides horizontally with portal')
    # WorldSafeLoc Facing is degrees, unlike native gameobject orientation radians.
    facing = math.degrees(math.atan2(dy, dx)) % 360
    return facing, dict(policy=policy, source_spawn=anchor, method='bearing to nearest sourced trash; Dawnbreaker uses approved Path Helper route approximation')


def derive_entrances(source, maps):
    rows, evidence = [], []
    for map_id in sorted(set(maps)):
        safe_id = SAFE_LOCATION_BASE + map_id
        native = source.execute("SELECT * FROM tdb_world_safe_locs WHERE MapID=? AND ID!=? "
                                "AND Comment LIKE '%Entrance%' ORDER BY ID", (map_id, safe_id)).fetchall()
        if native:
            evidence.append(dict(map_id=map_id, status='AUTHORED_ENTRANCE_RETAINED', rows=[dict(r) for r in native]))
            continue
        portal = read_portal(source, map_id)
        facing, facing_evidence = read_facing_anchor(source, portal)
        comment = f"{POLICY}; source TDB gameobject guid={portal['guid']} entry={portal['id']}; {facing_evidence['policy']}; Entrance"
        row = (safe_id, map_id, portal['position_x'], portal['position_y'], portal['position_z'], facing, None, comment)
        existing = source.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=?', (safe_id,)).fetchone()
        if existing is not None and tuple(existing) != row:
            raise ValueError(f'reserved safe-location ID{safe_id} overlaps existing content')
        rows.append(row)
        evidence.append(dict(map_id=map_id, status='DERIVED_FROM_NATIVE_PORTAL', safe_location_id=safe_id,
                             portal=portal, facing=facing_evidence, world_safe_location=dict(zip(COLUMNS, row))))
    return rows, evidence


def read_respawn_pois(directory):
    receipt = json.loads((directory / 'receipt.json').read_text())
    identity = receipt['identity']
    if identity['build'] != BUILD or identity['extraction_source'] != 'authenticated-local-CASC-content-root':
        raise ValueError('initial respawn POIs require authenticated local CASC build70448')
    path = directory / 'AreaPOI.csv'
    metadata = receipt['tables']['AreaPOI']
    if not metadata['content_authenticated'] or metadata['unreadable_records'] or digest(path) != metadata['csv_sha256']:
        raise ValueError('AreaPOI authentication/hash mismatch')
    with path.open() as stream:
        rows = list(csv.DictReader(stream))
    return rows, {'AreaPOI': metadata}, identity


def select_initial_respawn_poi(pois, map_id):
    if map_id not in INITIAL_RESPAWN_POIS:
        raise ValueError('initial respawn admission policy is approved only for2649/2669')
    candidates = [p for p in pois if int(p['ContinentID']) == map_id and 'respawn' in p['Name_lang'].lower()
                  and int(p['PlayerConditionID']) == 0 and int(p['WorldStateID']) == 0]
    if len(candidates) != 1 or int(candidates[0]['ID']) != INITIAL_RESPAWN_POIS[map_id]:
        raise ValueError(f'map{map_id} requires its unique approved unconditional initial respawn POI')
    if not all(math.isfinite(float(candidates[0][f'Pos_{i}'])) for i in range(3)):
        raise ValueError('initial respawn POI has nonfinite coordinates')
    return candidates[0]


def derive_initial_respawn_entrances(source, maps, pois):
    rows, evidence = [], []
    for map_id in sorted(set(maps)):
        poi = select_initial_respawn_poi(pois, map_id)
        safe_id = SAFE_LOCATION_BASE + map_id
        native = source.execute("SELECT ID FROM tdb_world_safe_locs WHERE MapID=? AND ID!=? AND Comment LIKE '%Entrance%'",
                                (map_id, safe_id)).fetchall()
        portal = source.execute(f"SELECT g.guid FROM tdb_gameobject g JOIN tdb_gameobject_template t ON t.entry=g.id "
                                f"WHERE g.map=? AND t.name='Instance Portal' AND {NORMAL} AND {BASE_PHASE}", (map_id,)).fetchall()
        if native or portal:
            raise ValueError(f'map{map_id} has a higher-priority native entrance/portal source')
        location = dict(map=map_id, **{f'position_{axis}': float(poi[f'Pos_{i}']) for i, axis in enumerate(('x', 'y', 'z'))})
        facing, facing_evidence = read_facing_anchor(source, location)
        gap = f'FOLLOWER_ENTRANCE_NATIVE_PORTAL_UNSOURCED_{map_id}'
        comment = f"{RESPAWN_POLICY}; source AreaPOI={poi['ID']}; {facing_evidence['policy']}; {gap}; Entrance (project admission, not native portal)"
        row = (safe_id, map_id, location['position_x'], location['position_y'], location['position_z'], facing, None, comment)
        existing = source.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=?', (safe_id,)).fetchone()
        if existing is not None and tuple(existing) != row:
            raise ValueError(f'reserved safe-location ID{safe_id} overlaps existing content')
        rows.append(row)
        evidence.append(dict(map_id=map_id, status='PROJECT_ADMISSION_FROM_INITIAL_RESPAWN_POI',
                             safe_location_id=safe_id, source_poi=poi, facing=facing_evidence,
                             native_portal_gap=gap, world_safe_location=dict(zip(COLUMNS, row))))
    return rows, evidence


def calibrate_respawn_against_portals(source, pois):
    samples = []
    for map_id in (2651, 2652, 2660, 2661, 2662, 2773):
        safe = source.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=? AND MapID=?',
                              (SAFE_LOCATION_BASE + map_id, map_id)).fetchone()
        candidates = [p for p in pois if int(p['ContinentID']) == map_id and 'respawn' in p['Name_lang'].lower()]
        if safe is None or not candidates:
            raise ValueError(f'map{map_id} lacks the published portal/respawn calibration control')
        xyz = [safe['LocX'], safe['LocY'], safe['LocZ']]
        measured = [(math.dist(xyz, [float(p[f'Pos_{i}']) for i in range(3)]), p) for p in candidates]
        distance, nearest = min(measured, key=lambda item: item[0])
        samples.append(dict(map_id=map_id, portal_safe_location=dict(safe), source_poi=nearest, distance_yards=distance))
    return [dict(finding='FOLLOWER_ENTRANCE_RESPAWN_POI_PORTAL_DISTANCE',
                 distance_min_yards=min(s['distance_yards'] for s in samples),
                 distance_max_yards=max(s['distance_yards'] for s in samples), samples=samples,
                 interpretation='Known nearest respawn POIs differ from portal entries by approximately13-51yd; some controls are conditional and their conditions are retained. New admissions require unconditional initial POIs. These are project admission positions, not captured native entrance points.')]


def make_sql(source, rows):
    quote = lambda value: source.execute('SELECT quote(?)', (value,)).fetchone()[0]
    statements = ['BEGIN IMMEDIATE;', 'CREATE TEMP TABLE follower_entrance_guard(ok INTEGER CHECK(ok=1));']
    for row in rows:
        match = ' AND '.join(f'{column} IS {quote(value)}' for column, value in zip(COLUMNS, row))
        statements.append(f'INSERT INTO follower_entrance_guard SELECT NOT EXISTS(SELECT 1 FROM tdb_world_safe_locs WHERE ID={row[0]} AND NOT({match}));')
        values = ','.join(quote(value) for value in row)
        statements.append(f"INSERT INTO tdb_world_safe_locs({','.join(COLUMNS)}) VALUES({values}) ON CONFLICT(ID) DO NOTHING;")
    statements.extend(['DROP TABLE follower_entrance_guard;', 'COMMIT;'])
    return '\n'.join(statements) + '\n'


def prepare(args):
    if args.output.exists():
        raise ValueError('output directory must be new; never overwrite a source or proof')
    args.output.mkdir(parents=True)
    baseline, replay_path = args.output / 'baseline.sqlite', args.output / 'replayed.sqlite'
    with closing(open_readonly(args.source_db)) as source:
        backup(source, baseline)
    with closing(open_readonly(baseline)) as source:
        source.row_factory = sqlite3.Row
        policy, authentication, identity, findings = POLICY, {}, None, []
        if args.initial_respawn_poi_db2 is not None:
            pois, authentication, identity = read_respawn_pois(args.initial_respawn_poi_db2)
            rows, evidence = derive_initial_respawn_entrances(source, args.maps, pois)
            policy = RESPAWN_POLICY
            findings = calibrate_respawn_against_portals(source, pois)
        else:
            rows, evidence = derive_entrances(source, args.maps)
        sql = make_sql(source, rows)
        expected = {r['ID']: tuple(r) for r in source.execute('SELECT * FROM tdb_world_safe_locs')}
        expected.update({row[0]: row for row in rows})
        backup(source, replay_path)
    with closing(sqlite3.connect(replay_path)) as replay:
        for _ in range(3):
            replay.executescript(sql)
            actual = {r[0]: r for r in replay.execute('SELECT * FROM tdb_world_safe_locs')}
            if actual != expected:
                raise ValueError('safe-location SQL replay changed unexpected rows or failed to reproduce derived entrances')
    (args.output / 'delta.sql').write_text(sql)
    receipt = dict(policy=policy, producer_sha256=digest(__file__), source_database=str(args.source_db.resolve()),
                   baseline_sha256=digest(baseline), replayed_sha256=digest(replay_path), sql_sha256=digest(args.output / 'delta.sql'),
                   maps=evidence, changed_keys=[r[0] for r in rows], changed_tables=['tdb_world_safe_locs'] if rows else [],
                   reserved_id_rule='81000000+mapID; reject an existing nonidentical row', sql_replay_count=3,
                   exact_portal_xyz=policy == POLICY, facing_is_project_policy=True, canonical_modified=False,
                   db2_authentication=authentication, db2_identity=identity, findings=findings,
                   native_portal_gaps=[e['native_portal_gap'] for e in evidence if 'native_portal_gap' in e],
                   navigation_scope='Source coordinates only; navigation queries owned separately, no all-route or dungeon-clear proof')
    (args.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(f'Prepared{len(rows)} {policy} entrance rows on a fresh canonical COPY; replay3x exact')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-db', type=Path, required=True)
    parser.add_argument('--maps', type=int, nargs='+', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--initial-respawn-poi-db2', type=Path, help='explicit project-admission policy for2649/2669 only')
    prepare(parser.parse_args())


if __name__ == '__main__':
    main()
