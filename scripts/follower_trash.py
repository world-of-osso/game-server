#!/usr/bin/env python3
"""Prepare direct individual Wowhead trash-marker packets; never publish canonical data."""
import argparse
from contextlib import closing
import hashlib
import itertools
import json
import math
from pathlib import Path
import sqlite3

from follower_placements import (PLACEMENT_POLICY, TRASH_HOSTILITY_POLICY, TABLE_KEYS,
    backup, make_sql, open_readonly, prepare_templates, read_table_rows)
from follower_placement_sources import (SINGLE_FRAME_POLICY, digest, read_cached_npc, read_db2, read_json_payload,
    read_surface_polygons, sample_surfaces, select_assignment, transform_marker)


SURFACE_CELL_YARDS = 32


def index_surface_polygons(polygons):
    cells = {}
    for polygon in polygons:
        xs = [vertex[0] for vertex in polygon]
        zs = [vertex[2] for vertex in polygon]
        cell_x = range(math.floor(min(xs) / SURFACE_CELL_YARDS), math.floor(max(xs) / SURFACE_CELL_YARDS) + 1)
        cell_z = range(math.floor(min(zs) / SURFACE_CELL_YARDS), math.floor(max(zs) / SURFACE_CELL_YARDS) + 1)
        for key in itertools.product(cell_x, cell_z):
            cells.setdefault(key, []).append(polygon)
    return cells


def read_marker_reports(page):
    raw = Path(page['cache_body']).read_text()
    source = json.loads(raw) if raw.startswith('"') else raw
    mapper = read_json_payload(source, r'g_mapperData\s*=\s*')
    if mapper == []:
        return []
    reports = []
    for zone, levels in mapper.items():
        indexed = enumerate(levels) if isinstance(levels, list) else levels.items()
        reports.extend(dict(zone=int(zone), floor=int(floor), reported_count=data['count'],
                            explicit_marker_count=len(data['coords'])) for floor, data in indexed)
    return reports


def project_marker(map_id, page, index, tables, polygons_by_cell, assignments_by_frame, projection_policy=None):
    marker = page['markers'][index]
    point = dict(entry=page['entry'], marker_index=index, marker=marker,
                 cache_sha256=page['cache_sha256'], source_url=page['url'])
    try:
        frame = (map_id, marker['zone'], marker['floor'], projection_policy)
        if frame not in assignments_by_frame:
            assignments_by_frame[frame] = select_assignment(tables, map_id, marker, projection_policy)
        assignments = assignments_by_frame[frame]
    except ValueError as error:
        return dict(point, status='FOLLOWER_TRASH_MARKER_PROJECTION_UNRESOLVED', reason=str(error))
    if projection_policy == SINGLE_FRAME_POLICY:
        point.update(projection_policy=projection_policy, observed_wowhead_floor=marker['floor'], db2_group_floor_identity=None)
    x, y = transform_marker(marker, assignments)
    candidates = polygons_by_cell.get((math.floor(x / SURFACE_CELL_YARDS), math.floor(-y / SURFACE_CELL_YARDS)), [])
    heights = sample_surfaces(candidates, x, y, assignments)
    point.update(assignments=assignments, wow_xy=[x, y], all_surface_heights=heights)
    if len(heights) != 1:
        return dict(point, status='FOLLOWER_TRASH_FLOOR_UNRESOLVED')
    return dict(point, status='UNIQUE_BAKED_SURFACE', wow_xyz=[x, y, heights[0]])


def read_trash_markers(manifest, tables, cache, polygons):
    pages, points, gaps = [], [], []
    polygons_by_cell = index_surface_polygons(polygons)
    assignments_by_frame = {}
    for record in manifest['entries']:
        page = read_cached_npc(cache, record['entry'])
        if page['cache_sha256'] != record['cache_sha256']:
            raise ValueError(f"pinned own NPC page changed: {record['entry']}")
        pages.append(dict(page, marker_count=len(page['markers']), reported_markers=read_marker_reports(page)))
        if not page['markers']:
            gaps.append(dict(entry=record['entry'], status='FOLLOWER_TRASH_INDIVIDUAL_MARKERS_MISSING'))
        for index in range(len(page['markers'])):
            point = project_marker(manifest['map_id'], page, index, tables, polygons_by_cell, assignments_by_frame, record.get('projection_policy'))
            (points if 'wow_xyz' in point else gaps).append(point)
    return pages, points, gaps


