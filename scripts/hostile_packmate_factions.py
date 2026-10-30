#!/usr/bin/env python3
"""Apply approved hostile reaction and pack-mate faction evidence to a DB copy."""
import argparse
import json
import sqlite3
from pathlib import Path

from import_log_population import validate_copy

POLICY = 'FACTION_FROM_COMBATLOG_HOSTILE_PACKMATE'
# Murder Row Stalker: logged HOSTILE, pulled near Row Hooligan in the follower walk.
# Charonus: logged only HOSTILE; Voidscar's other encounter boss Atroxus supplies faction 16.
# Den of Nalorakk: Sentinel and Frigid Mauler are logged only HOSTILE;
# Hoardmonger's TDB faction supplies the same-map donor.
PACKMATES = {234359: 236073, 239167: 239008, 241872: 241812, 244100: 241812}


def prepare(db, audit):
    db.execute('''CREATE TABLE IF NOT EXISTS log_derived_hostile_packmate (
        entry INTEGER PRIMARY KEY, policy TEXT NOT NULL, packmate_entry INTEGER NOT NULL,
        faction INTEGER NOT NULL, source_file TEXT NOT NULL, source_line INTEGER NOT NULL,
        observed_flags TEXT NOT NULL)''')
    applied, mixed, missing = [], [], []
    for entry, donor in sorted(PACKMATES.items()):
        flags = audit['flags'].get(str(entry), {})
        if not flags:
            missing.append(entry)
            continue
        if set(flags) != {'0x40'}:
            mixed.append(entry)
            continue
        source = flags['0x40']['player_event_example'] or flags['0x40']['example']
        if not source or int(source['flags'], 16) & 0x70 != 0x40:
            raise ValueError(f'hostile evidence missing for {entry}')
        donor_row = db.execute('SELECT faction FROM content_creature_template WHERE entry=?', (donor,)).fetchone()
        if donor_row is None or donor_row[0] != 16:
            raise ValueError(f'pack-mate {donor} faction is missing or changed')
        row = db.execute('SELECT faction FROM content_creature_template WHERE entry=?', (entry,)).fetchone()
        if row is None:
            raise ValueError(f'creature template missing for {entry}')
        provenance = (entry, POLICY, donor, donor_row[0], source['file'], source['line'], source['flags'])
        old = db.execute('SELECT * FROM log_derived_hostile_packmate WHERE entry=?', (entry,)).fetchone()
        if row[0] == 0 and old is None:
            db.execute('UPDATE content_creature_template SET faction=? WHERE entry=?', (donor_row[0], entry))
            db.execute('INSERT INTO log_derived_hostile_packmate VALUES (?,?,?,?,?,?,?)', provenance)
        elif row[0] != donor_row[0] or old != provenance:
            raise ValueError(f'creature {entry} faction or provenance changed')
        applied.append(entry)
    return {'hostile': applied, 'mixed_reactions': mixed, 'no_reaction_observed': missing}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--audit', required=True, type=Path)
    args = parser.parse_args()
    path = validate_copy(args.db)
    with sqlite3.connect(path) as db:
        print(json.dumps(prepare(db, json.loads(args.audit.read_text())), indent=2))


if __name__ == '__main__':
    main()
