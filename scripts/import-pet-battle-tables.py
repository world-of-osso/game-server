#!/usr/bin/env python3
"""Import local CASC BattlePet CSV tables into world.db; no runtime CSV fallback."""
import argparse
import csv
import json
from pathlib import Path
import re
import sqlite3

TABLES = {
    'BattlePetAbility': 'battle_pet_ability',
    'BattlePetAbilityTurn': 'battle_pet_ability_turn',
    'BattlePetAbilityEffect': 'battle_pet_ability_effect',
    'BattlePetEffectProperties': 'battle_pet_effect_properties',
    'BattlePetState': 'battle_pet_state',
    'BattlePetSpeciesState': 'battle_pet_species_state',
    'BattlePetBreedState': 'battle_pet_breed_state',
    'BattlePetBreedQuality': 'battle_pet_breed_quality',
    'BattlePetSpeciesXAbility': 'battle_pet_species_x_ability',
    'BattlePetAbilityState': 'battle_pet_ability_state',
}


def column_type(name):
    if name.endswith('_lang') or name.startswith('ParamLabel_') or name == 'LuaName':
        return 'TEXT'
    if name == 'StateMultiplier':
        return 'REAL'
    return 'INTEGER'


def read_export(path):
    with path.open(newline='', encoding='utf-8-sig') as handle:
        reader = csv.DictReader(handle)
        headers = reader.fieldnames
        if (not headers or 'ID' not in headers or len(set(headers)) != len(headers)
                or any(not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', h) for h in headers)):
            raise ValueError(f'{path.name}: invalid DB2 column names')
        rows = []
        for row in reader:
            if None in row or any(row[h] is None for h in headers):
                raise ValueError(f'{path.name}: malformed CSV row {reader.line_num}')
            converted = []
            for header in headers:
                value = row[header]
                kind = column_type(header)
                try:
                    converted.append(value if kind == 'TEXT' else float(value) if kind == 'REAL' else int(value))
                except ValueError as error:
                    raise ValueError(f'{path.name} row {reader.line_num} {header}: {value!r}') from error
            rows.append(converted)
    return headers, rows


def import_tables(source: Path, database: Path) -> dict[str, int]:
    exports = [(table, *read_export(source / f'{name}.csv')) for name, table in TABLES.items()]
    counts = {}
    with sqlite3.connect(database) as db:
        db.execute('BEGIN IMMEDIATE')
        for table, headers, rows in exports:
            columns = ','.join(f'"{h}" {column_type(h)} NOT NULL' + (' PRIMARY KEY' if h == 'ID' else '') for h in headers)
            db.execute(f'DROP TABLE IF EXISTS {table}')
            db.execute(f'CREATE TABLE {table} ({columns})')
            parameters = ','.join('?' for _ in headers)
            db.executemany(f'INSERT INTO {table} VALUES ({parameters})', rows)
            counts[table] = len(rows)
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('csv_directory', type=Path)
    parser.add_argument('database', type=Path)
    args = parser.parse_args()
    print(json.dumps(import_tables(args.csv_directory, args.database), sort_keys=True))


if __name__ == '__main__':
    main()
