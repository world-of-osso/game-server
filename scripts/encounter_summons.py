#!/usr/bin/env python3
"""Admit source-selected encounter summon templates into independent COPIES.

No placements, guessed stats, loot or schedules. Kystia: local TDB255050/models/
Normal1 and DB2 summon1264098. Golden Serpent: pinned Spit Gold removal summons
TDB135406 and the Serpentine Seal summons Shadow of Zul137020; each gets its actual
base0 row, models and template addon; no new difficulty row is fabricated.
Adarogg: server-side Inferno Charge119297 summons TDB61409 (base0 + Normal1).
Wailing ritual: source groups2/3 select5762/5763; project only their actual Normal1
rows, retaining existing base templates/models and all population/credit.
"""
import argparse
import sqlite3
from contextlib import closing
from pathlib import Path

import retail_content

MIRROR = 255050
ANIMATED_GOLD = 135406
SHADOW_OF_ZUL = 137020
DEMONIC_LEAP = 61409


def import_mirror_template(connection):
    effect = connection.execute('SELECT Effect,EffectMiscValue_0 FROM spell_effect WHERE SpellID=1264098 AND EffectIndex=0 AND DifficultyID=0').fetchone()
    if effect != (28,MIRROR):
        raise ValueError('Mirror Image1264098 summon identity changed')
    row = connection.execute('SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=?',(MIRROR,)).fetchone()
    if row != ('SmartAI',''):
        raise ValueError('Mirror source AI identity changed')
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    connection.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=? AND DifficultyID=1',(MIRROR,))
    if connection.execute('SELECT count(*) FROM temp.tdb_difficulty').fetchone()[0] != 1:
        raise ValueError('Missing authored Normal mirror difficulty1')
    retail_content.insert_tdb_templates(connection,str(MIRROR))
    connection.execute('INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=?',(MIRROR,))
    # Only the actual authored row is projected: other difficulties remain gaps,
    # never manufactured by copying these numeric values to another difficulty.
    project_difficulty(connection, MIRROR, 1)
    return MIRROR


def project_difficulty(connection, entry, difficulty):
    """Project one actual authored TDB difficulty row with its content tuning levels."""
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('''INSERT OR REPLACE INTO content_creature_template_difficulty
        (entry,DifficultyID,minlevel,maxlevel,exp,ContentTuningID,HealthScalingExpansion,
         LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,ArmorModifier,
         DamageModifier,lootid,mingold,maxgold)
        SELECT d.Entry,d.DifficultyID,l.minlevel,l.maxlevel,health_expansion(d.HealthScalingExpansion),
            d.ContentTuningID,d.HealthScalingExpansion,d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,
            d.HealthModifier,d.ManaModifier,d.ArmorModifier,d.DamageModifier,d.LootID,d.GoldMin,d.GoldMax
        FROM tdb_creature_template_difficulty d JOIN content_tuning_level l ON l.ID=d.ContentTuningID
        WHERE d.Entry=? AND d.DifficultyID=?''',(entry,difficulty))
    if connection.execute('SELECT count(*) FROM content_creature_template_difficulty WHERE entry=? AND DifficultyID=?',(entry,difficulty)).fetchone()[0] != 1:
        raise ValueError(f'Missing authored {entry} difficulty{difficulty} content tuning')


def import_base_template(connection, entry, script_name):
    """Project one source script's actual base difficulty0 template and models."""
    row = connection.execute('SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=?',(entry,)).fetchone()
    if row != ('',script_name):
        raise ValueError(f'{entry} source AI identity changed')
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    connection.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=? AND DifficultyID=0',(entry,))
    if connection.execute('SELECT count(*) FROM temp.tdb_difficulty').fetchone()[0] != 1:
        raise ValueError(f'Missing authored {entry} base difficulty0')
    retail_content.insert_tdb_templates(connection,str(entry))
    connection.execute('INSERT OR IGNORE INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=?',(entry,))
    # retail_content.build_addons' projection of the template's actual addon row.
    connection.execute(f'''INSERT OR IGNORE INTO content_creature_template_addon
        SELECT a.entry, {retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=?''',(entry,))
    return entry


def import_animated_gold_template(connection):
    return import_base_template(connection, ANIMATED_GOLD, 'npc_animated_gold')


