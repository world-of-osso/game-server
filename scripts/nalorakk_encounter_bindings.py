#!/usr/bin/env python3
"""Prepare log-observed Den of Nalorakk encounter credit on a private DB copy."""
import argparse
import csv
import sqlite3
from contextlib import closing
from pathlib import Path

# WoWCombatLog-101926_223417.txt: PARTY_KILL immediately precedes successful
# ENCOUNTER_END 3207 at 22:42:52.899 and 3208 at 22:50:48.146. Journal
# actor/display identities and encounter bits are from the installed DB2 rows.
BOSSES = ((241812, 3207, 0, 2776, 5941, 129344),
          (244100, 3208, 1, 2777, 5948, 129418))


def read_rows(path):
    with path.open(newline='', encoding='utf-8-sig') as source:
        return {int(row['ID']): row for row in csv.DictReader(source)}


def prepare_bindings(db, db2):
    encounters = read_rows(db2 / 'JournalEncounter.csv')
    actors = read_rows(db2 / 'JournalEncounterCreature.csv')
    rows = []
    for entry, encounter, bit, journal, actor, display in BOSSES:
        if int(encounters[journal]['DungeonEncounterID']) != encounter:
            raise ValueError(f'journal encounter identity changed: {entry}')
        row = actors[actor]
        if (int(row['JournalEncounterID']), int(row['CreatureDisplayInfoID'])) != (journal, display):
            raise ValueError(f'journal actor identity changed: {entry}')
        if db.execute('SELECT MapID, Bit FROM dungeon_encounter WHERE ID=?', (encounter,)).fetchone() != (2825, bit):
            raise ValueError(f'encounter map or bit changed: {entry}')
        if not db.execute('SELECT 1 FROM content_creature WHERE map=2825 AND id1=?', (entry,)).fetchone():
            raise ValueError(f'normal population missing: {entry}')
        binding = (2825, entry, encounter, bit, journal, actor, display)
        existing = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2825 AND creature_entry=?', (entry,)).fetchone()
        if existing is not None and existing != binding:
            raise ValueError(f'encounter binding conflicts: {entry}')
        rows.append(binding)
    db.executemany('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', rows)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--db2', required=True, type=Path)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('use an independent existing database copy, not canonical world.db')
    with closing(sqlite3.connect(args.db)) as connection, connection:
        rows = prepare_bindings(connection, args.db2)
    print(f'Prepared Den of Nalorakk encounter bindings: {rows}')


if __name__ == '__main__':
    main()
