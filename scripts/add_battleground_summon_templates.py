#!/usr/bin/env python3
"""Lead-owned authentic Silvershard/Deephaul summons, paths and spellclick import.

python3 scripts/import-battlegrounds.py --dump <local-TDB.sql> --db <database>
python3 scripts/promote_battlegrounds.py --maps <explicit maps> --db <database>
python3 scripts/add_battleground_summon_templates.py --db <database>
No generated static cart placements: the battleground runtime summons them.
"""
import argparse
import json
import sqlite3
from contextlib import closing
from pathlib import Path

import retail_content

ENTRIES = (60140, 60283, 214690, 217346)
PATHS = (*range(6014000, 6014012), 21469000, 21734600)


def validate_sources(conn):
    for entry in ENTRIES:
        row = conn.execute('''SELECT 1 FROM tdb_creature_template t
            JOIN tdb_creature_template_model m ON m.CreatureID=t.entry
            WHERE t.entry=?''', (entry,)).fetchone()
        if not row:
            raise ValueError(f'missing authentic battleground template/model: {entry}')
    for path in PATHS:
        row = conn.execute('''SELECT p.MoveType,p.Flags,p.Velocity,count(n.NodeId)
            FROM tdb_waypoint_path p JOIN tdb_waypoint_path_node n ON n.PathId=p.PathId
            WHERE p.PathId=? GROUP BY p.PathId''', (path,)).fetchone()
        if not row or row[:3] != (0, 2, None):
            raise ValueError(f'missing or incompatible exact ground cart path: {path}')
    click = conn.execute('SELECT spell_id,cast_flags,user_type FROM tdb_npc_spellclick_spells WHERE npc_entry=60283').fetchall()
    if click != [(124491, 1, 0)]:
        raise ValueError(f'missing or incompatible source Track Switch spellclick: {click}')


def add_templates(conn):
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
    entries = ','.join(str(entry) for entry in ENTRIES)
    before = {entry for (entry,) in conn.execute(f'SELECT entry FROM content_creature_template WHERE entry IN ({entries})')}
    retail_content.insert_tdb_templates(conn, entries)
    for entry in set(ENTRIES) - before:
        conn.execute('''INSERT INTO content_creature_template_model
            SELECT * FROM tdb_creature_template_model WHERE CreatureID=?''', (entry,))
        conn.execute(f'''INSERT INTO content_creature_template_addon
            SELECT a.entry,{retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=?''', (entry,))
    for entry in ENTRIES:
        if not conn.execute('SELECT 1 FROM content_creature_template WHERE entry=?', (entry,)).fetchone():
            raise ValueError(f'missing prepared battleground template: {entry}')


def import_battleground_summons(conn):
    validate_sources(conn)
    add_templates(conn)
    # Fourteen TC paths are dynamic NPC routes, not static spawn addons.
    paths = ','.join(str(path) for path in PATHS)
    conn.execute(f'DELETE FROM content_waypoint_data WHERE id IN ({paths})')
    conn.execute(f'''INSERT INTO content_waypoint_data
        (id,point,position_x,position_y,position_z,orientation,delay,move_type)
        SELECT n.PathId,n.NodeId,n.PositionX,n.PositionY,n.PositionZ,n.Orientation,n.Delay,p.MoveType
        FROM tdb_waypoint_path_node n JOIN tdb_waypoint_path p ON p.PathId=n.PathId
        WHERE n.PathId IN ({paths})''')
    conn.execute('''CREATE TABLE IF NOT EXISTS content_npc_spellclick_spells
        (npc_entry INTEGER,spell_id INTEGER,cast_flags INTEGER,user_type INTEGER,
        PRIMARY KEY(npc_entry,spell_id))''')
    conn.execute('DELETE FROM content_npc_spellclick_spells WHERE npc_entry=60283')
    conn.execute('INSERT INTO content_npc_spellclick_spells SELECT * FROM tdb_npc_spellclick_spells WHERE npc_entry=60283')
    return {'templates': list(ENTRIES), 'paths': list(PATHS), 'spellclick': [60283, 124491, 1, 0]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', type=Path, required=True)
    args = parser.parse_args()
    with closing(sqlite3.connect(args.db.resolve().as_uri() + '?mode=rw', uri=True)) as conn:
        with conn:
            conn.execute('BEGIN')
            result = import_battleground_summons(conn)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
