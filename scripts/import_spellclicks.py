#!/usr/bin/env python3
"""Project every TDB spellclick row into an independent world database copy."""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3


def import_spellclicks(conn):
    conn.execute('''CREATE TABLE IF NOT EXISTS content_npc_spellclick_spells (
        npc_entry INTEGER NOT NULL, spell_id INTEGER NOT NULL,
        cast_flags INTEGER NOT NULL, user_type INTEGER NOT NULL,
        PRIMARY KEY(npc_entry, spell_id))''')
    conn.execute('''INSERT INTO content_npc_spellclick_spells (npc_entry,spell_id,cast_flags,user_type)
        SELECT npc_entry,spell_id,cast_flags,user_type FROM tdb_npc_spellclick_spells WHERE 1
        ON CONFLICT(npc_entry,spell_id) DO UPDATE SET
          cast_flags=excluded.cast_flags, user_type=excluded.user_type''')
    return conn.execute('SELECT count(*) FROM tdb_npc_spellclick_spells').fetchone()[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    args = parser.parse_args()
    if args.db.name == 'world.db' or args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('use an independent existing database copy, not canonical world.db')
    with closing(sqlite3.connect(args.db)) as conn, conn:
        count = import_spellclicks(conn)
    print(f'Projected {count} TDB spellclick rows')


if __name__ == '__main__':
    main()
