#!/usr/bin/env python3
"""Import resolved, non-summoned log observations into a world database COPY."""
import argparse
import json
import math
import sqlite3
import sys
from contextlib import closing
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import retail_content
from log_population_census import minimum_assignment

POLICY = 'LOG_DERIVED_POPULATION'


def validate_copy(path):
    path = Path(path)
    if path.name == 'world.db' or path.resolve().name == 'world.db':
        raise ValueError('refusing canonical world.db; use a copy')
    if not path.is_file() or path.is_symlink() or path.stat().st_nlink != 1:
        raise ValueError('destination must be an existing, unlinked database copy')
    return path


def template_row_keys(db, npc_id):
    tables = (('content_creature_template', 'entry', ('entry',)),
              ('content_creature_template_difficulty', 'entry', ('entry', 'DifficultyID')),
              ('content_creature_template_model', 'CreatureID', ('CreatureID', 'Idx')),
              ('content_creature_template_addon', 'entry', ('entry',)))
    return {(table, json.dumps(row, separators=(',', ':')))
            for table, key, columns in tables
            for row in db.execute(f"SELECT {','.join(columns)} FROM {table} WHERE {key}=?", (npc_id,))}


def prepare_tdb_template(db, npc_id):
    if not db.execute('SELECT 1 FROM tdb_creature_template WHERE entry=?', (npc_id,)).fetchone():
        return 'TEMPLATE_ABSENT_FROM_TDB_AND_CONTENT'
    if not db.execute('SELECT 1 FROM tdb_creature_template_difficulty WHERE Entry=? AND DifficultyID=0', (npc_id,)).fetchone():
        return 'TDB_BASE_DIFFICULTY_UNAVAILABLE'
    previous_rows = template_row_keys(db, npc_id)
    db.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    db.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=? AND DifficultyID=0', (npc_id,))
    db.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    retail_content.insert_tdb_templates(db, 'SELECT Entry FROM temp.tdb_difficulty', replace_legacy=True)
    if not db.execute('SELECT 1 FROM content_creature_template WHERE entry=?', (npc_id,)).fetchone():
        return 'TDB_TEMPLATE_PROJECTION_FAILED'
    db.execute('''INSERT OR IGNORE INTO content_creature_template_difficulty
        (entry,DifficultyID,minlevel,maxlevel,exp,ContentTuningID,HealthScalingExpansion,
         LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,ArmorModifier,
         DamageModifier,lootid,mingold,maxgold)
        SELECT d.Entry,d.DifficultyID,COALESCE(l.minlevel,1),COALESCE(l.maxlevel,1),
         health_expansion(d.HealthScalingExpansion),d.ContentTuningID,d.HealthScalingExpansion,
         d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,d.HealthModifier,d.ManaModifier,d.ArmorModifier,
         d.DamageModifier,d.LootID,d.GoldMin,d.GoldMax
        FROM tdb_creature_template_difficulty d LEFT JOIN content_tuning_level l ON l.ID=d.ContentTuningID
        WHERE d.Entry=? AND d.DifficultyID>0''', (npc_id,))
    db.execute('INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=?', (npc_id,))
    db.execute(f'INSERT OR IGNORE INTO content_creature_template_addon SELECT entry,{retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon WHERE entry=?', (npc_id,))
    db.execute('INSERT INTO log_derived_templates VALUES (?,?)', (npc_id, POLICY))
    added_rows = template_row_keys(db, npc_id) - previous_rows
    db.executemany('INSERT INTO log_derived_template_rows VALUES (?,?,?,?)',
                   [(npc_id, table, key, POLICY) for table, key in sorted(added_rows)])
    return None


def load_existing_positions(db, map_id, npc_id):
    populations = []
    for table, column in (('tdb_creature', 'id'), ('content_creature', 'id1')):
        rows = db.execute(f'SELECT position_x,position_y,position_z FROM {table} WHERE map=? AND {column}=?',
                          (map_id, npc_id))
        populations.append(Counter(tuple(row) for row in rows))
    counts = populations[0] | populations[1]
    return [position for position, count in sorted(counts.items()) for _ in range(count)]


