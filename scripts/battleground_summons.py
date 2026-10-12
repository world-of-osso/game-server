"""Materialize TC-scripted summons as runtime content; never mutate static tdb_gameobject.

Authority: TrinityCore a352b1fa, TempleOfKotmogu::StaticOrbDataValue/SpawnOrb.
Source spells/templates remain verbatim TDB1210. Run after content_policy.py because
that policy rebuilds content tables. 90M+entry is a runtime prefab ID namespace,
not an invented TDB spawn ID or gameplay identifier.
"""

KOTMOGU_MAP = 998
PREFAB_GUID_BASE = 90_000_000
KOTMOGU_ORBS = (
    (212094, 1850.1666, 1250.118, 13.2056),
    (212091, 1716.9479, 1250.0173, 13.3266),
    (212093, 1716.8923, 1416.618, 13.2056),
    (212092, 1850.217, 1416.8229, 13.3382),
)


def read_orb_templates(conn):
    columns = [row[1] for row in conn.execute('PRAGMA table_info(content_gameobject_template)')]
    names = ','.join(f'"{name}"' for name in columns)
    templates = []
    for entry, *_ in KOTMOGU_ORBS:
        row = conn.execute(f'SELECT {names} FROM tdb_gameobject_template WHERE entry=?', (entry,)).fetchone()
        if row is None:
            raise ValueError(f'Kotmogu source template {entry} is absent')
        template = dict(zip(columns, row))
        if template['type'] != 24:
            raise ValueError(f'Kotmogu source template {entry} is not a flagstand')
        display = conn.execute('SELECT ID FROM game_object_display_info WHERE ID=?',
                               (template['displayId'],)).fetchone()
        if display is None:
            raise ValueError(f'Kotmogu orb {entry} source display {template["displayId"]} is absent')
        guid = PREFAB_GUID_BASE + entry
        existing = conn.execute('SELECT id,map FROM content_gameobject WHERE guid=?', (guid,)).fetchone()
        if existing is not None and existing != (entry, KOTMOGU_MAP):
            raise ValueError(f'Kotmogu prefab GUID {guid} collides with unrelated object {existing}')
        templates.append(row)
    return columns, templates


def import_kotmogu_orbs(conn):
    columns, templates = read_orb_templates(conn)
    names = ','.join(f'"{name}"' for name in columns)
    placeholders = ','.join('?' for _ in columns)
    for template, (entry, x, y, z) in zip(templates, KOTMOGU_ORBS):
        conn.execute('DELETE FROM content_gameobject_template WHERE entry=?', (entry,))
        conn.execute(f'INSERT INTO content_gameobject_template ({names}) VALUES ({placeholders})', template)
        guid = PREFAB_GUID_BASE + entry
        conn.execute('DELETE FROM content_gameobject WHERE guid=?', (guid,))
        # TC SpawnOrb: Position default orientation 0, quaternion identity,
        # animprogress 255, GO_STATE_READY=1, respawn time 7200 seconds.
        conn.execute('''INSERT INTO content_gameobject
            (guid,id,map,position_x,position_y,position_z,orientation,rotation3,
             spawntimesecs,animprogress,state,spawnDifficulties,Comment)
            VALUES (?,?,?,?,?,?,0,1,7200,255,1,'0',?)''',
            (guid, entry, KOTMOGU_MAP, x, y, z,
             'TC a352b1fa TempleOfKotmogu OnInit scripted summon (not a static TDB placement)'))
    return len(templates)
