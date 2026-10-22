#!/usr/bin/env python3
"""Prepare one sourced map packet; publish nothing and never write canonical world.db."""
import argparse
from contextlib import closing
import csv
import json
from pathlib import Path
import sqlite3
import subprocess

import retail_content
from follower_placement_sources import (digest, read_cached_npc, read_db2,
    read_surface_polygons, sample_surfaces, select_assignment, transform_marker)

PLACEMENT_POLICY = 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS'
FACTION_POLICY = 'FOLLOWER_PLACEMENT_FACTION_FROM_REACT_AND_DONOR'
TABLE_KEYS = {'content_creature_template': ('entry',),
              'content_creature_template_model': ('CreatureID', 'Idx'),
              'content_creature': ('guid',),
              'raid_encounter_creature': ('map_id', 'creature_entry')}


def open_readonly(path):
    return sqlite3.connect(Path(path).resolve().as_uri() + '?mode=ro', uri=True)


def backup(source, destination):
    with closing(sqlite3.connect(destination)) as output:
        source.backup(output)


def query_route(executable, nav, start, end):
    argv = [str(executable), str(nav), *map(str, start), *map(str, end)]
    result = subprocess.run(argv, capture_output=True, text=True, check=False, timeout=120)
    corners = [list(map(float, line.split(','))) for line in result.stdout.splitlines()] if result.returncode == 0 else []
    return dict(argv=argv, exit_code=result.returncode, corners=corners, stderr=result.stderr)


def derive_placement(record, manifest, tables, cache, polygons, route, nav, entrance):
    page = read_cached_npc(cache, record['entry'])
    if page['cache_sha256'] != record['cache_sha256'] or len(page['markers']) != 1:
        raise ValueError(f"NPC{record['entry']} pinned marker evidence changed")
    marker = page['markers'][0]
    assignments = select_assignment(tables, manifest['map_id'], marker)
    x, y = transform_marker(marker, assignments)
    heights = sample_surfaces(polygons, x, y, assignments)
    start = [entrance['LocX'], entrance['LocZ'], -entrance['LocY']]
    queries = [dict(height=z, **query_route(route, nav, start, [x, z, -y])) for z in heights]
    reachable = {tuple(q['corners'][-1]): q for q in queries if q['exit_code'] == 0 and q['corners']}
    placement = dict(entry=record['entry'], page=page, marker=marker,
                     assignments=assignments, wow_xy=[x, y], surface_queries=queries,
                     status='UNIQUE_REACHABLE_SURFACE' if len(reachable) == 1 else 'MISSING_OR_AMBIGUOUS_SURFACE')
    if 'source_floor_surface_height' in record:
        selected = [q for q in queries if abs(q['height'] - record['source_floor_surface_height']) < 0.000001]
        if len(selected) != 1 or marker['floor'] != record['source_floor']:
            raise ValueError('approved source-floor surface no longer matches baked geometry')
        placement['wow_xyz'] = [x, y, selected[0]['height']]
        placement['source_floor_evidence'] = record['source_floor_evidence']
        if selected[0]['exit_code'] != 0:
            placement['status'] = 'SOURCE_FLOOR_ROUTE_GAP'
            placement['navigation_gap'] = record['navigation_gap']
        else:
            placement['status'] = 'UNIQUE_REACHABLE_SURFACE'
    elif len(reachable) == 1:
        placement['wow_xyz'] = [x, y, next(iter(reachable.values()))['height']]
    return placement


def read_binding_tables(directory):
    receipt = json.loads((directory / 'receipt.json').read_text())
    tables, proof = {}, {}
    for name in ('DungeonEncounter', 'JournalEncounter', 'JournalEncounterCreature'):
        path = directory / f'{name}.csv'
        metadata = receipt['tables'][name]
        if not metadata['content_authenticated'] or metadata['unreadable_records'] or digest(path) != metadata['csv_sha256']:
            raise ValueError(f'{name} authentication/hash mismatch')
        with path.open() as stream:
            tables[name] = list(csv.DictReader(stream))
        proof[name] = metadata
    return tables, proof


def derive_binding(source, inventory, tables, map_id, entry):
    encounter = next(e for e in inventory['encounters'] if e['MapID'] == map_id and entry in e['enabled_entries'])
    dungeon = next(r for r in tables['DungeonEncounter'] if int(r['ID']) == encounter['encounter']['ID'])
    journal = next(r for r in tables['JournalEncounter'] if int(r['ID']) in encounter['journal_ids'])
    displays = {r[0] for r in source.execute('SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?', (entry,))}
    actors = [r for r in tables['JournalEncounterCreature'] if r['JournalEncounterID'] == journal['ID'] and int(r['CreatureDisplayInfoID']) in displays]
    if len(actors) != 1 or int(dungeon['MapID']) != map_id or journal['DungeonEncounterID'] != dungeon['ID']:
        raise ValueError(f'NPC{entry} lacks an unambiguous authenticated primary identity')
    actor = actors[0]
    return (map_id, entry, int(dungeon['ID']), int(dungeon['Bit']), int(journal['ID']), int(actor['ID']), int(actor['CreatureDisplayInfoID']))


