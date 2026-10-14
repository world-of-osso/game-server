#!/usr/bin/env python3
"""Lead-owned import of verbatim TDB battleground metadata; no spawn/content rebuild.

Reads local TDB1210 only. Writes an existing database, transactionally; unrelated
source/content tables stay unchanged. Missing source tables are reported, not invented.
"""
import argparse
import json
import re
import sqlite3
from contextlib import closing
from pathlib import Path

from mysql_dump import INSERT_RE, parse_values

TABLES = frozenset({
    'battleground_template', 'battleground_scripts', 'spawn_group', 'spawn_group_template',
    'world_state', 'areatrigger', 'areatrigger_template', 'areatrigger_create_properties',
    'areatrigger_create_properties_orbit',
    'areatrigger_template_actions', 'areatrigger_scripts', 'areatrigger_teleport',
    'npc_spellclick_spells', 'transports', 'destructible_hitpoint',
})
COLUMN = re.compile(r'^\s+`(\w+)`\s+(\w+)')


def sqlite_type(mysql_type):
    if 'int' in mysql_type:
        return 'INTEGER'
    if mysql_type in {'float', 'double', 'decimal'}:
        return 'REAL'
    if mysql_type in {'varchar', 'char', 'text', 'mediumtext', 'longtext'}:
        return 'TEXT'
    raise ValueError(f'unsupported TDB column type {mysql_type}')


def read_catalogs(source, tables=TABLES):
    catalogs = {}
    active = None
    with source.open(encoding='utf-8') as stream:
        for line in stream:
            match = re.match(r'^CREATE TABLE `(\w+)`', line)
            if match:
                active = match[1] if match[1] in tables else None
                if active:
                    catalogs[active] = {'columns': [], 'keys': [], 'rows': []}
            if active:
                column = COLUMN.match(line)
                if column:
                    catalogs[active]['columns'].append((column[1], sqlite_type(column[2])))
                if 'PRIMARY KEY (' in line:
                    catalogs[active]['keys'] = re.findall(r'`(\w+)`', line)
                if line.startswith(') ENGINE'):
                    active = None
            insert = INSERT_RE.match(line)
            if insert and insert[1] in tables:
                table = insert[1]
                catalog = catalogs[table]
                columns = [name for name, _ in catalog['columns']]
                supplied = re.findall(r'`(\w+)`', insert[2]) if insert[2] else columns
                if supplied != columns:
                    raise ValueError(f'{table}: INSERT column order differs from DDL')
                for row in parse_values(line, insert.end(), source):
                    if len(row) != len(columns):
                        raise ValueError(f'{table}: row has {len(row)} values for {len(columns)} columns')
                    catalog['rows'].append(row)
    if not catalogs:
        raise ValueError(f'{source}: no battleground metadata tables')
    return catalogs


def write_catalog(conn, table, catalog):
    target = 'tdb_' + table
    columns = catalog['columns']
    definitions = [f'"{name}" {kind}' for name, kind in columns]
    if catalog['keys']:
        definitions.append('PRIMARY KEY (' + ','.join(f'"{name}"' for name in catalog['keys']) + ')')
    conn.execute(f'CREATE TABLE IF NOT EXISTS {target} ({",".join(definitions)})')
    actual = [row[1] for row in conn.execute(f'PRAGMA table_info({target})')]
    if actual != [name for name, _ in columns]:
        raise ValueError(f'{target}: existing schema differs from TDB source')
    conn.execute(f'DELETE FROM {target}')
    placeholders = ','.join('?' for _ in columns)
    conn.executemany(f'INSERT INTO {target} VALUES ({placeholders})', catalog['rows'])


def import_catalogs(source, database, tables=TABLES):
    catalogs = read_catalogs(source, tables)
    with closing(sqlite3.connect(database.resolve().as_uri() + '?mode=rw', uri=True)) as conn:
        with conn:
            # Explicit BEGIN includes DDL in the rollback boundary.
            conn.execute('BEGIN')
            for table, catalog in sorted(catalogs.items()):
                write_catalog(conn, table, catalog)
    return {'source': str(source), 'rows': {table: len(data['rows']) for table, data in catalogs.items()},
            'missing_tables': sorted(tables - catalogs.keys())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dump', type=Path, required=True)
    parser.add_argument('--db', type=Path, required=True)
    parser.add_argument('--table', action='append', choices=sorted(TABLES),
                        help='Publish only named source tables; repeat for multiple tables')
    args = parser.parse_args()
    tables = frozenset(args.table) if args.table else TABLES
    print(json.dumps(import_catalogs(args.dump, args.db, tables), sort_keys=True))


if __name__ == '__main__':
    main()
