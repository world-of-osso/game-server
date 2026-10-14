#!/usr/bin/env python3
"""Prepare authentic Retail AdventureJournal/Perks SQL; never write a world database.

The supplied existing game-engine decoder validates pinned local WDC5 layouts.
Apply generated SQL only to an explicitly owned copy after reviewing the report.
"""
import argparse
from contextlib import closing
import importlib.util
import json
from pathlib import Path
import sqlite3
import struct
import sys
TABLES = {
    'AdventureJournal': ('adventure_journal', 1115949, 0x6DC6E4E8),
    'PerksActivity': ('perks_activity', 4519766, 0x5361589F),
    'PerksActivityXInterval': ('perks_activity_x_interval', 4519781, 0x2E6128D2),
    'PerksActivityCondition': ('perks_activity_condition', 5524226, 0x2E9DF173),
    'PerksActivityTag': ('perks_activity_tag', 4694317, 0xEBA8ED0E),
    'PerksActivityXTag': ('perks_activity_x_tag', 4694318, 0xFFB58E04),
    'PerksActivityThreshold': ('perks_activity_threshold', 4519767, 0x7995A0C8),
    'PerksActivityThresholdGroup': ('perks_activity_threshold_group', 4519768, 0x377370DC),
}


def load_decoder(path):
    sys.path.insert(0, str(path.parent))
    spec = importlib.util.spec_from_file_location('journal_decoder', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.TABLES.update({name: (fdid, layout) for name, (_, fdid, layout) in TABLES.items()})
    return module


def quote_sql(value):
    return "'" + value.replace("'", "''") + "'"


def decode_sources(decoder, db2, definitions):
    sources = {}
    for name, (table, fdid, layout) in TABLES.items():
        binary = db2 / f'{fdid}.db2'
        columns, rows, dropped = decoder.read_table(name, binary, definitions / f'{name}.dbd')
        layout = struct.unpack_from('<I', binary.read_bytes(), 156)[0]
        fields = [field for field, _ in columns]
        sources[table] = {
            'name': name, 'fdid': fdid, 'layout': f'{layout:08X}',
            'unreadable': dropped, 'records': [dict(zip(fields, row)) for row in rows],
        }
    return sources


def prepare_sql(sources, world, build):
    with closing(sqlite3.connect(f'file:{world.resolve()}?mode=ro', uri=True)) as connection:
        tree_ids = {row[0] for row in connection.execute('SELECT ID FROM criteria_tree')}
        condition_ids = {row[0] for row in connection.execute('SELECT ID FROM player_condition')}
    report = {'build': build, 'sources': {}, 'missing_activity_trees': [], 'missing_player_conditions': []}
    sql = ['BEGIN TRANSACTION;', 'CREATE TABLE IF NOT EXISTS adventure_guide_source (name TEXT PRIMARY KEY,build TEXT,fdid INTEGER,layout TEXT,readable INTEGER,unreadable INTEGER);', 'DELETE FROM adventure_guide_source;']
    for table, source in sources.items():
        records = source['records']
        report['sources'][source['name']] = {key: source[key] for key in ('fdid', 'layout', 'unreadable')}
        report['sources'][source['name']]['readable'] = len(records)
        sql.extend([f'DROP TABLE IF EXISTS {table};', f'CREATE TABLE {table} (ID INTEGER PRIMARY KEY,payload TEXT NOT NULL);'])
        for row in records:
            payload = quote_sql(json.dumps(row, ensure_ascii=False, separators=(',', ':')))
            sql.append(f'INSERT INTO {table} VALUES({row["ID"]},{payload});')
        values = ','.join((quote_sql(source['name']), quote_sql(build), str(source['fdid']), quote_sql(source['layout']), str(len(records)), str(source['unreadable'])))
        sql.append(f'INSERT INTO adventure_guide_source VALUES({values});')
    report['missing_activity_trees'] = [row['ID'] for row in sources['perks_activity']['records'] if row['CriteriaTreeID'] not in tree_ids]
    referenced = {row['PlayerConditionID'] for row in sources['adventure_journal']['records']}
    referenced.update(row['PlayerConditionID'] for row in sources['perks_activity_condition']['records'])
    report['missing_player_conditions'] = sorted(referenced - condition_ids - {0})
    report['month_assignment'] = 'PerksActivityXInterval has no verified year/month field; approved all-source-row policy applies.'
    report['reward_binding'] = 'Threshold groups retain PerksMonth but no verified calendar epoch; no reward/currency grant invented.'
    sql.append('COMMIT;')
    return '\n'.join(sql) + '\n', report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('decoder', 'db2', 'definitions', 'world', 'output-sql', 'report'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    decoder = load_decoder(args.decoder)
    sources = decode_sources(decoder, args.db2, args.definitions)
    sql, report = prepare_sql(sources, args.world, decoder.source_identity(args.db2)['build'])
    args.output_sql.parent.mkdir(parents=True, exist_ok=True)
    args.output_sql.write_text(sql)
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({name: entry['readable'] for name, entry in report['sources'].items()}))


if __name__ == '__main__':
    main()
