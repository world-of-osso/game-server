#!/usr/bin/env python3
"""Apply unambiguous logged friendly reactions to a world database copy."""
import argparse
import json
import sqlite3
from pathlib import Path

from import_log_population import validate_copy

POLICY = 'FACTION_FROM_COMBATLOG_REACTION_FLAGS'
FRIENDLY_FACTION = 35


def prepare(db, audit):
    db.execute('''CREATE TABLE IF NOT EXISTS log_derived_faction_reaction (
        entry INTEGER PRIMARY KEY, policy TEXT NOT NULL, source_file TEXT NOT NULL,
        source_line INTEGER NOT NULL, observed_flags TEXT NOT NULL)''')
    scoped = {int(row[1]) for row in audit['scope']}
    applied, conflicts, missing = [], [], []
    for entry in sorted(scoped):
        flags = audit['flags'][str(entry)]
        if not flags:
            missing.append(entry)
            continue
        if len(flags) > 1:
            conflicts.append(entry)
            continue
        if set(flags) != {'0x10'}:
            continue
        source = flags['0x10']['player_event_example'] or flags['0x10']['example']
        if not source or int(source['flags'], 16) & 0x70 != 0x10:
            raise ValueError(f'friendly evidence missing for {entry}')
        row = db.execute('SELECT faction FROM content_creature_template WHERE entry=?', (entry,)).fetchone()
        if row is None:
            raise ValueError(f'creature template missing for {entry}')
        existing = db.execute('SELECT policy,source_file,source_line,observed_flags FROM log_derived_faction_reaction WHERE entry=?', (entry,)).fetchone()
        provenance = (POLICY, source['file'], source['line'], source['flags'])
        if row[0] == 0:
            db.execute('UPDATE content_creature_template SET faction=? WHERE entry=?', (FRIENDLY_FACTION, entry))
            db.execute('INSERT INTO log_derived_faction_reaction VALUES (?,?,?,?,?)', (entry, *provenance))
        elif row[0] != FRIENDLY_FACTION or existing != provenance:
            raise ValueError(f'creature {entry} faction or provenance changed')
        applied.append(entry)
    return {'friendly': applied, 'conflicting_reactions': conflicts, 'no_reaction_observed': missing}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--audit', required=True, type=Path)
    args = parser.parse_args()
    path = validate_copy(args.db)
    audit = json.loads(args.audit.read_text())
    with sqlite3.connect(path) as db:
        report = prepare(db, audit)
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
