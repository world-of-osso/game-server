#!/usr/bin/env python3
"""Prepare one sourced map packet; publish nothing and never write canonical world.db."""
import argparse
from contextlib import closing
import csv
import json
import re
from pathlib import Path
import sqlite3
import subprocess

import retail_content
from follower_placement_sources import (digest, read_cached_npc, read_db2,
    read_journal_marker, read_source_floor_support, read_surface_polygons, sample_surfaces, select_assignment, transform_marker)

PLACEMENT_POLICY = 'FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS'
JOURNAL_PLACEMENT_POLICY = 'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP'
LUA_BINDING_POLICY = 'FOLLOWER_BINDING_LUA_ENTRY_TO_UNIQUE_JOURNAL_ACTOR'
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


def select_surface_query(record, marker, queries):
    evidence = record.get('source_floor_evidence')
    if isinstance(evidence, dict) and evidence.get('policy') == 'FOLLOWER_PLACEMENT_Z_FROM_SAME_ROOM_NATIVE_ANCHORS':
        supported = [q for q in queries if all(abs(q['height'] - a['wow_xyz'][2]) <=
                     evidence['maximum_vertical_difference_yards'] for a in evidence['anchors'])]
        if len(supported) != 1 or marker['floor'] != record['source_floor']:
            raise ValueError('same-room native anchors must support exactly one baked candidate on the source floor')
        return supported[0]
    if 'source_floor_surface_height' in record:
        selected = [q for q in queries if abs(q['height'] - record['source_floor_surface_height']) < 0.000001]
        if len(selected) != 1 or marker['floor'] != record['source_floor']:
            raise ValueError('approved source-floor surface no longer matches baked geometry')
        return selected[0]
    if len(queries) == 1:
        return queries[0]
    reachable = {tuple(q['corners'][-1]): q for q in queries if q['exit_code'] == 0 and q['corners']}
    return next(iter(reachable.values())) if len(reachable) == 1 else None


def compare_wowhead_marker(record, tables, cache, map_id, assignments, xy):
    page = read_cached_npc(cache, record['entry'])
    if page['cache_sha256'] != record['cache_sha256']:
        raise ValueError('pinned Wowhead comparison evidence changed')
    comparison = dict(page=page, status='NO_INDIVIDUAL_WOWHEAD_MARKER')
    if len(page['markers']) == 1:
        marker = page['markers'][0]
        area = next(r for r in tables['AreaTable'] if int(r['ID']) == marker['zone'])
        if int(area['ContinentID']) != map_id:
            raise ValueError('Wowhead comparison marker belongs to a different map')
        ui_maps = {a['UiMapID'] for a in assignments}
        floors = {int(g['FloorIndex']) for g in tables['UiMapGroupMember'] if g['UiMapID'] in ui_maps}
        if floors and marker['floor'] not in floors:
            comparison.update(status='DIFFERENT_SOURCED_FLOOR', finding='FOLLOWER_JOURNAL_WOWHEAD_FLOOR_DISAGREEMENT')
            return comparison
        observed = transform_marker(marker, assignments)
        distance = sum((a - b) ** 2 for a, b in zip(xy, observed)) ** 0.5
        comparison.update(status='COMPARED', wowhead_xy=observed, distance_yards=distance)
        if distance > 5:
            comparison['finding'] = 'FOLLOWER_JOURNAL_WOWHEAD_XY_DISAGREEMENT'
    return comparison


