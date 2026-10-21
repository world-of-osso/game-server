#!/usr/bin/env python3
"""Local TDB/70448 conversation source ingress; canonical publication is lead-owned."""
import argparse
from contextlib import closing
import csv
import hashlib
import json
from pathlib import Path
import sqlite3

from mysql_dump import dump_rows

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / 'data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql'
BUILD = '12.1.5.70448'
DEFAULT_CSV = ROOT / 'data/db2' / BUILD
TDB_COLUMNS = {
    'conversation_template': ('Id', 'FirstLineId', 'TextureKitId', 'Flags', 'ScriptName', 'VerifiedBuild'),
    'conversation_actors': ('ConversationId', 'ConversationActorId', 'ConversationActorGuid', 'Idx',
                            'CreatureId', 'CreatureDisplayInfoId', 'NoActorObject', 'ActivePlayerObject', 'VerifiedBuild'),
    'conversation_line_template': ('Id', 'UiCameraID', 'ActorIdx', 'Flags', 'ChatType', 'VerifiedBuild'),
}
TDB_FINGERPRINTS = {
    'conversation_template': (248, 'f942bd0e54b64fedd5fa441584be6870dc42d85ac3906d330f5d152dafb312c7'),
    'conversation_actors': (424, '54b2d6e31254c2bd737a4d75f88830ef9683af357f1900ac389200a59ff51b89'),
    'conversation_line_template': (702, 'ce0c7806f584c9623818088907eb6c2aa06431fead9eb0dc8fd3f568b608c064'),
}
CSV_TABLES = {
    'ConversationLine': ('conversation_line', ('ID', 'BroadcastTextID', 'Field_10_2_0_51239_001',
        'SpellVisualKitID', 'AdditionalDuration', 'NextConversationLineID', 'AnimKitID',
        'SpeechType', 'StartAnimation', 'EndAnimation')),
    'BroadcastTextDuration': ('broadcast_text_duration', ('ID', 'Locale', 'DurationMS', 'BroadcastTextID')),
}


def fingerprint(rows):
    encoded = json.dumps(rows, separators=(',', ':')).encode()
    return hashlib.sha256(encoded).hexdigest()


def read_tdb_tables(source):
    tables = {name: (columns, []) for name, columns in TDB_COLUMNS.items()}
    with source.open(encoding='utf-8') as stream:
        for line in stream:
            if not line.startswith('INSERT INTO `conversation_'):
                continue
            name = line.split('`', 2)[1]
            if name not in tables:
                continue
            for columns, row in dump_rows(line, name, source):
                if columns is not None and tuple(columns) != TDB_COLUMNS[name]:
                    raise ValueError(f'{name}: source INSERT columns changed')
                tables[name][1].append(row)
    for name, (_, rows) in tables.items():
        count, expected = TDB_FINGERPRINTS[name]
        if len(rows) != count or fingerprint(rows) != expected:
            raise ValueError(f'{name}: differs from pinned TDB1210 source rows')
    return {'tdb_' + name: table for name, table in tables.items()}


def read_csv_tables(csv_dir):
    receipt = json.loads((csv_dir / 'receipt.json').read_text())
    if receipt['identity']['build'] != BUILD:
        raise ValueError(f'conversation data requires local build {BUILD}')
    tables = {}
    for name, (table, columns) in CSV_TABLES.items():
        path = csv_dir / f'{name}.csv'
        source_receipt = receipt['tables'][name]
        if hashlib.sha256(path.read_bytes()).hexdigest() != source_receipt['csv_sha256']:
            raise ValueError(f'{name}: CSV differs from local extraction receipt')
        with path.open(newline='', encoding='utf-8') as stream:
            reader = csv.reader(stream)
            if tuple(next(reader)) != columns:
                raise ValueError(f'{name}: source CSV columns changed')
            rows = [tuple(int(value) for value in row) for row in reader]
        if len(rows) != source_receipt['rows']:
            raise ValueError(f'{name}: source CSV row count differs from receipt')
        # Preserve partial-authentication/unreadable-record facts in reports; do not
        # turn an available row into a claim that encrypted table sections were read.
        tables[table] = (columns, rows)
    return tables


