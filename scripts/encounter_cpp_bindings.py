#!/usr/bin/env python3
"""Prepare source-authenticated primary-actor binding SQL; never modify world.db.

TC instance_murder_row.cpp explicitly assigns completion to Kystia, not Nibbles.
This resolves that Journal multi-actor join without treating every actor death
as encounter success. The lead applies the generated SQL to a reviewed copy.
"""
import argparse
import hashlib
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from raid_encounters import read_journal


SOURCES = {
    'QuelThalas/MurderRow/boss_kystia_manaheart.cpp': '48277cefac3dce24529564b2685b6a75a15a10d1063a875ba06dcf14e382bb2b',
    'QuelThalas/MurderRow/instance_murder_row.cpp': 'b1022ae88166015dd3b85ef01a920bbbcb24be9e48334ca365c5133c71d0b222',
    'QuelThalas/MurderRow/murder_row.h': 'a1129bb03bf3c83795942e342125d07ee69b7472761e35b56e4a395d0d4050e6',
}
PRIMARY = dict(map_id=2813, entry=234648, encounter_id=3101, bit=0,
               script='boss_kystia_manaheart', journal_id=2679, actor_id=5847, display_id=124578)


def prepare_bindings(connection, bindings):
    rows = []
    for binding in bindings:
        encounter = connection.execute('SELECT MapID,Bit FROM dungeon_encounter WHERE ID=?',
                                       (binding['encounter_id'],)).fetchone()
        if encounter != (binding['map_id'], binding['bit']):
            raise ValueError(f"encounter identity changed: {binding['encounter_id']}")
        template = connection.execute('SELECT ScriptName FROM tdb_creature_template WHERE entry=?',
                                      (binding['entry'],)).fetchone()
        if template != (binding['script'],):
            raise ValueError(f"creature script identity changed: {binding['entry']}")
        existing = connection.execute('SELECT encounter_id,bit FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?',
                                      (binding['map_id'],binding['entry'])).fetchone()
        if existing is not None and existing != (binding['encounter_id'],binding['bit']):
            raise ValueError(f"existing encounter binding conflicts: {binding['entry']}")
        rows.append(tuple(int(binding[field]) for field in ['map_id','entry','encounter_id','bit','journal_id','actor_id','display_id']))
    return rows


def binding_sql(bindings):
    statements = ['BEGIN TRANSACTION;']
    for row in bindings:
        values = ','.join(map(str,row))
        statements.append('INSERT INTO raid_encounter_creature '
                          '(map_id,creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id) '
                          f'VALUES({values}) ON CONFLICT(map_id,creature_entry) DO UPDATE SET '
                          'encounter_id=excluded.encounter_id,bit=excluded.bit,'
                          'journal_encounter_id=excluded.journal_encounter_id,'
                          'journal_creature_id=excluded.journal_creature_id,display_id=excluded.display_id;')
    return '\n'.join(statements+['COMMIT;'])+'\n'


def validate_source_files(source):
    for name, expected in SOURCES.items():
        path = source / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f'pinned TC source hash changed: {path}')


def validate_journal(journal, creatures):
    encounters = read_journal(journal,0xB88D11CC,13,{2:2})
    actors = read_journal(creatures,0x81A3C769,9)
    if not any(r[4] == PRIMARY['journal_id'] and r[6] & 0xFFFF == PRIMARY['encounter_id'] for r in encounters):
        raise ValueError('missing Kystia journal encounter relation')
    if not any(r[2] == PRIMARY['actor_id'] and r[3] & 0xFFFF == PRIMARY['journal_id'] and r[4] & 0xFFFFFFFF == PRIMARY['display_id'] for r in actors):
        raise ValueError('missing Kystia journal primary actor relation')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db',type=Path,required=True)
    parser.add_argument('--tc-dir',type=Path,required=True)
    parser.add_argument('--journal',type=Path,required=True)
    parser.add_argument('--creatures',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    args=parser.parse_args()
    validate_source_files(args.tc_dir)
    validate_journal(args.journal,args.creatures)
    with closing(sqlite3.connect(args.db.resolve().as_uri()+'?mode=ro',uri=True)) as conn:
        rows=prepare_bindings(conn,[PRIMARY])
    args.output.write_text(binding_sql(rows))
    args.report.write_text(json.dumps(dict(build='12.1.0.69933',tc_revision='a352b1fa',source_sha256=SOURCES,
                                         primary_actor=PRIMARY,bindings=rows),indent=2)+'\n')
    print(f'Prepared {len(rows)} primary-actor binding; no database modified')


if __name__ == '__main__':
    main()
