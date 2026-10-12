#!/usr/bin/env python3
"""Admit Kystia's sourced summoned mirror template into an independent COPY.

No placements, guessed stats, loot or spell schedules are inserted. Lead applies
this script to a reviewed copy. Sources: local TDB1210 template255050, models and
explicit difficulty1 row; DB2 Mirror Image1264098 effect0 references255050.
"""
import argparse
import sqlite3
from contextlib import closing
from pathlib import Path

import retail_content

MIRROR = 255050


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
    connection.execute('''INSERT OR REPLACE INTO content_creature_template_difficulty
        (entry,DifficultyID,minlevel,maxlevel,exp,ContentTuningID,HealthScalingExpansion,
         LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,ArmorModifier,
         DamageModifier,lootid,mingold,maxgold)
        SELECT d.Entry,d.DifficultyID,l.minlevel,l.maxlevel,health_expansion(d.HealthScalingExpansion),
            d.ContentTuningID,d.HealthScalingExpansion,d.LevelScalingDeltaMin,d.LevelScalingDeltaMax,
            d.HealthModifier,d.ManaModifier,d.ArmorModifier,d.DamageModifier,d.LootID,d.GoldMin,d.GoldMax
        FROM tdb_creature_template_difficulty d JOIN content_tuning_level l ON l.ID=d.ContentTuningID
        WHERE d.Entry=? AND d.DifficultyID=1''',(MIRROR,))
    if connection.execute('SELECT count(*) FROM content_creature_template_difficulty WHERE entry=? AND DifficultyID=1',(MIRROR,)).fetchone()[0] != 1:
        raise ValueError('Missing authored mirror content tuning')
    return MIRROR


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db',type=Path,required=True)
    args=parser.parse_args()
    canonical=Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
    if args.db.name == 'world.db' or args.db.resolve() == canonical.resolve():
        parser.error('use an independent database copy, not canonical world.db')
    if args.db.is_symlink() or not args.db.is_file() or args.db.stat().st_nlink != 1:
        parser.error('copy must be an existing independent regular file')
    with closing(sqlite3.connect(args.db)) as conn:
        with conn:
            mirror=import_mirror_template(conn)
    print(f'Imported sourced mirror template/model/difficulty:{mirror}; no placement inserted')


if __name__ == '__main__':
    main()