def import_shadow_of_zul_template(connection):
    """go_kings_rest_serpentine_seal summons NPC_SHADOW_OF_ZUL137020."""
    return import_base_template(connection, SHADOW_OF_ZUL, 'npc_kings_rest_shadow_of_zul')


def import_demonic_leap_template(connection):
    """boss_adarogg's server-side Inferno Charge119297 summons Demonic Leap61409.

    Its identity is the prepared tdb_serverside_spell_effect row, so
    encounter_serverside_spells.py must have run on this copy first. The copy
    receives the actual base row/models and the authored Normal difficulty1 row.
    """
    try:
        effect = connection.execute('SELECT Effect,EffectMiscValue1,EffectMiscValue2 FROM tdb_serverside_spell_effect WHERE SpellID=119297 AND EffectIndex=0 AND DifficultyID=0').fetchone()
    except sqlite3.OperationalError as error:
        raise ValueError('prepare server-side spells with encounter_serverside_spells.py first') from error
    if effect != (28,DEMONIC_LEAP,491):
        raise ValueError('Inferno Charge119297 summon identity changed')
    import_base_template(connection, DEMONIC_LEAP, '')
    project_difficulty(connection, DEMONIC_LEAP, 1)
    return DEMONIC_LEAP


def import_wailing_ritual_difficulties(connection):
    """TC a352b1fa Disciple groups2/3: only authored Normal wave stats."""
    groups = connection.execute('''SELECT groupId,entry,count(*) FROM tdb_creature_summon_groups
        WHERE summonerId=3678 AND summonerType=0 AND groupId IN (2,3)
        GROUP BY groupId,entry ORDER BY groupId,entry''').fetchall()
    if groups != [(2,5762,3),(3,5763,7)]:
        raise ValueError('Wailing ritual wave identity changed')
    entries = [5762,5763]
    source = connection.execute('''SELECT d.Entry FROM tdb_creature_template_difficulty d
        JOIN content_tuning_level l ON l.ID=d.ContentTuningID
        WHERE d.Entry IN (5762,5763) AND d.DifficultyID=1 ORDER BY d.Entry''').fetchall()
    if source != [(entry,) for entry in entries]:
        raise ValueError('Missing authored Wailing Normal wave difficulty or content tuning')
    templates = connection.execute('''SELECT entry FROM content_creature_template
        WHERE entry IN (5762,5763) ORDER BY entry''').fetchall()
    if templates != [(entry,) for entry in entries]:
        raise ValueError('Missing runtime Wailing wave base template')
    for entry in entries:
        project_difficulty(connection, entry, 1)
    return entries


def import_noxxion_spawn_difficulty(connection):
    """Pinned21708 aura triggers21707: five13456 summons, actual Normal1 stats."""
    effect = connection.execute('''SELECT Effect,EffectMiscValue_0,EffectBasePointsF
        FROM spell_effect WHERE SpellID=21707 AND EffectIndex=0 AND DifficultyID=0''').fetchone()
    if effect != (28,13456,5.0):
        raise ValueError('Noxxion21707 summon identity changed')
    template = connection.execute('SELECT AIName,ScriptName FROM content_creature_template WHERE entry=13456').fetchone()
    if template != ('SmartAI',''):
        raise ValueError('Noxxion spawn13456 runtime AI identity changed')
    project_difficulty(connection, 13456, 1)
    return [13456]


def import_celebras_redeemed(connection):
    """Pinned Celebras JustDied summons13716; replace its legacy base, not difficulty1.

    TDB1210 has only base0/ContentTuning0. Preserve that absence rather than
    invent a Normal row or assign the boss's tuning to the redeemed actor.
    """
    identity = connection.execute('SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=13716').fetchone()
    difficulties = connection.execute('SELECT DifficultyID FROM tdb_creature_template_difficulty WHERE Entry=13716 ORDER BY DifficultyID').fetchall()
    if identity != ('SmartAI','') or difficulties != [(0,)]:
        raise ValueError('Celebras redeemed13716 source identity changed')
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    connection.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=13716 AND DifficultyID=0')
    connection.execute('DELETE FROM content_creature_template WHERE entry=13716')
    retail_content.insert_tdb_templates(connection,'13716',replace_legacy=True)
    connection.execute('DELETE FROM content_creature_template_model WHERE CreatureID=13716')
    connection.execute('INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=13716')
    connection.execute('DELETE FROM content_creature_template_addon WHERE entry=13716')
    connection.execute(f'''INSERT INTO content_creature_template_addon
        SELECT a.entry, {retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=13716''')
    return [13716]


