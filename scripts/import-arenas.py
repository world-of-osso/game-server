#!/usr/bin/env python3
"""Lead-owned arena catalog import from pinned DB2 CSV, TDB starts and local CASC WDT receipts.

No spawn promotion or canonical DB mutation occurs unless the operator supplies --db.
"""
import argparse
import csv
import sqlite3
from contextlib import closing
from pathlib import Path


def import_catalog(db, source, available_wdts):
    with source.open() as stream:
        battlemaster = {int(row['ID']): row for row in csv.DictReader(stream)}
    candidates = []
    query = '''SELECT t.ID,a.MapID,b.MapID,t.AllianceStartLoc,t.HordeStartLoc,
                      m.WdtFileDataID,t.Weight
               FROM tdb_battleground_template t
               JOIN tdb_world_safe_locs a ON a.ID=t.AllianceStartLoc
               JOIN tdb_world_safe_locs b ON b.ID=t.HordeStartLoc
               JOIN map m ON m.ID=a.MapID WHERE m.InstanceType=4'''
    for identity, map_a, map_b, start_a, start_b, wdt, weight in db.execute(query):
        row = battlemaster.get(identity)
        if not row or int(row['InstanceType']) != 4 or int(row['PvpType']) != 1:
            continue
        if map_a != map_b:
            raise ValueError(f'{identity}: start maps disagree')
        if wdt not in available_wdts or int(row['Required_Player_Condition_ID']):
            continue
        candidates.append((identity, map_a, row['Name_lang'], start_a, start_b,
                           int(row['MinLevel']), int(row['MaxLevel']), weight, wdt))
    if not candidates:
        raise ValueError('no source-backed arenas with local CASC WDT receipts')
    with db:
        db.execute('''CREATE TABLE IF NOT EXISTS arena_map(
            battlemaster_id INTEGER PRIMARY KEY,map_id INTEGER UNIQUE,name TEXT,
            start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,
            weight INTEGER,wdt_fdid INTEGER)''')
        db.execute('DELETE FROM arena_map')
        db.executemany('INSERT INTO arena_map VALUES(?,?,?,?,?,?,?,?,?)', candidates)
    return len(candidates)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--battlemaster', type=Path, default=Path(__file__).parent / 'reference/arena-12.1.0.69933/BattlemasterList.csv')
    parser.add_argument('--wdt-dir', type=Path, required=True)
    args = parser.parse_args()
    available = {int(path.stem) for path in args.wdt_dir.glob('*.wdt') if path.stat().st_size > 0}
    with closing(sqlite3.connect(args.db.resolve().as_uri() + '?mode=rw', uri=True)) as db:
        print(f'Imported {import_catalog(db, args.battlemaster, available)} source-backed arenas')


if __name__ == '__main__':
    main()
