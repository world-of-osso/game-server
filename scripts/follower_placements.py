#!/usr/bin/env python3
"""Prepare one sourced map packet; publish nothing and never write canonical world.db."""
import argparse
from contextlib import closing
import csv
import hashlib
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
NATIVE_BINDING_POLICY = 'FOLLOWER_BINDING_EXACT_NATIVE_NAME_DISPLAY_TO_JOURNAL_ACTOR'
FACTION_POLICY = 'FOLLOWER_PLACEMENT_FACTION_FROM_REACT_AND_DONOR'
JOURNAL_HOSTILITY_POLICY = 'FOLLOWER_BOSS_HOSTILE_FROM_JOURNAL_ENCOUNTER'
TRASH_HOSTILITY_POLICY = 'FOLLOWER_TRASH_HOSTILE_FROM_MDT_ENEMY_TABLE'
MAP_TUNING_POLICY = 'FOLLOWER_PLACEMENT_CONTENT_TUNING_FROM_NATIVE_MAP_DIFFICULTY'
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
    if entrance is None:
        if len(all_heights) != 1:
            raise ValueError('entrance-unsourced placement requires one baked surface at journal XY')
        queries = [dict(height=z, exit_code=None, corners=[], status='NOT_QUERIED_ENTRANCE_UNSOURCED') for z in heights]
    else:
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
        if entrance is None:
            placement['status'] = 'SOURCE_PLACEMENT_ENTRANCE_UNSOURCED'
            placement['entrance_gap'] = f"FOLLOWER_ENTRANCE_UNSOURCED_{manifest['map_id']}"
        else:
            placement['status'] = 'UNIQUE_REACHABLE_SURFACE' if selected['exit_code'] == 0 else 'SOURCE_FLOOR_ROUTE_GAP'
        if entrance is not None and selected['exit_code'] != 0:
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


def derive_exact_native_binding(source, tables, map_id, entry, identity):
    journal = next(row for row in tables['JournalEncounter'] if int(row['ID']) == identity['journal_encounter_id'])
    dungeon = next(row for row in tables['DungeonEncounter'] if row['ID'] == journal['DungeonEncounterID'])
    name = source.execute('SELECT name FROM tdb_creature_template WHERE entry=?', (entry,)).fetchone()[0]
    displays = {row[0] for row in source.execute('SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?', (entry,))}
    actors = [row for row in tables['JournalEncounterCreature'] if row['JournalEncounterID'] == journal['ID']
              and row['Name_lang'] == name and int(row['CreatureDisplayInfoID']) in displays]
    if len(actors) != 1 or int(dungeon['MapID']) != map_id or int(actors[0]['ID']) != identity['journal_actor_id']:
        raise ValueError('require one journal actor with exact native name and display on the source map')
    actor = actors[0]
    candidates = source.execute('SELECT DISTINCT t.entry FROM tdb_creature_template t JOIN tdb_creature_template_model m '
                                'ON m.CreatureID=t.entry WHERE t.name=? AND m.CreatureDisplayID=?',
                                (name, int(actor['CreatureDisplayInfoID']))).fetchall()
    if {row[0] for row in candidates} != {entry}:
        raise ValueError('exact native name and display has ambiguous template entries')
    return (map_id, entry, int(dungeon['ID']), int(dungeon['Bit']), int(journal['ID']), int(actor['ID']), int(actor['CreatureDisplayInfoID']))


def derive_binding(source, inventory, tables, map_id, entry, identity=None):
    if identity is not None and identity['policy'] == NATIVE_BINDING_POLICY:
        return derive_exact_native_binding(source, tables, map_id, entry, identity)
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


