#!/usr/bin/env python3
"""Lead-owned arena catalog import from pinned DB2 CSV, TDB starts and local CASC WDT receipts.

No spawn promotion or canonical DB mutation occurs unless the operator supplies --db.
"""
import argparse
import csv
import json
import sqlite3
from contextlib import closing
from pathlib import Path


def import_catalog(db, source, available_wdts):
    with source.open() as stream:
        battlemaster = {int(row['ID']): row for row in csv.DictReader(stream)}
    candidates = []
    query = '''SELECT t.ID,a.MapID,b.MapID,t.AllianceStartLoc,t.HordeStartLoc,
                      m.WdtFileDataID,t.Weight
               FROM tdb_battleground_template t
               JOIN tdb_world_safe_locs a ON a.ID=t.AllianceStartLoc
               JOIN tdb_world_safe_locs b ON b.ID=t.HordeStartLoc
               JOIN map m ON m.ID=a.MapID WHERE m.InstanceType=4'''
    for identity, map_a, map_b, start_a, start_b, wdt, weight in db.execute(query):
        row = battlemaster.get(identity)
        if not row or int(row['InstanceType']) != 4 or int(row['PvpType']) != 1:
            continue
        if map_a != map_b:
            raise ValueError(f'{identity}: start maps disagree')
        if wdt not in available_wdts or int(row['Required_Player_Condition_ID']):
            continue
        candidates.append((identity, map_a, row['Name_lang'], start_a, start_b,
                           int(row['MinLevel']), int(row['MaxLevel']), weight, wdt))
    if not candidates:
        raise ValueError('no source-backed arenas with local CASC WDT receipts')
    with db:
        db.execute('''CREATE TABLE IF NOT EXISTS arena_map(
            battlemaster_id INTEGER PRIMARY KEY,map_id INTEGER UNIQUE,name TEXT,
            start_a INTEGER,start_b INTEGER,min_level INTEGER,max_level INTEGER,
            weight INTEGER,wdt_fdid INTEGER)''')
        db.execute('DELETE FROM arena_map')
        db.executemany('INSERT INTO arena_map VALUES(?,?,?,?,?,?,?,?,?)', candidates)
    return len(candidates)


def import_gates(db, source):
    rows = json.loads(source.read_text())
    maps = {row[0] for row in db.execute('SELECT map_id FROM arena_map')}
    gates = [(row['map_id'], row['entry'], row['action'], row['despawn_secs']) for row in rows
             if row['map_id'] in maps and db.execute('SELECT 1 FROM tdb_gameobject WHERE map=? AND id=?',
                                                   (row['map_id'], row['entry'])).fetchone()]
    with db:
        db.execute('CREATE TABLE IF NOT EXISTS arena_gate(map_id INTEGER,entry INTEGER,action TEXT,despawn_secs INTEGER,PRIMARY KEY(map_id,entry))')
        db.execute('DELETE FROM arena_gate')
        db.executemany('INSERT INTO arena_gate VALUES(?,?,?,?)', gates)
    return len(gates)


def import_spouts(db):
    from import_instance_population import BASE_PHASE
    from retail_content import RETAIL_GUID_OFFSET
    rows = []
    if db.execute('SELECT 1 FROM arena_map WHERE map_id=617').fetchone():
        rows = db.execute(f'''SELECT guid+{RETAIL_GUID_OFFSET},map,id,StringId FROM tdb_creature
            WHERE map=617 AND id=28567 AND {BASE_PHASE}
            AND StringId IN('arena_dalaran_sewers_water_spout_pipe','arena_dalaran_sewers_water_spout_center')''').fetchall()
    with db:
        db.execute('CREATE TABLE IF NOT EXISTS arena_spout(guid INTEGER PRIMARY KEY,map_id INTEGER,entry INTEGER,string_id TEXT)')
        db.execute('DELETE FROM arena_spout')
        db.executemany('INSERT INTO arena_spout VALUES(?,?,?,?)', rows)
    return len(rows)


