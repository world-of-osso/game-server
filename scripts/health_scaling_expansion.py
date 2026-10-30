#!/usr/bin/env python3
"""Derive placeholder HealthScalingExpansion 0 from ContentTuning on a DB copy.

TDB creature_template_difficulty rows with HealthScalingExpansion 0 give Retail trash a
Classic expansion: TC Creature::GetAttackDistance then caps a level-80 creature by
GetMaxLevelForExpansion(0) = 30 and its aggro radius clamps to the 5yd minimum. For entries
spawned on follower-dungeon maps, a 0 row whose ContentTuningID resolves in native
ContentTuning takes that row's ExpansionID, and exp follows (retail_content.health_expansion).

Skipped, with the reason recorded in the report:
- ExpansionID < 0 (generic player-scaling tunings such as 81, 1279, 2702 name no expansion);
- ExpansionID 0 (already equal);
- tunings that sniffed nonzero tdb rows contradict (2940: ExpansionID 9, sniffed 10).
Never writes canonical world.db; the lead publishes.
"""
import argparse
import csv
from collections import Counter, defaultdict
from contextlib import closing
from pathlib import Path
import sqlite3

POLICY = 'HEALTH_SCALING_EXPANSION_FROM_CONTENT_TUNING'
BUILD = '12.1.5.70448'
# TC CreatureData.h: EXPANSION_LEVEL_CURRENT -1 is CURRENT_EXPANSION, 11 (Midnight).
CURRENT_EXPANSION = 11
# retail_content.health_expansion: AzerothCore basehp column exp for a positive expansion.
WRATH_HEALTH_EXPANSIONS = 2
TABLES = {
    'content_creature_template': ('entry', '0'),
    'content_creature_template_difficulty': ('entry', 'DifficultyID'),
}


def read_content_tuning(path):
    with open(path, newline='') as handle:
        return {int(row['ID']): int(row['ExpansionID']) for row in csv.DictReader(handle)}


def contradicted_tunings(connection, expansions):
    """Tunings with a sniffed nonzero tdb row whose expansion differs from ExpansionID."""
    sniffed = defaultdict(Counter)
    for tuning, expansion in connection.execute(
            'SELECT ContentTuningID, HealthScalingExpansion FROM tdb_creature_template_difficulty '
            'WHERE HealthScalingExpansion != 0'):
        sniffed[tuning][CURRENT_EXPANSION if expansion == -1 else expansion] += 1
    return {tuning: dict(counts) for tuning, counts in sniffed.items()
            if tuning in expansions and set(counts) != {expansions[tuning]}}


def follower_entries(connection):
    return connection.execute(
        'SELECT DISTINCT c.id1 FROM content_creature c WHERE c.map IN '
        '(SELECT map_id FROM follower_lfg_dungeons WHERE difficulty_id = 205)').fetchall()


def derive(connection, expansions):
    """Update the copy; returns (changes, skipped) per (table, entry, difficulty)."""
    contradicted = contradicted_tunings(connection, expansions)
    connection.execute('CREATE TEMP TABLE follower_entry (entry INTEGER PRIMARY KEY)')
    connection.executemany('INSERT INTO follower_entry VALUES (?)', follower_entries(connection))
    connection.execute("""CREATE TABLE IF NOT EXISTS health_scaling_expansion_source (
        table_name TEXT NOT NULL, entry INTEGER NOT NULL, DifficultyID INTEGER NOT NULL,
        ContentTuningID INTEGER NOT NULL, old_value INTEGER NOT NULL, new_value INTEGER NOT NULL,
        policy TEXT NOT NULL, build TEXT NOT NULL, PRIMARY KEY (table_name, entry, DifficultyID))""")
    changes, skipped = [], Counter()
    for table, (entry, difficulty) in TABLES.items():
        rows = connection.execute(
            f'SELECT {entry}, {difficulty}, ContentTuningID FROM {table} '
            'WHERE HealthScalingExpansion = 0 AND ContentTuningID != 0 '
            'AND entry IN (SELECT entry FROM follower_entry) ORDER BY 1, 2').fetchall()
        for row_entry, row_difficulty, tuning in rows:
            expansion = expansions.get(tuning)
            if expansion is None:
                skipped[(tuning, 'not in ContentTuning')] += 1
            elif expansion < 0:
                skipped[(tuning, f'ExpansionID {expansion} names no expansion')] += 1
            elif tuning in contradicted:
                skipped[(tuning, f'sniffed {contradicted[tuning]} contradicts ExpansionID {expansion}')] += 1
            elif expansion > 0:
                changes.append((table, row_entry, row_difficulty, tuning, expansion))
    for table, row_entry, row_difficulty, tuning, expansion in changes:
        key = 'entry = ?' if table == 'content_creature_template' else 'entry = ? AND DifficultyID = ?'
        params = (row_entry,) if table == 'content_creature_template' else (row_entry, row_difficulty)
        connection.execute(f'UPDATE {table} SET HealthScalingExpansion = ?, exp = ? WHERE {key}',
                           (expansion, min(expansion, WRATH_HEALTH_EXPANSIONS), *params))
        connection.execute('INSERT OR REPLACE INTO health_scaling_expansion_source VALUES (?,?,?,?,?,?,?,?)',
                           (table, row_entry, row_difficulty, tuning, 0, expansion, POLICY, BUILD))
    return changes, skipped


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    parser.add_argument('--content-tuning', type=Path, required=True, help=f'native {BUILD} ContentTuning.csv')
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file():
        parser.error('use an existing independent database copy, not world.db')
    expansions = read_content_tuning(args.content_tuning)
    with closing(sqlite3.connect(args.db)) as connection, connection:
        changes, skipped = derive(connection, expansions)
    by_tuning = Counter((table, tuning, expansion) for table, _, _, tuning, expansion in changes)
    for (table, tuning, expansion), count in sorted(by_tuning.items()):
        print(f'{POLICY}: {table} ContentTuning {tuning} -> {expansion}: {count} rows')
    for (tuning, reason), count in sorted(skipped.items()):
        print(f'SKIPPED ContentTuning {tuning}: {reason}: {count} rows')


if __name__ == '__main__':
    main()