def write_trash_spawns(target, manifest, placements):
    lower = manifest['guid_base'] + manifest['map_id'] * 1000
    if manifest['first_trash_ordinal'] != 100 or len(placements) > 900:
        raise ValueError('trash markers must fit the explicitly reserved map ordinals100–999')
    expected = {lower + 100 + index: point for index, point in enumerate(placements)}
    occupied = target.execute('SELECT guid FROM content_creature WHERE guid BETWEEN ? AND ?', (lower + 100, lower + 999)).fetchall()
    if any(row[0] not in expected for row in occupied):
        raise ValueError('reserved trash namespace overlaps unrelated content')
    for guid, point in expected.items():
        x, y, z = point['wow_xyz']
        comment = f"{PLACEMENT_POLICY}; {point['source_url']}; floor={point['marker']['floor']}; percent={point['marker']['percent']}; independent trash"
        before = target.execute('SELECT id1,map,position_x,position_y,position_z,Comment FROM content_creature WHERE guid=?', (guid,)).fetchone()
        if before is not None and tuple(before) != (point['entry'], manifest['map_id'], x, y, z, comment):
            raise ValueError(f'reserved trash GUID collision at{guid}')
        target.execute('INSERT OR IGNORE INTO content_creature(guid,id1,map,position_x,position_y,position_z,spawnDifficulties,Comment) VALUES(?,?,?,?,?,?,?,?)',
                       (guid, point['entry'], manifest['map_id'], x, y, z, '1', comment))
        point.update(guid=guid, orientation_policy='unsourced schema-default facing; independent stationary project placement')


def fingerprint_group_tables(db):
    result = {}
    for row in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%spawn_group%' ORDER BY name"):
        table = row[0]
        columns = [column[1] for column in db.execute(f'PRAGMA table_info({table})')]
        rows = [tuple(r) for r in db.execute(f"SELECT * FROM {table} ORDER BY {','.join(columns)}")]
        result[table] = dict(count=len(rows), sha256=hashlib.sha256(json.dumps(rows, separators=(',', ':')).encode()).hexdigest())
    return result


def validate_map_tuning(source, tables, map_id):
    native = [dict(row) for row in source.execute('SELECT * FROM map_difficulty WHERE MapID=? AND DifficultyID=1', (map_id,))]
    authenticated = [r for r in tables['MapDifficulty'] if int(r['MapID']) == map_id and r['DifficultyID'] == '1']
    if len(native) != 1 or [{key: str(value) for key, value in row.items()} for row in native] != authenticated:
        raise ValueError('native Normal map difficulty differs from authenticated DB2')