def eligible_position(record):
    ambiguous = record.get('identity_ambiguous')
    spread = record.get('candidate_spread_yards')
    centroid_ok = (ambiguous is True and record.get('position_method') == 'candidate_centroid'
                   and isinstance(spread, (int, float)) and not isinstance(spread, bool)
                   and math.isfinite(spread) and 0 <= spread <= 5)
    if record.get('population_kind') != 'likely_spawn' or record.get('summoned') is not False:
        return None, 'SUMMON_OR_UNCLASSIFIED'
    if not (ambiguous is False or centroid_ok):
        return None, 'AMBIGUOUS_SPAWN_IDENTITY'
    values = [record.get(key) for key in ('x', 'y', 'z', 'facing')]
    if any(not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value) for value in values):
        return None, 'POSITION_OR_FACING_UNAVAILABLE'
    return values, None


def match_existing(db, records, radius):
    groups = defaultdict(list)
    keys = set()
    for record in records:
        source_key = record['source_key']
        if source_key in keys:
            raise ValueError(f'duplicate source key {source_key}')
        keys.add(source_key)
        position, _ = eligible_position(record)
        if position is not None:
            groups[(int(record['map']), int(record['npc_id']))].append((source_key, position))
    matched = set()
    existing = {}
    for (map_id, npc_id), entries in groups.items():
        positions = load_existing_positions(db, map_id, npc_id)
        existing[(map_id, npc_id)] = positions
        if not positions:
            continue
        # A dummy costs more than every possible sum of real distances, so match
        # count takes precedence over total distance.
        penalty = (radius + 1) * (len(entries) + 1)
        costs = []
        for _, values in entries:
            distances = [math.dist(values[:3], position) for position in positions]
            costs.append([distance if distance <= radius else math.inf for distance in distances]
                         + [penalty] * len(entries))
        assignment, _ = minimum_assignment(costs)
        for (source_key, _), column in zip(entries, assignment):
            if column < len(positions):
                matched.add(source_key)
    return matched, existing


def affix_summons(db):
    return {row[0] for row in db.execute('''
        SELECT DISTINCT effect.EffectMiscValue_0
        FROM keystone_affix AS affix
        JOIN spell_name AS spell ON spell.Name_lang = affix.Name_lang
        JOIN spell_effect AS effect ON effect.SpellID = spell.ID
        WHERE effect.Effect = 28 AND effect.EffectMiscValue_0 > 0
    ''')}


def retire_affix_spawns(db, summons, maps, report):
    rows = db.execute('''SELECT p.guid,p.map,p.npc_id,p.source_key
        FROM log_derived_population AS p
        JOIN content_creature AS c ON c.guid=p.guid
        WHERE p.policy=? AND c.Comment=?''', (POLICY, POLICY)).fetchall()
    for guid, map_id, npc_id, source_key in rows:
        if map_id not in maps or npc_id not in summons:
            continue
        db.execute('DELETE FROM content_creature WHERE guid=?', (guid,))
        db.execute('DELETE FROM log_derived_population WHERE guid=?', (guid,))
        report[map_id]['gaps'].append({'source_key': source_key, 'reason': 'AFFIX_SUMMON_RETIRED'})


