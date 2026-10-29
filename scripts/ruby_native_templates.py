#!/usr/bin/env python3
"""Import the sourced Ruby final boss and Firestorm templates into a DB copy."""
import argparse
from contextlib import closing
from pathlib import Path
import sqlite3

import retail_content

ENTRIES = (190484, 189886)


def import_templates(conn):
    for entry in ENTRIES:
        if conn.execute('SELECT count(*) FROM tdb_creature_template WHERE entry=?', (entry,)).fetchone()[0] != 1:
            raise ValueError(f'missing native template {entry}')
        if not conn.execute('SELECT 1 FROM tdb_creature_template_model WHERE CreatureID=?', (entry,)).fetchone():
            raise ValueError(f'missing native model {entry}')
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    retail_content.insert_tdb_templates(conn, ','.join(map(str, ENTRIES)))
    for entry in ENTRIES:
        conn.execute('INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=?', (entry,))
        conn.execute(f'''INSERT OR IGNORE INTO content_creature_template_addon
            SELECT a.entry,{retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=?''', (entry,))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    args = parser.parse_args()
    if args.db.name == 'world.db' or not args.db.is_file():
        parser.error('provide an existing prepared copy, not world.db')
    with closing(sqlite3.connect(args.db)) as conn, conn:
        import_templates(conn)
    print('imported native Ruby templates and models; no spawn changes')


if __name__ == '__main__':
    main()