def promote_dalaran_spouts(db):
    from retail_content import RETAIL_GUID_OFFSET
    if not db.execute('SELECT 1 FROM arena_map WHERE map_id=617').fetchone():
        return 0
    source_columns = {row[1] for row in db.execute('PRAGMA table_info(tdb_creature)')}
    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature)') if row[1] in source_columns or row[1]=='id1']
    names = ','.join(f'"{column}"' for column in columns)
    # TDB NULL means no spawn override; the target's0 sentinel means inherit the template.
    values = ','.join(f'g.guid+{RETAIL_GUID_OFFSET}' if column=='guid' else 'g.id' if column=='id1' else f'COALESCE(g."{column}",0)' if column in ('npcflag','unit_flags') else f'g."{column}"' for column in columns)
    template_source = {row[1] for row in db.execute('PRAGMA table_info(tdb_creature_template)')}
    template_columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)') if row[1] in template_source and row[1]!='entry']
    assignments = ','.join(f'"{column}"=(SELECT "{column}" FROM tdb_creature_template WHERE entry=28567)' for column in template_columns)
    with db:
        db.execute('DELETE FROM content_creature WHERE map=617 AND id1=28567')
        db.execute(f'INSERT INTO content_creature({names}) SELECT {values} FROM tdb_creature g JOIN arena_spout s ON s.guid=g.guid+{RETAIL_GUID_OFFSET}')
        # Derived level/tuning columns stay in the existing Retail catalog; literal TDB flags/models replace legacy ones.
        db.execute(f'UPDATE content_creature_template SET {assignments} WHERE entry=28567')
        db.execute('DELETE FROM content_creature_template_model WHERE CreatureID=28567')
        db.execute('INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=28567')
    return db.execute('SELECT count(*) FROM content_creature WHERE map=617 AND id1=28567').fetchone()[0]


def promote_content(db):
    from import_instance_population import BASE_PHASE
    from retail_content import RETAIL_GUID_OFFSET
    # Only arena objects and their templates. Decorative NPCs and unrelated loot
    # rebuilding are not part of arena gate/orb installation.
    template_columns = [row[1] for row in db.execute('PRAGMA table_info(content_gameobject_template)')]
    spawn_columns = [row[1] for row in db.execute('PRAGMA table_info(content_gameobject)')
                     if row[1] in {column[1] for column in db.execute('PRAGMA table_info(tdb_gameobject)')}]
    with db:
        db.execute('BEGIN')
        db.execute(f'CREATE TEMP TABLE arena_source_objects AS SELECT * FROM tdb_gameobject '
                   f'WHERE map IN(SELECT map_id FROM arena_map) AND {BASE_PHASE}')
        names = ','.join(f'"{column}"' for column in template_columns)
        db.execute(f'INSERT OR REPLACE INTO content_gameobject_template({names}) SELECT {names} '
                   'FROM tdb_gameobject_template WHERE entry IN(SELECT id FROM arena_source_objects)')
        db.execute('DELETE FROM content_gameobject WHERE map IN(SELECT map_id FROM arena_map)')
        columns = ','.join(f'"{column}"' for column in spawn_columns)
        values = ','.join(f'guid+{RETAIL_GUID_OFFSET}' if column == 'guid' else f'"{column}"' for column in spawn_columns)
        db.execute(f'INSERT INTO content_gameobject({columns}) SELECT {values} FROM arena_source_objects')
        count = db.execute('SELECT count(*) FROM arena_source_objects').fetchone()[0]
        db.execute('DROP TABLE arena_source_objects')
    return {'gameobjects': count, 'functional_spouts': promote_dalaran_spouts(db)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db', required=True, type=Path)
    parser.add_argument('--battlemaster', type=Path, default=Path(__file__).parent / 'reference/arena-12.1.0.69933/BattlemasterList.csv')
    parser.add_argument('--wdt-dir', type=Path, required=True)
    parser.add_argument('--promote', action='store_true', help='Promote source arena population through existing scoped importer')
    args = parser.parse_args()
    available = {int(path.stem) for path in args.wdt_dir.glob('*.wdt') if path.stat().st_size > 0}
    with closing(sqlite3.connect(args.db.resolve().as_uri() + '?mode=rw', uri=True)) as db:
        print(f'Imported {import_catalog(db, args.battlemaster, available)} source-backed arenas')
        gates = import_gates(db, Path(__file__).parent / 'reference/arena-12.1.0.69933/arena-gates.json')
        print(f'Imported {gates} source-backed gate entries')
        print(f'Imported {import_spouts(db)} source-backed Dalaran spout identities')
        if args.promote:
            print(json.dumps(promote_content(db), sort_keys=True))


if __name__ == '__main__':
    main()
