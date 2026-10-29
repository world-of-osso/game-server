#!/usr/bin/env python3
"""Place log-observed Ruby final actors on a prepared database copy.

Z values must come from the server ground query at the exact X/Y below and
must have one floor. Coordinates are post-pull medians, not authored spawns.
"""
import argparse
from contextlib import closing
import math
from pathlib import Path
import sqlite3

from ruby_native_templates import import_templates

KYRAKKA_XY = (1786.9, 27.945)
ERKHART_XY = (1785.13, 20.495)
POLICY = 'LOG_DERIVED_POPULATION; interim Z from single-floor server ground query'
PLACEHOLDER = 'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP'


def import_population(db, kyrakka_z, erkhart_z):
    if not all(math.isfinite(z) for z in (kyrakka_z, erkhart_z)):
        raise ValueError('ground heights must be finite')
    placeholder = db.execute('SELECT id1,map,Comment,position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone()
    if placeholder is None or placeholder[:2] != (190485, 2521) or not placeholder[2] or PLACEHOLDER not in placeholder[2]:
        raise ValueError('expected Erkhart journal placeholder not found')
    if db.execute('SELECT 1 FROM content_creature WHERE guid=63521001').fetchone():
        raise ValueError('Kyrakka placement GUID already occupied')
    if db.execute('SELECT 1 FROM content_creature WHERE id1=190484 AND map=2521').fetchone():
        raise ValueError('Kyrakka already has a placement')
    import_templates(db)
    db.execute('UPDATE content_creature SET position_x=?,position_y=?,position_z=?,Comment=? WHERE guid=63521000',
               (*ERKHART_XY, erkhart_z, POLICY + f'; replaced xyz={placeholder[3:6]}; ' + placeholder[2]))
    db.execute('INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z,spawnDifficulties,Comment) VALUES(?,?,?,?,?,?,?,?)',
               (63521001, 190484, 2521, *KYRAKKA_XY, kyrakka_z, '1', POLICY))
    binding = (2521, 190484, 2623, 2, 2503, 5393, 107137)
    existing = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=190484').fetchone()
    if existing is not None and existing != binding:
        raise ValueError('Kyrakka encounter binding conflicts')
    db.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', binding)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--kyrakka-z', required=True, type=float)
    parser.add_argument('--erkhart-z', required=True, type=float)
    args = parser.parse_args()
    canonical = Path(__file__).resolve().parents[1] / 'data/world.db'
    if not args.db.is_file() or args.db.name == 'world.db' or args.db.samefile(canonical):
        parser.error('provide an existing prepared copy, not world.db')
    with closing(sqlite3.connect(args.db)) as db, db:
        import_population(db, args.kyrakka_z, args.erkhart_z)
    print('imported log-derived placements and native templates on copy')


if __name__ == '__main__':
    main()