def derive_placement(record, manifest, tables, cache, polygons, route, nav, entrance):
    if manifest['placement_policy'] == JOURNAL_PLACEMENT_POLICY:
        page, marker, assignments = read_journal_marker(tables, manifest['map_id'], record['entry'], record['journal_encounter_id'])
    else:
        page = read_cached_npc(cache, record['entry'])
        if page['cache_sha256'] != record['cache_sha256'] or len(page['markers']) != 1:
            raise ValueError(f"NPC{record['entry']} pinned marker evidence changed")
        marker = page['markers'][0]
        assignments = select_assignment(tables, manifest['map_id'], marker)
    x, y = transform_marker(marker, assignments)
    evidence = record.get('source_floor_evidence')
    if isinstance(evidence, dict) and evidence.get('policy') == 'FOLLOWER_PLACEMENT_Z_FROM_SAME_ROOM_NATIVE_ANCHORS':
        if [x, y] != evidence['marker_wow_xy']:
            raise ValueError('same-room anchor evidence differs from current source XY')
    all_heights = sample_surfaces(polygons, x, y, assignments)
    heights = read_source_floor_support(record, marker, assignments, all_heights)
    start = [entrance['LocX'], entrance['LocZ'], -entrance['LocY']]
    queries = [dict(height=z, **query_route(route, nav, start, [x, z, -y])) for z in heights]
    selected = select_surface_query(record, marker, queries)
    placement = dict(entry=record['entry'], page=page, marker=marker,
                     assignments=assignments, wow_xy=[x, y], all_surface_heights=all_heights,
                     surface_queries=queries, status='MISSING_OR_AMBIGUOUS_SURFACE')
    if manifest['placement_policy'] == JOURNAL_PLACEMENT_POLICY and 'cache_sha256' in record:
        placement['wowhead_comparison'] = compare_wowhead_marker(record, tables, cache, manifest['map_id'], assignments, [x, y])
    if 'source_floor_evidence' in record:
        placement['source_floor_evidence'] = record['source_floor_evidence']
    if selected is not None:
        placement['wow_xyz'] = [x, y, selected['height']]
        placement['status'] = 'UNIQUE_REACHABLE_SURFACE' if selected['exit_code'] == 0 else 'SOURCE_FLOOR_ROUTE_GAP'
        if selected['exit_code'] != 0:
            placement['navigation_gap'] = record.get('navigation_gap', f"FOLLOWER_ROUTE_INCOMPLETE_{manifest['map_id']}_{record['entry']}")
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


def validate_lua_primary_identity(identity, inventory, encounter, map_id, entry, journal_id):
    path = Path(identity['path'])
    if identity['policy'] != LUA_BINDING_POLICY or identity['revision'] != inventory['littlewigs'] or digest(path) != identity['sha256']:
        raise ValueError('pinned Lua primary identity evidence changed')
    if not path.as_posix().endswith('/' + encounter['module']):
        raise ValueError('Lua primary module differs from pinned source inventory')
    text = path.read_text()
    enabled = re.findall(r'RegisterEnableMob\(([^)]*)\)', text)
    entries = {int(token) for args in enabled for token in re.findall(r'\b\d+\b', args)}
    encounter_ids = re.findall(r'SetEncounterID\(\s*(\d+)\s*\)', text)
    journals = re.findall(r'NewBoss\("[^"]+",\s*(\d+),\s*(\d+)\)', text)
    if entry not in entries or encounter_ids != [str(encounter['encounter']['ID'])] or (str(map_id), str(journal_id)) not in journals:
        raise ValueError('Lua source does not identify this primary entry, encounter and journal')


def derive_binding(source, inventory, tables, map_id, entry, identity=None):
    encounter = next(e for e in inventory['encounters'] if e['MapID'] == map_id and entry in e['enabled_entries'])
    dungeon = next(r for r in tables['DungeonEncounter'] if int(r['ID']) == encounter['encounter']['ID'])
    journal = next(r for r in tables['JournalEncounter'] if int(r['ID']) in encounter['journal_ids'])
    participants = [r for r in tables['JournalEncounterCreature'] if r['JournalEncounterID'] == journal['ID']]
    if identity is not None:
        validate_lua_primary_identity(identity, inventory, encounter, map_id, entry, int(journal['ID']))
        name = source.execute('SELECT name FROM tdb_creature_template WHERE entry=?', (entry,)).fetchone()[0]
        actors = [r for r in participants if r['Name_lang'] == name]
        if len(actors) != 1 or int(actors[0]['ID']) != identity['journal_actor_id']:
            raise ValueError(f'NPC{entry} has no unique exact-name journal actor for its pinned Lua identity')
    else:
        displays = {r[0] for r in source.execute('SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?', (entry,))}
        actors = [r for r in participants if int(r['CreatureDisplayInfoID']) in displays]
    if len(actors) != 1 or int(dungeon['MapID']) != map_id or journal['DungeonEncounterID'] != dungeon['ID']:
        raise ValueError(f'NPC{entry} lacks an unambiguous authenticated primary identity')
    actor = actors[0]
    return (map_id, entry, int(dungeon['ID']), int(dungeon['Bit']), int(journal['ID']), int(actor['ID']), int(actor['CreatureDisplayInfoID']))