def prepare_templates(target, records, cache):
    entries = ','.join(str(r['entry']) for r in records)
    target.execute(f'CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry IN({entries}) AND DifficultyID=1')
    count = target.execute('SELECT count(*) FROM temp.tdb_difficulty').fetchone()[0]
    if count != len(records):
        raise ValueError('every first-map actor requires its own sourced Normal1 difficulty')
    target.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    retail_content.insert_tdb_templates(target, f'SELECT Entry FROM temp.tdb_difficulty')
    donors = []
    for record in records:
        actor = read_cached_npc(cache, record['entry'])
        donor = read_cached_npc(cache, record['faction_donor'])
        faction = target.execute('SELECT faction FROM tdb_creature_template WHERE entry=?', (record['faction_donor'],)).fetchone()[0]
        if actor['entity']['react'] != [-1, -1] or donor['entity']['react'] != actor['entity']['react'] or faction <= 0:
            raise ValueError('faction donor does not have the sourced matching hostile A/H reaction')
        target.execute('UPDATE content_creature_template SET faction=? WHERE entry=? AND faction=0', (faction, record['entry']))
        donors.append(dict(entry=record['entry'], donor_entry=record['faction_donor'], donor_page=donor,
                           faction=faction, policy=FACTION_POLICY, reason=record['donor_reason']))
    target.execute(f'INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID IN({entries})')
    return donors


def write_placements(target, manifest, placements, bindings):
    for record, placement, binding in zip(manifest['entries'], placements, bindings):
        ordinal = record['ordinal']
        if not 0 <= ordinal < 1000:
            raise ValueError('placement ordinal exceeds reserved map namespace')
        guid = manifest['guid_base'] + manifest['map_id'] * 1000 + ordinal
        x, y, z = placement['wow_xyz']
        comment = f"{PLACEMENT_POLICY}; {placement['page']['url']}; floor={placement['marker']['floor']}; percent={placement['marker']['percent']}"
        existing = target.execute('SELECT id1,map,Comment FROM content_creature WHERE guid=?', (guid,)).fetchone()
        if existing is not None and tuple(existing) != (record['entry'], manifest['map_id'], comment):
            raise ValueError(f'reserved placement GUID collision at{guid}')
        target.execute('INSERT OR IGNORE INTO content_creature(guid,id1,map,position_x,position_y,position_z,spawnDifficulties,Comment) VALUES(?,?,?,?,?,?,?,?)',
                       (guid, record['entry'], manifest['map_id'], x, y, z, '1', comment))
        existing_binding = target.execute('SELECT * FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?', binding[:2]).fetchone()
        if existing_binding is not None and tuple(existing_binding) != binding:
            raise ValueError(f'primary binding conflicts for{binding[:2]}')
        target.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', binding)
        placement['guid'] = guid
        placement['binding'] = binding
        placement['orientation_policy'] = 'unsourced schema-default orientation; stationary project placement, not Retail facing'


def read_table_rows(db, table):
    keys = TABLE_KEYS[table]
    return [tuple(row) for row in db.execute(f"SELECT * FROM {table} ORDER BY {','.join(keys)}")]


def calibrate(source, entries, tables, cache):
    controls = []
    for entry in entries:
        page = read_cached_npc(cache, entry)
        native = source.execute('SELECT guid,map,position_x,position_y,position_z FROM tdb_creature WHERE id=?', (entry,)).fetchall()
        if len(native) != 1 or len(page['markers']) != 1:
            raise ValueError('calibration requires one authored spawn and one observed marker')
        row = native[0]
        assignments = select_assignment(tables, row['map'], page['markers'][0])
        x, y = transform_marker(page['markers'][0], assignments)
        controls.append(dict(entry=entry, page=page, native_spawn=dict(row), assignments=assignments,
                             transformed_xy=[x, y], residual_xy=[x-row['position_x'], y-row['position_y']]))
    return controls


def make_sql(baseline, prepared):
    statements = ['BEGIN IMMEDIATE;', 'CREATE TEMP TABLE follower_placement_guard(ok INTEGER CHECK(ok=1));']
    changed = {}
    for table, keys in TABLE_KEYS.items():
        columns = [r[1] for r in prepared.execute(f'PRAGMA table_info({table})')]
        indexes = [columns.index(k) for k in keys]
        original = {tuple(row[i] for i in indexes): row for row in read_table_rows(baseline, table)}
        inserted = []
        for row in read_table_rows(prepared, table):
            key = tuple(row[i] for i in indexes)
            if key in original:
                if row != original[key]:
                    raise ValueError(f'placement packet must not replace existing {table}{key}')
                continue
            quote = lambda v: prepared.execute('SELECT quote(?)', (v,)).fetchone()[0]
            where = ' AND '.join(f'{k}={quote(row[columns.index(k)])}' for k in keys)
            # Scripts may be attached later by the separate combat owner.
            equal = ' AND '.join(f'{name} IS {quote(value)}' for name, value in zip(columns, row) if name not in ('AIName', 'ScriptName'))
            statements.append(f'INSERT INTO follower_placement_guard SELECT NOT EXISTS(SELECT 1 FROM {table} WHERE {where} AND NOT({equal}));')
            statements.append(f"INSERT INTO {table}({','.join(columns)}) VALUES({','.join(quote(v) for v in row)}) ON CONFLICT({','.join(keys)}) DO NOTHING;")
            inserted.append(key)
        changed[table] = inserted
    statements.extend(['DROP TABLE follower_placement_guard;', 'COMMIT;'])
    return '\n'.join(statements) + '\n', changed


