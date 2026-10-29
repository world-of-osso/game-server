#!/usr/bin/env python3
"""Place log-observed Ruby final actors on a prepared database copy.

Coordinates are post-pull medians, not authored spawns. Z values are interim
sourced geometry estimates from authored neighbor anchors, not logged altitude.
"""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3

from ruby_native_templates import import_templates

KYRAKKA_XY = (1786.9, 27.945)
ERKHART_XY = (1785.13, 20.495)
POLICY = 'LOG_DERIVED_POPULATION; interim Z from authored_neighbor_anchor'
KYRAKKA_Z = 312.92258
ERKHART_Z = 312.59048
KYRAKKA_ANCHORS = 'tdb_creature:9001671,9001749;content_creature:29001671,29001749'
ERKHART_ANCHORS = 'tdb_creature:9001758,9001671,9001749,9001785;content_creature:29001758,29001671,29001749,29001785'
PLACEHOLDER = 'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP'


def import_population(db):
    placeholder = db.execute('SELECT id1,map,Comment,position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone()
    if placeholder is None or placeholder[:2] != (190485, 2521) or not placeholder[2] or PLACEHOLDER not in placeholder[2]:
        raise ValueError('expected Erkhart journal placeholder not found')
    if db.execute('SELECT 1 FROM content_creature WHERE guid=63521001').fetchone():
        raise ValueError('Kyrakka placement GUID already occupied')
    if db.execute('SELECT 1 FROM content_creature WHERE id1=190484 AND map=2521').fetchone():
        raise ValueError('Kyrakka already has a placement')
    import_templates(db)
    partner = db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190485').fetchone()
    kyrakka = db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190484').fetchone()
    if partner != (16, 64) or kyrakka not in ((0, 0), partner):
        raise ValueError('Kyrakka or Erkhart faction/flags differ from the approved policy')
    # LOG_OBSERVED_HOSTILE_ENCOUNTER_ACTOR: all four logs show hostile190484
    # taking player damage; faction and flags come from her encounter partner.
    db.execute('UPDATE content_creature_template SET faction=?,unit_flags=? WHERE entry=190484', partner)
    db.execute('UPDATE content_creature SET position_x=?,position_y=?,position_z=?,Comment=? WHERE guid=63521000',
               (*ERKHART_XY, ERKHART_Z, POLICY + f'; anchors={ERKHART_ANCHORS}; replaced xyz={placeholder[3:6]}; ' + placeholder[2]))
    db.execute('INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z,spawnDifficulties,Comment) VALUES(?,?,?,?,?,?,?,?)',
               (63521001, 190484, 2521, *KYRAKKA_XY, KYRAKKA_Z, '1', POLICY + f'; anchors={KYRAKKA_ANCHORS}'))
    binding = (2521, 190484, 2623, 2, 2503, 5393, 107137)
    existing = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=190484').fetchone()
    if existing is not None and existing != binding:
        raise ValueError('Kyrakka encounter binding conflicts')
    db.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', binding)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    args = parser.parse_args()
    canonical = Path(__file__).resolve().parents[1] / 'data/world.db'
    if not args.db.is_file() or args.db.name == 'world.db' or args.db.samefile(canonical):
        parser.error('provide an existing prepared copy, not world.db')
    with closing(sqlite3.connect(args.db)) as db, db:
        import_population(db)
    print('imported log-derived placements and native templates on copy')


if __name__ == '__main__':
    main()