def qualify_map_content_tuning(target, record):
    """TC Creature.cpp3122–3150: noncritter CT0 uses its native map difficulty."""
    if record.get('content_tuning_policy') != MAP_TUNING_POLICY:
        return None
    creature_type = target.execute('SELECT type FROM tdb_creature_template WHERE entry=?', (record['entry'],)).fetchone()[0]
    if creature_type == 8:
        return None
    cursor = target.execute('SELECT * FROM map_difficulty WHERE MapID=? AND DifficultyID=1', (record['map_id'],))
    columns = [column[0] for column in cursor.description]
    rows = [dict(zip(columns, row)) for row in cursor]
    if len(rows) != 1 or rows[0]['ContentTuningID'] <= 0:
        return None
    selected = rows[0]
    levels = target.execute('SELECT minlevel,maxlevel FROM content_tuning_level WHERE ID=?', (selected['ContentTuningID'],)).fetchone()
    if levels is None:
        return None
    return dict(policy=MAP_TUNING_POLICY, source='TrinityCore a352b1fa Creature.cpp3122–3150',
                map_difficulty_row=selected, content_tuning_id=selected['ContentTuningID'],
                runtime_level_range=list(levels), creature_type=creature_type,
                native_creature_row_preserved=True)


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
    proofs = []
    by_entry = {record['entry']: record for record in records}
    for row in rows:
        proof = dict(entry=row['Entry'], requested_difficulty_id=1, source_difficulty_id=row['DifficultyID'],
                     policy='FOLLOWER_PLACEMENT_SOURCE_DIFFICULTY_CHAIN', source_row=row)
        if row['ContentTuningID'] == 0:
            regional = qualify_map_content_tuning(target, by_entry[row['Entry']])
            if regional is None:
                raise ValueError(f"FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_{row['Entry']}")
            proof['effective_content_tuning'] = regional
        elif target.execute('SELECT 1 FROM content_tuning_level WHERE ID=?', (row['ContentTuningID'],)).fetchone() is None:
            raise ValueError(f"FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_{row['Entry']}")
        proofs.append(proof)
    return proofs


def qualify_journal_hostility(target, record, tables):
    identity = record['journal_actor_source']
    if identity['policy'] != NATIVE_BINDING_POLICY:
        raise ValueError('journal hostility requires the unique exact native actor binding')
    binding = derive_exact_native_binding(target, tables, record['map_id'], record['entry'], identity)
    follower = [row for row in tables['MapDifficulty']
                if int(row['MapID']) == record['map_id'] and row['DifficultyID'] == '205']
    if len(follower) != 1:
        raise ValueError('journal hostility requires one authenticated follower-difficulty map row')
    cursor = target.execute('SELECT * FROM map_difficulty WHERE MapID=? AND DifficultyID=205', (record['map_id'],))
    columns = [column[0] for column in cursor.description]
    native = [{key: str(value) for key, value in zip(columns, row)} for row in cursor]
    if native != follower:
        raise ValueError('native follower map differs from authenticated DB2')
    rows = {name: next(row for row in tables[name] if int(row['ID']) == row_id)
            for name, row_id in [('JournalEncounterCreature', binding[5]), ('JournalEncounter', binding[4]),
                                 ('DungeonEncounter', binding[2]), ('MapDifficulty', int(follower[0]['ID']))]}
    return dict(policy=JOURNAL_HOSTILITY_POLICY, binding=list(binding),
                rows={name: dict(row=row, sha256=hashlib.sha256(json.dumps(row, sort_keys=True, separators=(',', ':')).encode()).hexdigest())
                      for name, row in rows.items()})


def qualify_mdt_hostility(target, record, tables):
    evidence = record['mdt_enemy_source']
    roots = {'0ec5f50dea16cfc19f288de91125c70fa7455961': Path('/home/osso/.worktrees/mdt-0ec5f50'),
             'c6cccca29cb2612b8303cdfa543bc6c60f0f7c7f': Path('/home/osso/.worktrees/mdt-c6cccca'),
             'cb3c6cd723698178ff8ea500b62d28ef792edce8': Path('/home/osso/.worktrees/mdt-cb3c6cd72369')}
    path = Path(evidence['path']).resolve()
    root = roots.get(evidence['revision'])
    if evidence['policy'] != TRASH_HOSTILITY_POLICY or root is None or not path.is_relative_to(root.resolve()) or digest(path) != evidence['sha256']:
        raise ValueError('MDT enemy source is not the approved pinned file/revision/hash')
    text = path.read_text()
    challenge = re.search(r'\bmapID = (\d+)', text)
    maps = [r for r in tables['MapChallengeMode'] if challenge and int(r['ID']) == int(challenge[1]) and int(r['MapID']) == record['map_id']]
    if len(maps) != 1:
        raise ValueError('MDT challenge-mode row does not identify the requested dungeon')
    start = text.index('MDT.dungeonEnemies[dungeonIndex] = {')
    entries = []
    for match in re.finditer(r'^    \["id"\] = (\d+),', text[start:], re.M):
        if int(match[1]) == record['entry']:
            entries.append(start + match.start())
    if len(entries) != 1 or text[:entries[0]].count('\n') + 1 != evidence['source_line']:
        raise ValueError('MDT table lacks the unique enemy entry at its pinned source line')
    before = text[:entries[0]].splitlines()
    name = re.fullmatch(r'    \["name"\] = "(.*)",', before[-1])
    native = target.execute('SELECT name FROM tdb_creature_template WHERE entry=?', (record['entry'],)).fetchone()
    if name is None or native is None or name[1] != native[0]:
        raise ValueError('MDT enemy does not match its own native template name')
    return dict(policy=TRASH_HOSTILITY_POLICY, source=evidence,
                mdt_enemy=dict(entry=record['entry'], name=native[0], source_line=evidence['source_line']),
                db2_rows=dict(MapChallengeMode=maps[0]))