def stage_source_difficulties(target, records):
    requested = [(r['entry'], 1) for r in records]
    resolved = retail_content.resolve_difficulty_rows(target, requested)
    missing = [entry for entry, difficulty in requested if (entry, difficulty) not in resolved]
    if missing:
        raise ValueError(f'placement actors lack native difficulty-chain rows: {missing}')
    target.execute('CREATE TEMP TABLE follower_source_difficulty(entry INTEGER PRIMARY KEY, source_difficulty INTEGER)')
    target.executemany('INSERT INTO temp.follower_source_difficulty VALUES(?,?)',
                       [(entry, resolved[(entry, difficulty)]) for entry, difficulty in requested])
    target.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT d.* FROM tdb_creature_template_difficulty d '
                   'JOIN temp.follower_source_difficulty s ON s.entry=d.Entry AND s.source_difficulty=d.DifficultyID')
    cursor = target.execute('SELECT * FROM temp.tdb_difficulty ORDER BY Entry')
    columns = [c[0] for c in cursor.description]
    rows = [dict(zip(columns, row)) for row in cursor]
    for row in rows:
        levels = target.execute('SELECT minlevel,maxlevel FROM content_tuning_level WHERE ID=?', (row['ContentTuningID'],)).fetchone()
        if row['ContentTuningID'] <= 0 or levels is None:
            raise ValueError(f"FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_{row['Entry']}")
    return [dict(entry=r['Entry'], requested_difficulty_id=1, source_difficulty_id=r['DifficultyID'],
                 policy='FOLLOWER_PLACEMENT_SOURCE_DIFFICULTY_CHAIN', source_row=r) for r in rows]


def prepare_templates(target, records, cache):
    entries = ','.join(str(r['entry']) for r in records)
    difficulties = stage_source_difficulties(target, records)
    target.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    retail_content.insert_tdb_templates(target, f'SELECT Entry FROM temp.tdb_difficulty')
    donors = []
    for record in records:
        native_faction = target.execute('SELECT faction FROM tdb_creature_template WHERE entry=?', (record['entry'],)).fetchone()[0]
        if native_faction > 0:
            continue
        actor = read_cached_npc(cache, record['entry'])
        donor = read_cached_npc(cache, record['faction_donor'])
        faction = target.execute('SELECT faction FROM tdb_creature_template WHERE entry=?', (record['faction_donor'],)).fetchone()[0]
        if actor['entity']['react'] != [-1, -1] or donor['entity']['react'] != actor['entity']['react'] or faction <= 0:
            raise ValueError('faction donor does not have the sourced matching hostile A/H reaction')
        target.execute('UPDATE content_creature_template SET faction=? WHERE entry=? AND faction=0', (faction, record['entry']))
        donors.append(dict(entry=record['entry'], donor_entry=record['faction_donor'], donor_page=donor,
                           faction=faction, policy=FACTION_POLICY, reason=record['donor_reason']))
    target.execute(f'INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID IN({entries})')
    return donors, difficulties


def write_placements(target, manifest, placements, bindings):
    lower = manifest['guid_base'] + manifest['map_id'] * 1000
    expected = {lower + r['ordinal']: r['entry'] for r in manifest['entries']}
    occupied = target.execute('SELECT guid,id1,map FROM content_creature WHERE guid BETWEEN ? AND ?', (lower, lower + 999)).fetchall()
    if any(guid not in expected or entry != expected[guid] or map_id != manifest['map_id'] for guid, entry, map_id in occupied):
        raise ValueError('reserved map GUID namespace overlaps unrelated content')
    for record, placement, binding in zip(manifest['entries'], placements, bindings):
        ordinal = record['ordinal']
        if not 0 <= ordinal < 1000:
            raise ValueError('placement ordinal exceeds reserved map namespace')
        guid = manifest['guid_base'] + manifest['map_id'] * 1000 + ordinal
        x, y, z = placement['wow_xyz']
        comment = f"{manifest['placement_policy']}; {placement['page']['url']}; floor={placement['marker']['floor']}; percent={placement['marker']['percent']}"
        existing = target.execute('SELECT id1,map,Comment,position_x,position_y,position_z FROM content_creature WHERE guid=?', (guid,)).fetchone()
        if existing is not None and tuple(existing) != (record['entry'], manifest['map_id'], comment, x, y, z):
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