def prepare(args):
    if args.output.exists():
        raise ValueError('output must be new; never overwrite proof')
    args.output.mkdir(parents=True)
    manifest = json.loads(args.manifest.read_text())
    if manifest['placement_policy'] != PLACEMENT_POLICY or manifest['faction_policy'] != TRASH_HOSTILITY_POLICY:
        raise ValueError('trash manifest differs from approved source policies')
    updates = manifest.get('guarded_faction_updates', [])
    if updates and (manifest['map_id'] != 2811 or updates != [232106]):
        raise ValueError('only the explicit Magisters232106 faction correction is authorized')
    if digest(args.surfaces) != manifest['surface_sha256']:
        raise ValueError('covering surface witness changed')
    nav_files = {path.name: digest(path) for path in args.nav.glob('*.nav')}
    if nav_files != manifest['nav_files_sha256']:
        raise ValueError('published nav changed since the covering witness')
    if any(record['map_id'] != manifest['map_id'] for record in manifest['entries']):
        raise ValueError('trash template map differs from placement map')
    if any(record.get('projection_policy') not in (None, SINGLE_FRAME_POLICY) for record in manifest['entries']):
        raise ValueError('trash marker projection policy is not approved')
    tables, authentication = read_db2(args.db2, ('MapDifficulty', 'MapChallengeMode'))
    polygons = read_surface_polygons(args.surfaces)
    pages, placements, gaps = read_trash_markers(manifest, tables, args.cache, polygons)
    baseline_path, prepared_path, replay_path = [args.output / name for name in ('baseline.sqlite', 'prepared.sqlite', 'replayed.sqlite')]
    with closing(open_readonly(args.source_db)) as source:
        backup(source, baseline_path)
    receipt = dict(map_id=manifest['map_id'], policy=PLACEMENT_POLICY, faction_policy=TRASH_HOSTILITY_POLICY,
                   producer_sha256=digest(__file__), template_producer_sha256=digest(Path(__file__).with_name('follower_placements.py')),
                   source_reader_sha256=digest(Path(__file__).with_name('follower_placement_sources.py')),
                   manifest_sha256=digest(args.manifest), db2_authentication=authentication,
                   surface_file=str(args.surfaces), surface_sha256=digest(args.surfaces), surface_proof=manifest['surface_proof'],
                   source_database=str(args.source_db.resolve()), baseline_sha256=digest(baseline_path),
                   source_pages=pages, placements=placements, source_gaps=gaps,
                   marker_count=sum(p['marker_count'] for p in pages), pack_groups_created=False,
                   grouping_policy='Independent own-page markers; no inferred pack membership; native social aggro remains authoritative',
                   entrance=None, entrance_gaps=[f"FOLLOWER_ENTRANCE_UNSOURCED_{manifest['map_id']}"],
                   route_queries=[], nav_directory=str(args.nav.resolve()), nav_files_sha256=nav_files,
                   canonical_modified=False)
    if not placements:
        receipt['status'] = 'BLOCKED_NO_QUALIFIED_TRASH_MARKERS'
        (args.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        return 2
    with closing(open_readonly(baseline_path)) as baseline:
        baseline.row_factory = sqlite3.Row
        validate_map_tuning(baseline, tables, manifest['map_id'])
        if manifest['entrance_id'] is not None:
            entrance = baseline.execute('SELECT * FROM tdb_world_safe_locs WHERE MapID=? AND ID=?', (manifest['map_id'], manifest['entrance_id'])).fetchone()
            if entrance is None:
                raise ValueError('declared trash-map entrance is not sourced')
            receipt['entrance'], receipt['entrance_gaps'] = dict(entrance), []
        before_groups = fingerprint_group_tables(baseline)
        for point in placements:
            if baseline.execute('SELECT 1 FROM tdb_creature_template WHERE entry=?', (point['entry'],)).fetchone() is None:
                raise ValueError('qualified trash marker lacks its own native template')
        backup(baseline, prepared_path)
        records = [r for r in manifest['entries'] if r['entry'] in {p['entry'] for p in placements}]
        with closing(sqlite3.connect(prepared_path)) as prepared, prepared:
            receipt['donors'], receipt['source_difficulties'] = prepare_templates(prepared, records, args.cache, tables)
            write_trash_spawns(prepared, manifest, placements)
            sql, receipt['changed_keys'] = make_sql(baseline, prepared, set(updates))
            receipt['guarded_faction_updates'] = []
            for entry in updates:
                before = dict(baseline.execute('SELECT * FROM content_creature_template WHERE entry=?', (entry,)).fetchone())
                cursor = prepared.execute('SELECT * FROM content_creature_template WHERE entry=?', (entry,))
                after = dict(zip([c[0] for c in cursor.description], cursor.fetchone()))
                receipt['guarded_faction_updates'].append(dict(entry=entry, before=before, after=after,
                    policy=TRASH_HOSTILITY_POLICY, guard='faction0→16 only; every other column exact; exact already-applied16 accepted for idempotent replay'))
            prepared_groups = fingerprint_group_tables(prepared)
            if before_groups != prepared_groups:
                raise ValueError('independent-marker packet changed spawn-group rows')
        backup(baseline, replay_path)
        with closing(sqlite3.connect(replay_path)) as replay, closing(open_readonly(prepared_path)) as prepared:
            expected = {table: read_table_rows(prepared, table) for table in TABLE_KEYS}
            for _ in range(3):
                replay.executescript(sql)
                if any(read_table_rows(replay, table) != rows for table, rows in expected.items()):
                    raise ValueError('trash SQL replay differs from prepared copy')
            replay_groups = fingerprint_group_tables(replay)
            if replay_groups != before_groups:
                raise ValueError('SQL replay changed spawn-group rows')
        receipt['preserved_group_tables'] = dict(baseline=before_groups, prepared=prepared_groups, replayed=replay_groups)
        (args.output / 'delta.sql').write_text(sql)
        receipt.update(status='PREPARED_WITH_SOURCE_AND_ENTRANCE_GAPS', sql_replay_count=3,
                       sql_sha256=digest(args.output / 'delta.sql'), replayed_sha256=digest(replay_path),
                       guid_range=[manifest['guid_base'] + manifest['map_id'] * 1000 + 100,
                                   manifest['guid_base'] + manifest['map_id'] * 1000 + 999])
    (args.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(receipt['status'], len(placements), 'independent spawns')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source-db', 'manifest', 'db2', 'cache', 'surfaces', 'nav', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    return prepare(parser.parse_args())


if __name__ == '__main__':
    raise SystemExit(main())
