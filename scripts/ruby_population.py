#!/usr/bin/env python3
"""Place log-observed Ruby final actors on a prepared database copy.

Coordinates are post-pull medians, not authored spawns. Z values are interim
sourced geometry estimates from authored neighbor anchors, not logged altitude.
"""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3

from import_log_population import retire_encounter_duplicates
from ruby_native_templates import import_templates

KYRAKKA_XY = (1786.9, 27.945)
ERKHART_XY = (1785.13, 20.495)
POLICY = 'LOG_DERIVED_POPULATION; interim Z from authored_neighbor_anchor'
KYRAKKA_Z = 312.92258
ERKHART_Z = 312.59048
KYRAKKA_ANCHORS = 'tdb_creature:9001671,9001749;content_creature:29001671,29001749'
ERKHART_ANCHORS = 'tdb_creature:9001758,9001671,9001749,9001785;content_creature:29001758,29001671,29001749,29001785'
PLACEHOLDER = 'FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP'


def placements_applied(db):
    """False on the journal placeholder, True once this import placed both actors.

    Raises on anything else, including a half-applied import.
    """
    erkhart = db.execute('SELECT id1,map,Comment,position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone()
    if erkhart is None or erkhart[:2] != (190485, 2521) or not erkhart[2] or PLACEHOLDER not in erkhart[2]:
        raise ValueError('expected Erkhart journal placeholder not found')
    kyrakka = db.execute('SELECT id1,map,position_x,position_y,position_z,spawnDifficulties FROM content_creature WHERE guid=63521001').fetchone()
    erkhart_placed = erkhart[2].startswith(POLICY) and erkhart[3:] == (*ERKHART_XY, ERKHART_Z)
    if kyrakka is None and not erkhart_placed:
        if db.execute('SELECT 1 FROM content_creature WHERE id1=190484 AND map=2521').fetchone():
            raise ValueError('Kyrakka already has a placement')
        return False
    if erkhart_placed and kyrakka == (190484, 2521, *KYRAKKA_XY, KYRAKKA_Z, '1'):
        return True
    raise ValueError('Ruby placements are partially applied or conflict with GUID 63521000/63521001')


def import_population(db):
    """Apply the Ruby placements; a database that already holds them stays unchanged."""
    applied = placements_applied(db)
    import_templates(db)
    partner = db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190485').fetchone()
    kyrakka = db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190484').fetchone()
    if partner != (16, 64) or kyrakka not in ((0, 0), partner):
        raise ValueError('Kyrakka or Erkhart faction/flags differ from the approved policy')
    # LOG_OBSERVED_HOSTILE_ENCOUNTER_ACTOR: all four logs show hostile190484
    # taking player damage; faction and flags come from her encounter partner.
    db.execute('UPDATE content_creature_template SET faction=?,unit_flags=? WHERE entry=190484', partner)
    for entry in (190484, 190485):
        binding = db.execute('SELECT AIName,ScriptName FROM content_creature_template WHERE entry=?', (entry,)).fetchone()
        if binding not in (('', ''), ('', 'boss_kyrakka_erkhart')):
            raise ValueError(f'conflicting Ruby actor script {entry}')
        db.execute("UPDATE content_creature_template SET ScriptName='boss_kyrakka_erkhart' WHERE entry=?", (entry,))
    if not applied:
        placeholder = db.execute('SELECT Comment,position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone()
        db.execute('UPDATE content_creature SET position_x=?,position_y=?,position_z=?,Comment=? WHERE guid=63521000',
                   (*ERKHART_XY, ERKHART_Z, POLICY + f'; anchors={ERKHART_ANCHORS}; replaced xyz={placeholder[1:]}; ' + placeholder[0]))
        db.execute('INSERT INTO content_creature(guid,id1,map,position_x,position_y,position_z,spawnDifficulties,Comment) VALUES(?,?,?,?,?,?,?,?)',
                   (63521001, 190484, 2521, *KYRAKKA_XY, KYRAKKA_Z, '1', POLICY + f'; anchors={KYRAKKA_ANCHORS}'))
    binding = (2521, 190484, 2623, 2, 2503, 5393, 107137)
    existing = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=190484').fetchone()
    if existing is not None and existing != binding:
        raise ValueError('Kyrakka encounter binding conflicts')
    db.execute('INSERT OR IGNORE INTO raid_encounter_creature VALUES(?,?,?,?,?,?,?)', binding)
    # The log importer added the observed Erkhart while the placeholder was 12 yards away.
    retire_encounter_duplicates(db)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    args = parser.parse_args()
    canonical = Path(__file__).resolve().parents[1] / 'data/world.db'
    if not args.db.is_file() or args.db.name == 'world.db' or args.db.samefile(canonical):
        parser.error('provide an existing prepared copy, not world.db')
    if args.db.is_symlink() or args.db.stat().st_nlink != 1:
        parser.error('copy must be an independent regular file')
    with closing(sqlite3.connect(args.db)) as db, db:
        import_population(db)
    print('imported log-derived placements and native templates on copy')


if __name__ == '__main__':
    main()