def validate_native_room_anchors(source, manifest):
    proofs = []
    for record in manifest['entries']:
        evidence = record.get('source_floor_evidence')
        if not isinstance(evidence, dict) or evidence.get('policy') != 'FOLLOWER_PLACEMENT_Z_FROM_SAME_ROOM_NATIVE_ANCHORS':
            continue
        anchors = evidence['anchors']
        if len({a['guid'] for a in anchors}) < 2:
            raise ValueError('same-room floor policy requires at least two distinct native anchors')
        x, y = evidence['marker_wow_xy']
        floor = record['source_floor_reference_height']
        observed = []
        for anchor in anchors:
            native = source.execute('SELECT id,map,position_x,position_y,position_z FROM tdb_creature WHERE guid=?',
                                    (anchor['guid'],)).fetchone()
            if native is None or tuple(native) != (anchor['entry'], manifest['map_id'], *anchor['wow_xyz']):
                raise ValueError('same-room native anchor differs from recorded source')
            distance = ((x - native[2]) ** 2 + (y - native[3]) ** 2) ** 0.5
            vertical = abs(floor - native[4])
            if distance > evidence['maximum_horizontal_distance_yards'] or vertical > evidence['maximum_vertical_difference_yards']:
                raise ValueError('same-room native anchors do not agree on the supported floor')
            observed.append(dict(anchor, horizontal_distance_yards=distance, reference_vertical_difference_yards=vertical))
        proofs.append(dict(entry=record['entry'], policy=evidence['policy'], reference_floor_height=floor, anchors=observed))
    return proofs


def prepare(args):
    if args.output.exists():
        raise ValueError('output directory must be new; never overwrite proof or inputs')
    args.output.mkdir(parents=True)
    manifest = json.loads(args.manifest.read_text())
    if manifest['placement_policy'] not in (PLACEMENT_POLICY, JOURNAL_PLACEMENT_POLICY) or manifest['faction_policy'] != FACTION_POLICY:
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
        native_room_anchors = validate_native_room_anchors(baseline, manifest)
        placements = [derive_placement(r, manifest, tables | binding_tables, args.cache, polygons, args.route, args.nav, entrance) for r in manifest['entries']]
        receipt = dict(map_id=manifest['map_id'], policy=manifest['placement_policy'], db2_authentication=auth | binding_auth,
                       producer_sha256=digest(__file__), source_reader_sha256=digest(Path(__file__).with_name('follower_placement_sources.py')),
                       calibration=calibrate(baseline, manifest['calibration_entries'], tables, args.cache),
                       calibration_scope='Horizontal transform and floor controls; Wowhead markers are approximate observations, not exact native spawn capture',
                       nav_directory=str(args.nav.resolve()), nav_files_sha256={p.name: digest(p) for p in args.nav.glob('*.nav')},
                       transform='u,v=percent/100; X=Region3-(v-UiMin1)/(UiMax1-UiMin1)*(Region3-Region0); Y=Region4-(u-UiMin0)/(UiMax0-UiMin0)*(Region4-Region1)',
                       manifest_sha256=digest(args.manifest), inventory_sha256=digest(args.inventory),
                       surface_file=str(args.surfaces), surface_sha256=digest(args.surfaces),
                       route_binary_sha256=digest(args.route), entrance=entrance, placements=placements,
                       native_room_anchors=native_room_anchors, source_gaps=manifest.get('source_gaps', []),
                       source_database=str(args.source_db.resolve()), baseline_sha256=digest(baseline_path), canonical_modified=False)
        receipt['navigation_gaps'] = [p['navigation_gap'] for p in placements if 'navigation_gap' in p]
        receipt['source_findings'] = [dict(entry=p['entry'], **p['wowhead_comparison']) for p in placements
                                      if 'finding' in p.get('wowhead_comparison', {})]
        if any('wow_xyz' not in p for p in placements):
            receipt['status'] = 'BLOCKED_SURFACE_AMBIGUITY_OR_NO_ROUTE'
        else:
            bindings = [derive_binding(baseline, inventory, binding_tables, manifest['map_id'], r['entry'], r.get('journal_actor_source')) for r in manifest['entries']]
            for record, placement in zip(manifest['entries'], placements):
                if 'journal_actor_source' in record:
                    placement['binding_source'] = record['journal_actor_source']
                    placement['native_template_display_ids'] = [row[0] for row in baseline.execute(
                        'SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?', (record['entry'],))]
            backup(baseline, prepared_path)
            with closing(sqlite3.connect(prepared_path)) as prepared, prepared:
                receipt['donors'], receipt['source_difficulties'] = prepare_templates(prepared, manifest['entries'], args.cache)
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
