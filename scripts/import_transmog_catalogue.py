#!/usr/bin/env python3
"""Embed authentic Retail69933 transmog CSV exports, without touching world.db.

Inputs are produced by game-engine/scripts/export_db2_csv.py from local CASC.
Output is compiled into the server; regenerate explicitly when changing builds.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path

TABLES = ('TransmogSet', 'TransmogSetItem', 'TransmogIllusion',
          'TransmogOutfitEntry', 'TransmogOutfitSlotInfo', 'TransmogOutfitSlotOption',
          'SpellItemEnchantment')


def load_rows(directory, table):
    with (directory / f'{table}.csv').open() as stream:
        return list(csv.DictReader(stream))


def build_catalogue(directory):
    tables = {table: load_rows(directory, table) for table in TABLES}
    membership = {}
    for row in tables['TransmogSetItem']:
        membership.setdefault(int(row['TransmogSetID']), set()).add(int(row['ItemModifiedAppearanceID']))
    sets = [{'set_id': int(row['ID']), 'parent_set_id': int(row['ParentTransmogSetID'] or 0),
             'name': row['Name_lang'], 'source_ids': sorted(membership.get(int(row['ID']), set())),
             'class_mask': int(row['ClassMask']), 'condition_id': int(row['ConditionID'])}
            for row in tables['TransmogSet']]
    entries = [{'id':int(row['ID']), 'name':row['Name_lang'], 'order_index':int(row['OrderIndex']),
                'cost':int(row['Cost']), 'source':int(row['Source']), 'flags':int(row['Flags']),
                'set_type':int(row['SetType'])} for row in tables['TransmogOutfitEntry']]
    # Illusion name/icon come from SpellItemEnchantment (C_TransmogCollection.GetIllusionInfo).
    enchantments = {int(row['ID']): row for row in tables['SpellItemEnchantment']}
    illusions = [{'id':int(row['ID']), 'enchantment_id':int(row['SpellItemEnchantmentID']),
                  'name':enchantments[int(row['SpellItemEnchantmentID'])]['Name_lang'],
                  'icon_fdid':int(enchantments[int(row['SpellItemEnchantmentID'])]['IconFileDataID']),
                  'cost':int(row['TransmogCost']), 'unlock_condition_id':int(row['UnlockConditionID']),
                  'flags':int(row['Flags']), 'collected':False} for row in tables['TransmogIllusion']]
    slots = [{'id':int(row['ID']), 'inventory_slot_id':int(row['InventorySlotID']),
              'flags':int(row['Flags']), 'collection_type':int(row['TransmogCollectionType'])}
             for row in tables['TransmogOutfitSlotInfo']]
    options = [{'id':int(row['ID']), 'slot_info_id':int(row['TransmogOutfitSlotInfoID']),
                'name':row['Name_lang'], 'option':int(row['OptionEnum']), 'flags':int(row['Flags'])}
               for row in tables['TransmogOutfitSlotOption']]
    return {'build':'12.1.0.69933', 'sha256':{table:hashlib.sha256((directory / f'{table}.csv').read_bytes()).hexdigest() for table in TABLES},
            'sets':sets, 'outfit_entries':entries, 'illusions':illusions, 'slots':slots, 'outfit_options':options}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--csv-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = build_catalogue(args.csv_dir)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')
    print({key:len(result[key]) for key in ('sets', 'outfit_entries', 'illusions', 'slots', 'outfit_options')})


if __name__ == '__main__':
    main()