def read_source_tables(source=DEFAULT_SOURCE, csv_dir=DEFAULT_CSV):
    return {**read_tdb_tables(source), **read_csv_tables(csv_dir)}


def table_ddl(table, columns):
    primary = ('ConversationId', 'Idx') if table == 'tdb_conversation_actors' else (columns[0],)
    declarations = [f'"{name}" ' + ('TEXT' if name == 'ScriptName' else 'INTEGER') + ' NOT NULL'
                    for name in columns]
    declarations.append('PRIMARY KEY (' + ','.join(f'"{name}"' for name in primary) + ')')
    return f'CREATE TABLE IF NOT EXISTS "{table}" ({",".join(declarations)})'


def import_tables(connection, tables):
    with connection:
        if not connection.in_transaction:
            connection.execute('BEGIN IMMEDIATE')
        for table, (columns, rows) in tables.items():
            connection.execute(table_ddl(table, columns))
            connection.execute(f'DELETE FROM "{table}"')
            names = ','.join(f'"{name}"' for name in columns)
            placeholders = ','.join('?' for _ in columns)
            connection.executemany(f'INSERT INTO "{table}" ({names}) VALUES ({placeholders})', rows)
            actual = connection.execute(f'SELECT {names} FROM "{table}"').fetchall()
            if sorted(actual) != sorted(rows):
                raise ValueError(f'{table}: imported rows differ from source')
        connection.execute('CREATE TABLE IF NOT EXISTS db2_build (table_name TEXT PRIMARY KEY, build TEXT NOT NULL)')
        for name in CSV_TABLES:
            connection.execute('INSERT INTO db2_build VALUES (?,?) ON CONFLICT(table_name) DO UPDATE SET build=excluded.build',
                               (name, BUILD))
    return {name: len(rows) for name, (_, rows) in tables.items()}


def write_sql(path, tables):
    with closing(sqlite3.connect(':memory:')) as encoder, path.open('w', encoding='utf-8') as output:
        output.write('BEGIN IMMEDIATE;\n')
        for table, (columns, rows) in tables.items():
            output.write(table_ddl(table, columns) + ';\n')
            output.write(f'DELETE FROM "{table}";\n')
            quoted_columns = ','.join(f'"{name}"' for name in columns)
            quote = 'SELECT ' + ','.join('quote(?)' for _ in columns)
            for row in rows:
                values = ','.join(encoder.execute(quote, row).fetchone())
                output.write(f'INSERT INTO "{table}" ({quoted_columns}) VALUES ({values});\n')
        output.write('CREATE TABLE IF NOT EXISTS db2_build (table_name TEXT PRIMARY KEY, build TEXT NOT NULL);\n')
        for name in CSV_TABLES:
            output.write(f"INSERT INTO db2_build VALUES ('{name}','{BUILD}') ON CONFLICT(table_name) DO UPDATE SET build=excluded.build;\n")
        output.write('COMMIT;\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=DEFAULT_SOURCE)
    parser.add_argument('--csv-dir', type=Path, default=DEFAULT_CSV)
    parser.add_argument('--db', type=Path, required=True, help='existing owned SQLite copy')
    parser.add_argument('--sql', type=Path, help='reviewed SQL publication artifact')
    args = parser.parse_args()
    if args.db.resolve() == (ROOT / 'data/world.db').resolve():
        parser.error('canonical world.db publication is lead-owned; use an owned copy')
    tables = read_source_tables(args.source, args.csv_dir)
    with closing(sqlite3.connect(args.db.resolve().as_uri() + '?mode=rw', uri=True)) as connection:
        counts = import_tables(connection, tables)
    if args.sql is not None:
        write_sql(args.sql, tables)
    receipts = json.loads((args.csv_dir / 'receipt.json').read_text())
    report = {'source': str(args.source.resolve()), 'build': BUILD, 'tables': counts,
              'fingerprints': {name: fingerprint(rows) for name, (_, rows) in tables.items()},
              'csv_receipts': {name: receipts['tables'][name] for name in CSV_TABLES}}
    print(json.dumps(report, sort_keys=True))


if __name__ == '__main__':
    main()