def prepare_templates(target, records, cache, tables=None):
    entries = ','.join(str(r['entry']) for r in records)
    difficulties = stage_source_difficulties(target, records)
    target.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    retail_content.insert_tdb_templates(target, f'SELECT Entry FROM temp.tdb_difficulty')
    donors = []
    for record in records:
        native_faction = target.execute('SELECT faction FROM tdb_creature_template WHERE entry=?', (record['entry'],)).fetchone()[0]
        if native_faction != 0:
            continue
        donor = read_cached_npc(cache, record['faction_donor'])
        faction = target.execute('SELECT faction FROM tdb_creature_template WHERE entry=?', (record['faction_donor'],)).fetchone()[0]
        hostility = None
        if record.get('faction_policy') in (JOURNAL_HOSTILITY_POLICY, TRASH_HOSTILITY_POLICY):
            if tables is None:
                raise ValueError('source hostility requires authenticated DB2 tables')
            if record['faction_policy'] == TRASH_HOSTILITY_POLICY:
                hostility = qualify_mdt_hostility(target, record, tables)
            else:
                hostility = qualify_journal_hostility(target, record, tables)
            if faction != 16 or donor['entity']['react'] != [-1, -1]:
                raise ValueError('known-hostile donor must have native faction16 and sourced hostile A/H reaction')
        else:
            actor = read_cached_npc(cache, record['entry'])
            if actor['entity']['react'] != [-1, -1] or donor['entity']['react'] != actor['entity']['react'] or faction <= 0:
                raise ValueError('faction donor does not have the sourced matching hostile A/H reaction')
        target.execute('UPDATE content_creature_template SET faction=? WHERE entry=? AND faction=0', (faction, record['entry']))
        proof = dict(entry=record['entry'], donor_entry=record['faction_donor'], donor_page=donor,
                     faction=faction, policy=FACTION_POLICY, reason=record['donor_reason'])
        if hostility is not None:
            proof['hostility_source'] = hostility
        donors.append(proof)
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
        write_primary_binding(target, binding)
        placement['guid'] = guid
        placement['binding'] = binding
        placement['orientation_policy'] = 'unsourced schema-default orientation; stationary project placement, not Retail facing'


def write_primary_binding(target, binding):
    existing = target.execute('SELECT * FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?', binding[:2]).fetchone()
    if existing is not None and tuple(existing) != binding:
        raise ValueError(f'primary binding conflicts for{binding[:2]}')
    target.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', binding)