def import_zaetar_spirit(connection):
    """Pinned Theradras JustDied summons12238; actual base0 and Normal1 only."""
    identity = connection.execute('SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=12238').fetchone()
    difficulties = connection.execute('SELECT DifficultyID FROM tdb_creature_template_difficulty WHERE Entry=12238 ORDER BY DifficultyID').fetchall()
    if identity != ('SmartAI','') or difficulties != [(0,), (1,)]:
        raise ValueError('Zaetar spirit12238 source identity changed')
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    connection.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=12238 AND DifficultyID=0')
    connection.execute('DELETE FROM content_creature_template WHERE entry=12238')
    retail_content.insert_tdb_templates(connection,'12238',replace_legacy=True)
    connection.execute('DELETE FROM content_creature_template_model WHERE CreatureID=12238')
    connection.execute('INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=12238')
    connection.execute('DELETE FROM content_creature_template_addon WHERE entry=12238')
    connection.execute(f'''INSERT INTO content_creature_template_addon
        SELECT a.entry, {retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=12238''')
    project_difficulty(connection, 12238, 1)
    return [12238]


def import_fiendish_hound(connection):
    """Omor30707 summons17540; retain authored base0, not an invented Normal1."""
    identity = connection.execute('SELECT AIName,ScriptName FROM tdb_creature_template WHERE entry=17540').fetchone()
    difficulties = connection.execute('SELECT DifficultyID FROM tdb_creature_template_difficulty WHERE Entry=17540 ORDER BY DifficultyID').fetchall()
    effect = connection.execute('SELECT Effect,EffectMiscValue_0,EffectMiscValue_1 FROM spell_effect WHERE SpellID=30707 AND EffectIndex=0 AND DifficultyID=0').fetchone()
    if identity != ('SmartAI','') or difficulties != [(0,), (2,)] or effect != (28,17540,64):
        raise ValueError('Fiendish Hound17540 source identity changed')
    connection.create_function('health_expansion',1,retail_content.health_expansion,deterministic=True)
    connection.execute('DROP TABLE IF EXISTS temp.tdb_difficulty')
    connection.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE Entry=17540 AND DifficultyID=0')
    connection.execute('DELETE FROM content_creature_template WHERE entry=17540')
    retail_content.insert_tdb_templates(connection,'17540',replace_legacy=True)
    connection.execute('DELETE FROM content_creature_template_model WHERE CreatureID=17540')
    connection.execute('INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model WHERE CreatureID=17540')
    connection.execute('DELETE FROM content_creature_template_addon WHERE entry=17540')
    connection.execute(f'''INSERT INTO content_creature_template_addon
        SELECT a.entry, {retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a WHERE a.entry=17540''')
    return [17540]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--encounter',choices=['kystia','golden-serpent','adarogg','wailing-ritual','noxxion','celebras','theradras','omor'],default='kystia')
    parser.add_argument('--db',type=Path,required=True)
    args=parser.parse_args()
    canonical=Path('/home/osso/Projects/world-of-osso/game-engine/data/server-world/world.db')
    if args.db.name == 'world.db' or args.db.resolve() == canonical.resolve():
        parser.error('use an independent database copy, not canonical world.db')
    if args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('copy must be an existing independent regular file')
    with closing(sqlite3.connect(args.db)) as conn:
        with conn:
            if args.encounter == 'golden-serpent':
                entries = [import_animated_gold_template(conn), import_shadow_of_zul_template(conn)]
            elif args.encounter == 'adarogg':
                entries = [import_demonic_leap_template(conn)]
            elif args.encounter == 'omor':
                entries = import_fiendish_hound(conn)
            elif args.encounter == 'theradras':
                entries = import_zaetar_spirit(conn)
            elif args.encounter == 'celebras':
                entries = import_celebras_redeemed(conn)
            elif args.encounter == 'noxxion':
                entries = import_noxxion_spawn_difficulty(conn)
            elif args.encounter == 'wailing-ritual':
                entries = import_wailing_ritual_difficulties(conn)
            else:
                entries = [import_mirror_template(conn)]
    print(f'Prepared sourced encounter summon entries:{entries}; no placement inserted')


if __name__ == '__main__':
    main()
