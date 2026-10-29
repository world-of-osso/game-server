#!/usr/bin/env python3
"""Prepare Azure primary encounter credit on an independent world database copy."""
import argparse
import csv
import sqlite3
from contextlib import closing
from pathlib import Path

from encounter_cpp_bindings import prepare_bindings

# Pinned TC azure_vault.h/instance_azure_vault.cpp primaries; native70448
# DungeonEncounter bits and JournalEncounter/JournalEncounterCreature identities.
BOSSES = [
    (186644, 2582, 0, 'boss_leymor', 2492, 5366, 107127, 9001810),
    (186739, 2585, 3, '', 2505, 5402, 106829, 9001975),
    (186737, 2583, 1, 'boss_telash_greywing', 2483, 5410, 109087, 9001910),
    (186738, 2584, 2, '', 2508, 5399, 106802, 9002017),
]
RUNTIME_SCRIPTS = ['boss_leymor', 'boss_azureblade', 'boss_telash_greywing', 'boss_umbrelskul']


def read_rows(path):
    with path.open(newline='', encoding='utf-8-sig') as source:
        return {int(row['ID']): row for row in csv.DictReader(source)}


def prepare_azure_bindings(connection, db2):
    encounters = read_rows(db2 / 'JournalEncounter.csv')
    creatures = read_rows(db2 / 'JournalEncounterCreature.csv')
    bindings = []
    for (entry, encounter, bit, script, journal, actor, display, spawn), runtime_script in zip(BOSSES, RUNTIME_SCRIPTS, strict=True):
        if int(encounters[journal]['DungeonEncounterID']) != encounter:
            raise ValueError(f'Journal encounter identity changed: {entry}')
        creature = creatures[actor]
        if (int(creature['JournalEncounterID']), int(creature['CreatureDisplayInfoID'])) != (journal, display):
            raise ValueError(f'Journal creature identity changed: {entry}')
        runtime = connection.execute('SELECT ScriptName FROM content_creature_template WHERE entry=?', (entry,)).fetchone()
        if runtime != (runtime_script,):
            raise ValueError(f'runtime script missing or changed: {entry}')
        bindings.append(dict(map_id=2515, entry=entry, encounter_id=encounter, bit=bit,
                             script=script, ai='', journal_id=journal, actor_id=actor,
                             display_id=display, spawn_guid=spawn, spawn_difficulties='2,8,23,1'))
    rows = prepare_bindings(connection, bindings)
    connection.executemany('''INSERT INTO raid_encounter_creature
        (map_id,creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id)
        VALUES(?,?,?,?,?,?,?) ON CONFLICT(map_id,creature_entry) DO UPDATE SET
        encounter_id=excluded.encounter_id,bit=excluded.bit,journal_encounter_id=excluded.journal_encounter_id,
        journal_creature_id=excluded.journal_creature_id,display_id=excluded.display_id''', rows)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--db2', required=True, type=Path)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('use an independent existing database copy, not canonical world.db')
    with closing(sqlite3.connect(args.db)) as connection, connection:
        rows = prepare_azure_bindings(connection, args.db2)
    print(f'Prepared Azure encounter bindings: {rows}')


if __name__ == '__main__':
    main()