def read_native_binding_entries(source, manifest):
    proofs = []
    for record in manifest.get('native_binding_only_entries', []):
        rows = []
        for table, guid in [('tdb_creature', record['native_guid']), ('content_creature', record['runtime_guid'])]:
            cursor = source.execute(f'SELECT * FROM {table} WHERE guid=?', (guid,))
            row = cursor.fetchone()
            if row is None:
                raise ValueError('native binding-only actor is absent from the source population')
            rows.append(dict(zip([column[0] for column in cursor.description], row)))
        native, runtime = rows
        fields = ['map', 'position_x', 'position_y', 'position_z', 'spawnDifficulties', 'VerifiedBuild']
        expected = (manifest['map_id'], *record['source_xyz'], record['spawn_difficulties'], record['source_verified_build'])
        if native['id'] != record['entry'] or runtime['id1'] != record['entry'] or any(tuple(row[key] for key in fields) != expected for row in rows):
            raise ValueError('native binding-only actor differs from its exact authored pose/mask/build')
        occupied = source.execute('SELECT guid FROM content_creature WHERE map=? AND id1=?', (manifest['map_id'], record['entry'])).fetchall()
        if [row[0] for row in occupied] != [record['runtime_guid']]:
            raise ValueError('native binding-only actor has duplicate runtime placements')
        proofs.append(dict(entry=record['entry'], native_spawn=native, runtime_spawn=runtime,
                           policy='NATIVE_TDB_SPAWN_PRESERVED_BINDING_ONLY', source_record=record))
    return proofs


def validate_sourced_duo(manifest, placements, native_bindings, cache):
    policy = manifest.get('duo_placement_policy')
    if policy is None:
        return []
    if policy != 'FOLLOWER_PLACEMENT_DUO_INDIVIDUALLY_SOURCED_SAME_MARKER' or manifest['map_id'] != 2874:
        raise ValueError('individually sourced same-marker policy is approved only for Muro/Nekraxx')
    records = manifest['entries'] + manifest.get('native_binding_only_entries', [])
    if sorted(record['entry'] for record in records) != [247570, 247572]:
        raise ValueError('same-marker policy requires both individually identified Muro/Nekraxx actors')
    pages = []
    for record in sorted(records, key=lambda row: row['entry']):
        page = read_cached_npc(cache, record['entry'])
        if page['cache_sha256'] != record['cache_sha256'] or len(page['markers']) != 1 or record['journal_encounter_id'] != 2810:
            raise ValueError('duo individual marker source changed or is absent')
        pages.append(page)
    if pages[0]['markers'] != pages[1]['markers']:
        raise ValueError('duo individual pages do not independently identify the same marker')
    if len(placements) != 1 or placements[0]['entry'] != 247570 or len(native_bindings) != 1 or native_bindings[0]['entry'] != 247572:
        raise ValueError('preserve native Nekraxx as corroboration; place only missing Muro')
    return [dict(policy=policy, individual_pages=pages, journal_marker=placements[0]['marker'],
                 journal_xy=placements[0]['wow_xy'], native_corroboration=native_bindings[0],
                 marker_precision_scope='Both independent pages share the same rounded marker; use unchanged precise journal XY for Muro, preserve native Nekraxx XYZ.')]


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


def make_faction_update_sql(prepared, columns, before, after):
    faction = columns.index('faction')
    if before[faction] != 0 or after[faction] != 16 or any(a != b for index, (a, b) in enumerate(zip(before, after)) if index != faction):
        raise ValueError('guarded correction permits only native-zero faction16 with every other column unchanged')
    quote = lambda value: prepared.execute('SELECT quote(?)', (value,)).fetchone()[0]
    unchanged = ' AND '.join(f'{name} IS {quote(value)}' for index, (name, value) in enumerate(zip(columns, before)) if index != faction)
    return [f'INSERT INTO follower_placement_guard SELECT EXISTS(SELECT 1 FROM content_creature_template WHERE {unchanged} AND (faction IS 0 OR faction IS 16));',
            f'UPDATE content_creature_template SET faction=16 WHERE {unchanged} AND faction IS 0;']


