#!/usr/bin/env python3
"""Import resolved, non-summoned log observations into a world database COPY."""
import argparse
import json
import math
import sqlite3
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import retail_content

POLICY = 'LOG_DERIVED_POPULATION'


def validate_copy(path):
    path = Path(path)
    if path.name == 'world.db' or path.resolve().name == 'world.db':
        raise ValueError('refusing canonical world.db; use a copy')
    if not path.is_file() or path.is_symlink() or path.stat().st_nlink != 1:
        raise ValueError('destination must be an existing, unlinked database copy')
    return path


def prepare_tdb_template(db, npc_id):
    if not db.execute('SELECT 1 FROM tdb_creature_template WHERE entry=?', (npc_id,)).fetchone():
        return 'TEMPLATE_ABSENT_FROM_TDB_AND_CONTENT'
    if not db.execute('SELECT 1 FROM tdb_creature_template_difficulty WHERE Entry=? AND DifficultyID=0', (npc_id,)).fetchone():
        return 'TDB_BASE_DIFFICULTY_UNAVAILABLE'
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
    return None


def import_population(destination, records, radius=10.0):
    destination = validate_copy(destination)
    report = defaultdict(lambda: {'imported': 0, 'already_present': 0, 'gaps': [], 'templates_from_tdb': set(), 'templates_absent': set()})
    with sqlite3.connect(destination) as db:
        db.execute('BEGIN IMMEDIATE')
        db.execute('''CREATE TABLE IF NOT EXISTS log_derived_population (
            guid INTEGER PRIMARY KEY, source_key TEXT NOT NULL UNIQUE, map INTEGER NOT NULL,
            npc_id INTEGER NOT NULL, z_method TEXT NOT NULL, observed_event TEXT NOT NULL,
            observed_time TEXT NOT NULL, observed_level INTEGER, policy TEXT NOT NULL)''')
        db.execute('CREATE TABLE IF NOT EXISTS log_derived_templates (entry INTEGER PRIMARY KEY, policy TEXT NOT NULL)')
        next_guid = db.execute('SELECT COALESCE(MAX(guid), 0) + 1 FROM content_creature').fetchone()[0]
        for record in records:
            map_id, npc_id = int(record['map']), int(record['npc_id'])
            result = report[map_id]
            source_key = record['source_key']
            if not source_key or not record.get('z_method') or not record.get('observation_event') or not record.get('observation_time'):
                raise ValueError(f'incomplete provenance for {source_key!r}')
            existing = db.execute('SELECT guid FROM log_derived_population WHERE source_key=?', (source_key,)).fetchone()
            if existing:
                result['already_present'] += 1
                continue
            if record.get('population_kind') != 'likely_spawn' or record.get('summoned') is not False or record.get('identity_ambiguous') is not False:
                result['gaps'].append({'source_key': source_key, 'reason': 'SUMMON_OR_AMBIGUOUS_IDENTITY'})
                continue
            values = [record.get(key) for key in ('x', 'y', 'z', 'facing')]
            if any(not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value) for value in values):
                result['gaps'].append({'source_key': source_key, 'reason': 'POSITION_OR_FACING_UNAVAILABLE'})
                continue
            x, y, z, facing = values
            present = False
            for table, column in (('tdb_creature', 'id'), ('content_creature', 'id1')):
                rows = db.execute(f'SELECT position_x, position_y, position_z FROM {table} WHERE map=? AND {column}=?',
                                  (map_id, npc_id))
                if any((px-x)**2 + (py-y)**2 + (pz-z)**2 <= radius**2 for px, py, pz in rows):
                    present = True
                    break
            if present:
                result['already_present'] += 1
                continue
            if not db.execute('SELECT 1 FROM content_creature_template WHERE entry=?', (npc_id,)).fetchone():
                gap = prepare_tdb_template(db, npc_id)
                if gap:
                    if gap == 'TEMPLATE_ABSENT_FROM_TDB_AND_CONTENT':
                        result['templates_absent'].add(npc_id)
                    result['gaps'].append({'source_key': source_key, 'reason': gap})
                    continue
                result['templates_from_tdb'].add(npc_id)
            db.execute('''INSERT INTO content_creature
                (guid,id1,map,position_x,position_y,position_z,orientation,spawnDifficulties,Comment)
                VALUES (?,?,?,?,?,?,?,?,?)''', (next_guid, npc_id, map_id, x, y, z, facing, '1', POLICY))
            db.execute('INSERT INTO log_derived_population VALUES (?,?,?,?,?,?,?,?,?)',
                       (next_guid, source_key, map_id, npc_id, record['z_method'], record['observation_event'],
                        record['observation_time'], record.get('level'), POLICY))
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
