#!/usr/bin/env python3
"""Prepare authenticated TDB encounter server-side spells in independent copies.

Adarogg Inferno Charge 119297/119298 and Slagmaw's Magnaw teleports
119424/119425/119426/119428 (boss_adarogg.cpp/boss_slagmaw.cpp "Serverside").

Retain all source columns in tdb_serverside_spell/tdb_serverside_spell_effect;
no DB2 rows, substitute spells or derived effect IDs. The server's spell store
loads these tables (spell_info/load/serverside.rs). The telash-jumps mode
copies the complete server jump_charge_params table, preserving nullable options.
"""
import argparse
import re
import sqlite3
from contextlib import closing
from pathlib import Path

import mysql_dump

SPELLS = {119297, 119298, 119424, 119425, 119426, 119428}
ENCOUNTER_SPELLS = {
    'ragefire': SPELLS,
    'gargolmar': {30642},
    'omor': {35642, 32185},
    'herald': {18430, 20538, 30693, 30694, 30696, 30698, 30699},
}
TABLE_KEYS = {
    'serverside_spell': ('Id', 'DifficultyID'),
    'serverside_spell_effect': ('SpellID', 'EffectIndex', 'DifficultyID'),
}
JUMP_TABLE_KEYS = {'jump_charge_params': ('id',)}
CANONICAL = Path('/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db')


def read_source_rows(dump, spells=SPELLS, table_keys=TABLE_KEYS):
    schemas = {}
    selected = {table: {} for table in table_keys}
    active = None
    with dump.open(encoding='utf-8') as source:
        for statement in source:
            if statement.startswith('CREATE TABLE `'):
                table = statement.split('`')[1]
                active = table if table in table_keys else None
                if active:
                    schemas[active] = []
            elif active and statement.startswith('  `'):
                schemas[active].append(statement.split('`')[1])
            elif active and statement.startswith(')'):
                active = None
            elif statement.startswith('INSERT INTO `'):
                table = statement.split('`')[1]
                if table in table_keys:
                    select_statement_rows(statement, table, schemas, selected, dump, spells, table_keys)
    validate_source_rows(schemas, selected, dump, spells, table_keys)
    return schemas, selected


def select_statement_rows(statement, table, schemas, selected, dump, spells, table_keys):
    columns = schemas.get(table)
    if not columns:
        raise ValueError(f'{dump}: missing source schema for {table}')
    for explicit_columns, row in mysql_dump.dump_rows(statement, table, dump):
        if explicit_columns is not None and explicit_columns != columns:
            raise ValueError(f'{dump}: unexpected column order for {table}')
        if len(row) != len(columns):
            raise ValueError(f'{dump}: incompatible {table} row with {len(row)} fields')
        record = dict(zip(columns, row))
        keys = table_keys[table]
        if spells is not None and record[keys[0]] not in spells:
            continue
        identity = tuple(record[key] for key in keys)
        if identity in selected[table]:
            raise ValueError(f'{dump}: duplicate source {table} identity {identity}')
        selected[table][identity] = row


def validate_source_rows(schemas, selected, dump, spells, table_keys):
    for table, keys in table_keys.items():
        columns = schemas.get(table, [])
        if not columns or any(not re.fullmatch(r'[A-Za-z0-9_]+', column) for column in columns):
            raise ValueError(f'{dump}: invalid/missing source schema for {table}')
        if any(key not in columns for key in keys):
            raise ValueError(f'{dump}: missing identity columns for {table}')
        found = {identity[0] for identity in selected[table]}
        required = {713, 714} if table == 'jump_charge_params' else spells
        if missing := required - found:
            raise ValueError(f'{dump}: missing authored {table} spells {sorted(missing)}')


def write_source_tables(connection, schemas, selected, table_keys=TABLE_KEYS):
    for table, keys in table_keys.items():
        columns = schemas[table]
        quoted_columns = ','.join(f'"{column}"' for column in columns)
        quoted_keys = ','.join(f'"{key}"' for key in keys)
        # Untyped SQLite columns retain the MySQL token's actual value type.
        # No guessed DB2 metadata or synthetic record IDs are introduced.
        connection.execute(f'CREATE TABLE IF NOT EXISTS tdb_{table} ({quoted_columns}, PRIMARY KEY ({quoted_keys}))')
        placeholders = ','.join('?' for _ in columns)
        updates = ','.join(f'"{column}"=excluded."{column}"' for column in columns if column not in keys)
        connection.executemany(
            f'INSERT INTO tdb_{table} ({quoted_columns}) VALUES ({placeholders}) '
            f'ON CONFLICT ({quoted_keys}) DO UPDATE SET {updates}', selected[table].values(),
        )


def import_spells(db, dump, encounter='ragefire'):
    db, dump = Path(db), Path(dump)
    if db.name == 'world.db' or db.resolve() == CANONICAL.resolve():
        raise ValueError('use an independent database copy, not canonical world.db')
    if db.is_symlink() or not db.is_file() or db.stat().st_nlink != 1:
        raise ValueError('copy must be an existing independent regular file')
    jump_params = encounter == 'telash-jumps'
    table_keys = JUMP_TABLE_KEYS if jump_params else TABLE_KEYS
    spells = None if jump_params else ENCOUNTER_SPELLS[encounter]
    schemas, selected = read_source_rows(dump, spells, table_keys)
    with closing(sqlite3.connect(db)) as connection, connection:
        connection.execute('BEGIN')
        write_source_tables(connection, schemas, selected, table_keys)
    return {table: len(rows) for table, rows in selected.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--encounter', choices=[*ENCOUNTER_SPELLS, 'telash-jumps'], default='ragefire')
    parser.add_argument('--db', type=Path, required=True)
    parser.add_argument('--source-dump', type=Path, required=True)
    args = parser.parse_args()
    counts = import_spells(args.db, args.source_dump, args.encounter)
    print(f'Prepared authored {args.encounter} server-source rows: {counts}')


if __name__ == '__main__':
    main()