def make_sql(baseline, prepared, faction_update_entries=()):
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
                    if table != 'content_creature_template' or key[0] not in faction_update_entries:
                        raise ValueError(f'placement packet must not replace existing {table}{key}')
                    statements.extend(make_faction_update_sql(prepared, columns, original[key], row))
                    inserted.append(key)
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
    regional_records = [r for r in manifest['entries'] if 'content_tuning_policy' in r]
    if any(r['content_tuning_policy'] != MAP_TUNING_POLICY or r['map_id'] != manifest['map_id'] for r in regional_records):
        raise ValueError('map tuning policy or actor map differs from placement manifest')
    tables, auth = read_db2(args.db2, ('MapDifficulty',) if regional_records else ())
    binding_tables, binding_auth = read_binding_tables(args.db2)
    inventory = json.loads(args.inventory.read_text())
    polygons = read_surface_polygons(args.surfaces)
    baseline_path, prepared_path, replay_path = [args.output / name for name in ('baseline.sqlite', 'prepared.sqlite', 'replayed.sqlite')]
    with closing(open_readonly(args.source_db)) as source:
        backup(source, baseline_path)
    with closing(open_readonly(baseline_path)) as baseline:
        baseline.row_factory = sqlite3.Row
        if manifest['entrance_id'] is None:
            if manifest.get('entrance_gap') != f"FOLLOWER_ENTRANCE_UNSOURCED_{manifest['map_id']}":
                raise ValueError('missing entrance requires the explicit named entrance-source gap')
            entrance = None
        else:
            entrance = dict(baseline.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=? AND MapID=?', (manifest['entrance_id'], manifest['map_id'])).fetchone())
        if regional_records:
            cursor = baseline.execute('SELECT * FROM map_difficulty WHERE MapID=? AND DifficultyID=1', (manifest['map_id'],))
            native_maps = [{key: str(value) for key, value in dict(row).items()} for row in cursor]
            authenticated_maps = [row for row in tables['MapDifficulty']
                                  if int(row['MapID']) == manifest['map_id'] and row['DifficultyID'] == '1']
            if len(native_maps) != 1 or native_maps != authenticated_maps:
                raise ValueError('native map difficulty differs from authenticated DB2')
        native_room_anchors = validate_native_room_anchors(baseline, manifest)
        placements = [derive_placement(r, manifest, tables | binding_tables, args.cache, polygons, args.route, args.nav, entrance) for r in manifest['entries']]
        native_bindings = read_native_binding_entries(baseline, manifest)
        for proof in native_bindings:
            record = proof['source_record']
            proof['binding'] = derive_binding(baseline, inventory, binding_tables, manifest['map_id'], record['entry'], record['journal_actor_source'])
        source_relationships = validate_sourced_duo(manifest, placements, native_bindings, args.cache)
        receipt = dict(map_id=manifest['map_id'], policy=manifest['placement_policy'], db2_authentication=auth | binding_auth,
                       producer_sha256=digest(__file__), source_reader_sha256=digest(Path(__file__).with_name('follower_placement_sources.py')),
                       calibration=calibrate(baseline, manifest['calibration_entries'], tables, args.cache),
                       calibration_scope='Horizontal transform and floor controls; Wowhead markers are approximate observations, not exact native spawn capture',
                       nav_directory=str(args.nav.resolve()), nav_files_sha256={p.name: digest(p) for p in args.nav.glob('*.nav')},
                       transform='u,v=percent/100; X=Region3-(v-UiMin1)/(UiMax1-UiMin1)*(Region3-Region0); Y=Region4-(u-UiMin0)/(UiMax0-UiMin0)*(Region4-Region1)',
                       manifest_sha256=digest(args.manifest), inventory_sha256=digest(args.inventory),
                       surface_file=str(args.surfaces), surface_sha256=digest(args.surfaces),
                       route_binary_sha256=digest(args.route), entrance=entrance, placements=placements,
                       native_room_anchors=native_room_anchors, native_binding_only=native_bindings,
                       source_relationships=source_relationships, source_gaps=manifest.get('source_gaps', []),
                       source_database=str(args.source_db.resolve()), baseline_sha256=digest(baseline_path), canonical_modified=False)
        receipt['navigation_gaps'] = [p['navigation_gap'] for p in placements if 'navigation_gap' in p]
        receipt['entrance_gaps'] = sorted({p['entrance_gap'] for p in placements if 'entrance_gap' in p})
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
                receipt['donors'], receipt['source_difficulties'] = prepare_templates(prepared, manifest['entries'], args.cache, tables | binding_tables)
                write_placements(prepared, manifest, placements, bindings)
                for proof in native_bindings:
                    write_primary_binding(prepared, tuple(proof['binding']))
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
            status = 'PREPARED_NOT_PUBLISHED'
            if receipt['entrance_gaps']:
                status = 'PREPARED_WITH_ENTRANCE_GAPS'
            elif receipt['navigation_gaps']:
                status = 'PREPARED_WITH_NAVIGATION_GAPS'
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