def import_population(destination, records, radius=10.0):
    destination = validate_copy(destination)
    report = defaultdict(lambda: {'imported': 0, 'already_present': 0, 'gaps': [], 'templates_from_tdb': set(), 'templates_absent': set()})
    with closing(sqlite3.connect(destination)) as db, db:
        db.execute('BEGIN IMMEDIATE')
        db.execute('''CREATE TABLE IF NOT EXISTS log_derived_population (
            guid INTEGER PRIMARY KEY, source_key TEXT NOT NULL UNIQUE, map INTEGER NOT NULL,
            npc_id INTEGER NOT NULL, z_method TEXT NOT NULL, observed_event TEXT NOT NULL,
            observed_time TEXT NOT NULL, observed_level INTEGER, policy TEXT NOT NULL,
            observation_json TEXT NOT NULL)''')
        db.execute('CREATE TABLE IF NOT EXISTS log_derived_templates (entry INTEGER PRIMARY KEY, policy TEXT NOT NULL)')
        db.execute('''CREATE TABLE IF NOT EXISTS log_derived_template_rows (
            entry INTEGER NOT NULL, table_name TEXT NOT NULL, row_key TEXT NOT NULL, policy TEXT NOT NULL,
            PRIMARY KEY (table_name,row_key))''')
        summons = affix_summons(db)
        retire_affix_spawns(db, summons, {int(record['map']) for record in records}, report)
        next_guid = db.execute('SELECT COALESCE(MAX(guid), 0) + 1 FROM content_creature').fetchone()[0]
        matched, existing_positions = match_existing(db, records, radius)
        for record in records:
            map_id, npc_id = int(record['map']), int(record['npc_id'])
            result = report[map_id]
            source_key = record['source_key']
            if not source_key or not record.get('z_method') or not record.get('observation_event') or not record.get('observation_time'):
                raise ValueError(f'incomplete provenance for {source_key!r}')
            if npc_id in summons:
                result['gaps'].append({'source_key': source_key, 'reason': 'AFFIX_SUMMON'})
                continue
            existing = db.execute('SELECT guid FROM log_derived_population WHERE source_key=?', (source_key,)).fetchone()
            if existing:
                result['already_present'] += 1
                continue
            values, reason = eligible_position(record)
            if values is None:
                result['gaps'].append({'source_key': source_key, 'reason': reason})
                continue
            x, y, z, facing = values
            if source_key in matched:
                result['already_present'] += 1
                continue
            if any(math.dist((x, y, z), position) <= 3.0 for position in existing_positions.get((map_id, npc_id), ())):
                result['gaps'].append({'source_key': source_key, 'reason': 'AUTHORED_SPAWN_NEARBY_AMBIGUOUS'})
                continue
            if not db.execute('SELECT 1 FROM content_creature_template WHERE entry=?', (npc_id,)).fetchone():
                gap = prepare_tdb_template(db, npc_id)
                if gap:
                    if gap == 'TEMPLATE_ABSENT_FROM_TDB_AND_CONTENT':
                        result['templates_absent'].add(npc_id)
                    result['gaps'].append({'source_key': source_key, 'reason': gap})
                    continue
                result['templates_from_tdb'].add(npc_id)
            elif db.execute('SELECT 1 FROM log_derived_templates WHERE entry=?', (npc_id,)).fetchone():
                result['templates_from_tdb'].add(npc_id)
            db.execute('''INSERT INTO content_creature
                (guid,id1,map,position_x,position_y,position_z,orientation,spawnDifficulties,Comment)
                VALUES (?,?,?,?,?,?,?,?,?)''', (next_guid, npc_id, map_id, x, y, z, facing, '1', POLICY))
            db.execute('INSERT INTO log_derived_population VALUES (?,?,?,?,?,?,?,?,?,?)',
                       (next_guid, source_key, map_id, npc_id, record['z_method'], record['observation_event'],
                        record['observation_time'], record.get('level'), POLICY,
                        json.dumps(record, sort_keys=True, separators=(',', ':'))))
            result['imported'] += 1
            result['gaps'].append({'source_key': source_key, 'reason': 'PATROL_UNOBSERVED'})
            next_guid += 1
    for value in report.values():
        value['templates_from_tdb'] = sorted(value['templates_from_tdb'])
        value['templates_absent'] = sorted(value['templates_absent'])
    return {str(map_id): value for map_id, value in sorted(report.items())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path, help='existing world database copy, never world.db')
    parser.add_argument('--spawns', required=True, type=Path, help='resolved observations JSON array')
    parser.add_argument('--report', required=True, type=Path)
    args = parser.parse_args()
    with args.spawns.open(encoding='utf-8') as source:
        records = json.load(source)
    report = import_population(args.db, records)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
