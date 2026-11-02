"""Read spell roots from the follower Journal and WCL equipment pools."""
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DB2 = ROOT / 'data/db2/active'
WCL = ROOT / 'scripts/reference/follower-wcl-candidates.json'


def read_journal_items(db2, maps):
    def rows(name):
        with (db2 / name).open(newline='') as file:
            return list(csv.DictReader(file))

    instances = {int(row['ID']) for row in rows('JournalInstance.csv')
                 if int(row['MapID']) in maps}
    encounters = {int(row['ID']) for row in rows('JournalEncounter.csv')
                  if int(row['JournalInstanceID']) in instances}
    return {int(row['ItemID']) for row in rows('JournalEncounterItem.csv')
            if int(row['JournalEncounterID']) in encounters
            and int(row['DisplaySeasonID']) == 0
            and int(row['WorldStateExpressionID']) == 0}


def read_wcl_items(path):
    catalog = json.loads(path.read_text())
    items, gems, enchants, bonuses = set(), set(), set(), set()
    for profile in catalog['profiles'].values():
        for item in profile['gear']:
            items.add(int(item['id']))
            gems.update(int(gem['id']) for gem in item.get('gems', ()))
            for key in ('bonusIDs', 'sourceBonusIDs'):
                bonuses.update(int(bonus) for bonus in item.get(key, ()))
            for key in ('permanentEnchant', 'temporaryEnchant'):
                if item.get(key):
                    enchants.add(int(item[key]))
    return items, gems, enchants, bonuses


def gear_spell_roots(conn, db2=DB2, wcl=WCL):
    """Conservatively include effects, set bonuses, gems and enchants of gear candidates."""
    maps = {row[0] for row in conn.execute(
        'SELECT map_id FROM follower_lfg_dungeons WHERE difficulty_id=205')}
    items, gems, enchants, bonuses = read_wcl_items(wcl)
    items.update(read_journal_items(db2, maps))
    if not maps or not items:
        raise ValueError('follower Journal or WCL equipment roots are empty')
    conn.execute('DROP TABLE IF EXISTS temp.follower_summon_item')
    conn.execute('CREATE TEMP TABLE follower_summon_item (id INTEGER PRIMARY KEY)')
    conn.executemany('INSERT INTO temp.follower_summon_item VALUES (?)', ((item,) for item in items | gems))
    conn.execute('DROP TABLE IF EXISTS temp.follower_summon_enchant')
    conn.execute('CREATE TEMP TABLE follower_summon_enchant (id INTEGER PRIMARY KEY)')
    conn.executemany('INSERT INTO temp.follower_summon_enchant VALUES (?)', ((enchant,) for enchant in enchants))
    conn.execute('DROP TABLE IF EXISTS temp.follower_summon_bonus')
    conn.execute('CREATE TEMP TABLE follower_summon_bonus (id INTEGER PRIMARY KEY)')
    conn.executemany('INSERT INTO temp.follower_summon_bonus VALUES (?)', ((bonus,) for bonus in bonuses))
    return {row[0] for row in conn.execute('''
        WITH enchant_ids(id) AS (
            SELECT id FROM temp.follower_summon_enchant
            UNION SELECT g.Enchant_ID FROM temp.follower_summon_item i
              JOIN item_sparse s ON s.ID=i.id JOIN gem_properties g ON g.ID=s.Gem_properties
        ), roots(id) AS (
            SELECT e.SpellID FROM temp.follower_summon_item i
              JOIN item_x_item_effect x ON x.ItemID=i.id
              JOIN item_effect e ON e.ID=x.ItemEffectID WHERE e.SpellID>0
            UNION SELECT b.SpellID FROM temp.follower_summon_item i
              JOIN item_sparse s ON s.ID=i.id JOIN item_set_spell b ON b.ItemSetID=s.ItemSet
              WHERE s.ItemSet>0 AND b.SpellID>0
            UNION SELECT e.SpellID FROM temp.follower_summon_bonus i
              JOIN item_bonus b ON b.ParentItemBonusListID=i.id AND b.Type=23
              JOIN item_effect e ON e.ID=b.Value_0 WHERE e.SpellID>0
            UNION SELECT e.EffectArg_0 FROM enchant_ids i JOIN spell_item_enchantment e ON e.ID=i.id
              WHERE e.Effect_0=3 AND e.EffectArg_0>0
            UNION SELECT e.EffectArg_1 FROM enchant_ids i JOIN spell_item_enchantment e ON e.ID=i.id
              WHERE e.Effect_1=3 AND e.EffectArg_1>0
            UNION SELECT e.EffectArg_2 FROM enchant_ids i JOIN spell_item_enchantment e ON e.ID=i.id
              WHERE e.Effect_2=3 AND e.EffectArg_2>0
        ) SELECT id FROM roots WHERE id>0
    ''')}
