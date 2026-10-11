#!/usr/bin/env python3
"""Import pinned local Retail Scenario/ScenarioStep and their criteria into a private DB.

No network or extraction occurs here. Extract FDIDs1139062/1132761 locally first;
Criteria/CriteriaTree.csv come from the existing local achievement export. This
imports scenario data only, never spawns. --db is mandatory: canonical world.db is
not an implicit target. Layout field order: wowdev/WoWDBDefs Scenario B7C9ECF5,
ScenarioStep4F644CD3 (12.1.0.69933).
"""
import argparse
import csv
from pathlib import Path
import re
import sqlite3

import db2_casc

BUILD = '12.1.0.69933'
TABLES = {
    'scenario': (1139062, 0xB7C9ECF5, {1}, [
        ('ID', 0, None), ('Name_lang', 1, None), ('AreaTableID', 2, 0xffff),
        ('Type', 3, 0xff), ('Flags', 4, 0xff), ('UiTextureKitID', 5, None),
        ('UiScenarioDisplayInfoID', 6, None),
    ]),
    'scenario_step': (1132761, 0x4F644CD3, {0, 1}, [
        ('ID', 2, None), ('Description_lang', 0, None), ('Title_lang', 1, None),
        ('ScenarioID', 3, 0xffff), ('CriteriatreeID', 4, None),
        ('RewardQuestID', 5, None), ('RelatedStep', 6, None), ('Supersedes', 7, 0xffff),
        ('OrderIndex', 8, 0xff), ('Flags', 9, None),
        ('VisibilityPlayerConditionID', 10, None), ('WidgetSetID', 11, 0xffff),
    ]),
}


def export_tables(directory):
    if directory.name != BUILD:
        raise ValueError(f'requires local Retail build {BUILD}')
    tables = {}
    for table, (fdid, layout, strings, columns) in TABLES.items():
        header, records = db2_casc.decode((directory / f'{fdid}.db2').read_bytes(), strings)
        if header['layout_hash'] != layout:
            raise ValueError(f'{table}: layout {header["layout_hash"]:08X} != {layout:08X}')
        rows = [[values[index] if mask is None else values[index] & mask
                 for _, index, mask in columns] for _, values in sorted(records.items())]
        tables[table] = ([name for name, _, _ in columns], rows)
    for source, table in [('Criteria', 'criteria'), ('CriteriaTree', 'criteria_tree')]:
        with (directory / f'{source}.csv').open(newline='', encoding='utf-8') as f:
            reader = csv.reader(f)
            columns = next(reader)
            tables[table] = (columns, [[value if columns[i].endswith('_lang') else int(value)
                                      for i, value in enumerate(row)] for row in reader])
    return tables


def scenario_bindings(tdb):
    with tdb.open(encoding='utf-8') as source:
        for line in source:
            if line.startswith('INSERT INTO `scenarios` VALUES '):
                rows = [tuple(map(int, match)) for match in re.findall(
                    r'\((\d+),(\d+),(\d+),(\d+)\)', line)]
                if not rows:
                    raise ValueError('local TDB scenarios INSERT has no readable rows')
                return rows
    raise ValueError('local TDB scenarios INSERT not found')


def import_tables(db, tables, tdb):
    bindings = scenario_bindings(tdb)
    with db:
        for table, (columns, rows) in tables.items():
            db.execute(f'DROP TABLE IF EXISTS "{table}"')
            declarations = [f'"{name}" ' + ('TEXT' if name.endswith('_lang') else 'INTEGER')
                            + (' PRIMARY KEY' if name == 'ID' else ' NOT NULL') for name in columns]
            db.execute(f'CREATE TABLE "{table}" ({", ".join(declarations)})')
            db.executemany(f'INSERT INTO "{table}" VALUES ({",".join("?" for _ in columns)})', rows)
        db.execute('DROP TABLE IF EXISTS scenario_map')
        db.execute('CREATE TABLE scenario_map (map INTEGER NOT NULL, difficulty INTEGER NOT NULL, scenario_A INTEGER NOT NULL, scenario_H INTEGER NOT NULL, PRIMARY KEY(map,difficulty))')
        db.executemany('INSERT INTO scenario_map VALUES (?,?,?,?)', bindings)
        db.execute('CREATE TABLE IF NOT EXISTS db2_build (table_name TEXT PRIMARY KEY, build TEXT NOT NULL)')
        for table in tables:
            db.execute('INSERT OR REPLACE INTO db2_build VALUES (?,?)', (table, BUILD))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2-dir', type=Path, required=True)
    parser.add_argument('--tdb', type=Path, required=True)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    if args.db.resolve() == (Path(__file__).resolve().parent.parent / 'data/world.db').resolve():
        parser.error('canonical world.db writes prohibited; use a private copy and review the import first')
    tables = export_tables(args.db2_dir)
    with sqlite3.connect(args.db) as db:
        import_tables(db, tables, args.tdb)
    for table, (_, rows) in tables.items():
        print(f'{table}: {len(rows)} local rows -> {args.db}')


if __name__ == '__main__':
    main()
