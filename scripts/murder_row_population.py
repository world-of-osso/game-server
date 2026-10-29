#!/usr/bin/env python3
"""Prepare Murder Row's source-backed encounter rows on an independent DB copy."""
import argparse
import json
import sqlite3
from pathlib import Path

from import_log_population import import_population, validate_copy

# TC 15e0ed76 murder_row.h and instance_murder_row.cpp; Journal DB2 70448.
BINDINGS = [(2813, 234647, 3103, 2, 2681, 5853, 75659),
            (2813, 237415, 3105, 3, 2682, 5855, 124577)]
# 100526_202005 combat log 20:51:17.725: first hit on Xathuux.
# The closest logged pack at (8662.72,-5014.65,127) has a baked navmesh
# route to this upper floor (126.20164), but not to the lower candidates.
XATHUUX = dict(source_key='murder-row-xathuux-100526-205117', map=2813,
    npc_id=234647, x=8628.92, y=-5039.0, z=126.20164, facing=0.8230,
    z_method='log_correlated_navmesh_upper_floor',
    observation_event='SPELL_DAMAGE', observation_time='10/5/2026 20:51:17.725-5',
    source_file='WoWCombatLog-100526_202005.txt', level=92,
    population_kind='likely_spawn', summoned=False, identity_ambiguous=False)


# First positioned player event after zone entry, 100126_203256 line 737884.
# 9.167 seconds elapsed after entry: this is an observation, not an exact landing.
ENTRANCE = (81002813, 2813, 8687.87, -4927.05, 19.061333,
            83.4971394844149, None,
            'FOLLOWER_ENTRANCE_LOG_CORRELATED; 100126_203256 22:31:19.537; Entrance')


def prepare(path):
    path = validate_copy(path)
    report = import_population(path, [XATHUUX])
    with sqlite3.connect(path) as db:
        existing = db.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=?', (ENTRANCE[0],)).fetchone()
        if existing is not None and existing != ENTRANCE:
            raise ValueError(f'conflicting entrance: {existing}')
        db.execute('INSERT OR IGNORE INTO tdb_world_safe_locs VALUES (?,?,?,?,?,?,?,?)', ENTRANCE)
        for row in BINDINGS:
            existing = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?', row[:2]).fetchone()
            if existing is not None and existing != row:
                raise ValueError(f'conflicting encounter binding: {existing}')
            db.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES (?,?,?,?,?,?,?)', row)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.db), indent=2, default=list))


if __name__ == '__main__':
    main()