def prepare(args):
    if args.output.exists():
        raise ValueError('output directory must be new; never overwrite proof or inputs')
    args.output.mkdir(parents=True)
    manifest = json.loads(args.manifest.read_text())
    if manifest['placement_policy'] != PLACEMENT_POLICY or manifest['faction_policy'] != FACTION_POLICY:
        raise ValueError('manifest policies differ from approved placement policy')
    tables, auth = read_db2(args.db2)
    binding_tables, binding_auth = read_binding_tables(args.db2)
    inventory = json.loads(args.inventory.read_text())
    polygons = read_surface_polygons(args.surfaces)
    baseline_path, prepared_path, replay_path = [args.output / name for name in ('baseline.sqlite', 'prepared.sqlite', 'replayed.sqlite')]
    with closing(open_readonly(args.source_db)) as source:
        backup(source, baseline_path)
    with closing(open_readonly(baseline_path)) as baseline:
        baseline.row_factory = sqlite3.Row
        entrance = dict(baseline.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=? AND MapID=?', (manifest['entrance_id'], manifest['map_id'])).fetchone())
        placements = [derive_placement(r, manifest, tables, args.cache, polygons, args.route, args.nav, entrance) for r in manifest['entries']]
        receipt = dict(map_id=manifest['map_id'], policy=PLACEMENT_POLICY, db2_authentication=auth | binding_auth,
                       producer_sha256=digest(__file__), source_reader_sha256=digest(Path(__file__).with_name('follower_placement_sources.py')),
                       calibration=calibrate(baseline, manifest['calibration_entries'], tables, args.cache),
                       calibration_scope='Horizontal transform and floor controls; Wowhead markers are approximate observations, not exact native spawn capture',
                       nav_directory=str(args.nav.resolve()), nav_files_sha256={p.name: digest(p) for p in args.nav.glob('*.nav')},
                       transform='u,v=percent/100; X=Region3-(v-UiMin1)/(UiMax1-UiMin1)*(Region3-Region0); Y=Region4-(u-UiMin0)/(UiMax0-UiMin0)*(Region4-Region1)',
                       manifest_sha256=digest(args.manifest), inventory_sha256=digest(args.inventory),
                       surface_file=str(args.surfaces), surface_sha256=digest(args.surfaces),
                       route_binary_sha256=digest(args.route), entrance=entrance, placements=placements,
                       source_database=str(args.source_db.resolve()), baseline_sha256=digest(baseline_path), canonical_modified=False)
        receipt['navigation_gaps'] = [p['navigation_gap'] for p in placements if 'navigation_gap' in p]
        if any('wow_xyz' not in p for p in placements):
            receipt['status'] = 'BLOCKED_SURFACE_AMBIGUITY_OR_NO_ROUTE'
        else:
            bindings = [derive_binding(baseline, inventory, binding_tables, manifest['map_id'], r['entry']) for r in manifest['entries']]
            backup(baseline, prepared_path)
            with closing(sqlite3.connect(prepared_path)) as prepared, prepared:
                receipt['donors'] = prepare_templates(prepared, manifest['entries'], args.cache)
                write_placements(prepared, manifest, placements, bindings)
                sql, changed = make_sql(baseline, prepared)
                receipt['changed_keys'] = changed
            backup(baseline, replay_path)
            with closing(sqlite3.connect(replay_path)) as replay, closing(open_readonly(prepared_path)) as prepared:
                replay.executescript(sql)
                expected = {t: read_table_rows(prepared, t) for t in TABLE_KEYS}
                for _ in range(2):
                    for table in TABLE_KEYS:
                        if read_table_rows(replay, table) != expected[table]:
                            raise ValueError(f'SQL replay differs for{table}')
                    replay.executescript(sql)
            (args.output / 'delta.sql').write_text(sql)
            status = 'PREPARED_WITH_NAVIGATION_GAPS' if receipt['navigation_gaps'] else 'PREPARED_NOT_PUBLISHED'
            receipt.update(status=status, sql_sha256=digest(args.output / 'delta.sql'),
                           replayed_sha256=digest(replay_path), sql_replay_count=3,
                           guid_range=[manifest['guid_base'] + manifest['map_id'] * 1000,
                                       manifest['guid_base'] + manifest['map_id'] * 1000 + 999])
    (args.output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(receipt['status'])
    return 0 if receipt['status'].startswith('PREPARED_') else 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source-db', 'manifest', 'inventory', 'db2', 'cache', 'surfaces', 'route', 'nav', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    return prepare(parser.parse_args())


if __name__ == '__main__':
    raise SystemExit(main())
