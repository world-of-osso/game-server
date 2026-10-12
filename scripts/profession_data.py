#!/usr/bin/env python3
"""Import pinned Retail profession reference tables into an explicit staging DB.

The column names describe the DBD layout, not an inferred operation evaluator.
Unknown DBD semantics remain unknown; importing curves does not specify their inputs.
"""
import argparse
from contextlib import closing
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import sqlite3

import db2_casc
from crafting_data import BUILD, Table, decode_csv, validate_rows


@dataclass(frozen=True)
class SourceTable:
    pin: Table
    record_count: int
    columns: tuple[tuple[str, str], ...]


def columns(integer_names, real_names=()):
    """Preserve declared field order; float32 bits are decoded, never integer-cast."""
    return tuple((name, 'REAL' if name in real_names else 'INTEGER') for name in integer_names.split())


TABLES = (
    SourceTable(Table('crafting_difficulty', '4373536.db2', 0x7744A1A6, 27,
        '7743b941dd08ebc1cc4db86a4d5646d913a83d3765cfca1ccc84df68e8c7e284'), 26,
        columns('ID MaxRandomSkillBonusPercent CraftSkillBonusPercent ReCraftSkillBonusPercent InspirationSkillBonusPercent Field_10_0_0_44649_004 ConcentrationSkillCurveID ConcentrationDifficultyCurveID',
            ('MaxRandomSkillBonusPercent', 'CraftSkillBonusPercent', 'ReCraftSkillBonusPercent', 'InspirationSkillBonusPercent', 'Field_10_0_0_44649_004'))),
    SourceTable(Table('crafting_difficulty_quality', '4373537.db2', 0xE2C031D4, 91,
        'bf1b2cc9e970b45f1767272a91e5ca68325871193580ef367021bfe7ed6d16bc'), 91,
        columns('ID Order CraftingQualityID QualityPercentage Field_10_0_0_44895_004 CraftingDifficultyID',
            ('QualityPercentage', 'Field_10_0_0_44895_004'))),
    SourceTable(Table('crafting_reagent_quality', '4382197.db2', 0xBFED1EE1, 1403,
        '9250c1daf0233ba2894d1a4458d500ca70be7fe0a644ac397e154e7bfb0483a8'), 1403,
        columns('ID OrderIndex ItemID CurrencyTypesID MaxDifficultyAdjustment ReagentEffectPct Field_12_0_0_64124_006 ModifiedCraftingCategoryID',
            ('MaxDifficultyAdjustment', 'ReagentEffectPct'))),
    SourceTable(Table('crafting_reagent_effect', '4541803.db2', 0x70597398, 104,
        'd9c404adceaae317fce0e8f5cdf8c4727560e61c546c2b741ed9b64f8f8a03da'), 104,
        columns('ID ProfessionEffectID Order ModifiedCraftingCategoryID')),
    SourceTable(Table('profession', '4508544.db2', 0x86FE926D, 15,
        '119513ac11404057df11c35f6d2e057b357fbc1f5ee6ed2ba05f170c863447c4'), 15,
        columns('ID SkillLineID ProfessionEnumValue ResourcefulEffectLow ResourcefulEffectHigh MinCraftExtraQtyPct MaxCraftExtraQtyPct GatherTimeIncreasePct ActionTypeEnumValue RecraftModSkillGainPct PublicOrderCapacityCurrencyID',
            ('ResourcefulEffectLow', 'ResourcefulEffectHigh', 'MinCraftExtraQtyPct', 'MaxCraftExtraQtyPct', 'GatherTimeIncreasePct', 'RecraftModSkillGainPct'))),
    SourceTable(Table('prof_trait_path_node', '4542069.db2', 0x8622E546, 915,
        '19257e17683eb6414aa4b59a17d6d5672d9fdc48c7023d5cd8ff29adbd0380db'), 915,
        columns('ID TraitNodeID LeftTraitNodeGroupID RightTraitNodeGroupID TraitCurrencyID')),
    SourceTable(Table('prof_trait_perk_node', '4542084.db2', 0x741F7E26, 5929,
        '7c5906d4ba39a694415d1128d18c8bd2add3bcbfa0baf0c29f91ccd9cc4a9b5e'), 5929,
        columns('ID TraitNodeID RequiredPointsInThisNode Flags')),
    SourceTable(Table('prof_trait_tree', '4542085.db2', 0xD2C4C5BD, 138,
        '354c45ff848f5d6db768b312689bc60662745e7b2a73d2f68f3e35fd4e91f2b5'), 138,
        columns('ID TraitTreeID')),
    SourceTable(Table('crafting_order', '4529122.db2', 0x05F8699D, 2047,
        '1667dd0855db6d5c7790a452e7bf0114bdc577b531e1e419e933247d8766b0ce'), 2047,
        columns('ID SkillLineAbilityID CraftingOrderHouseID Field_10_0_2_46479_002')),
    SourceTable(Table('currency_types', 'CurrencyTypes.csv', None, 1523,
        'e29a73b88b0da1e9846ff453411d5d8a274986f64ab3223bc80bc12e68c9f61a'), 1523,
        (('ID', 'INTEGER'), ('Name_lang', 'TEXT'), ('Description_lang', 'TEXT'),
         ('InventoryIconFileID', 'INTEGER'), ('MaxQty', 'INTEGER'),
         ('MaxEarnablePerWeek', 'INTEGER'), ('AwardConditionID', 'INTEGER'),
         ('MaxQtyWorldStateID', 'INTEGER'), ('Flags_0', 'INTEGER'), ('Flags_1', 'INTEGER'))),
    SourceTable(Table('profession_trait', '4505298.db2', 0x306455FE, 6993,
        '2cdf52fe511e2a84ac63bad3ff6d28abd25394446ae6122db2c1d6284466a002'), 6993,
        columns('ID TraitDefinitionID')),
    SourceTable(Table('profession_trait_x_effect', '4505494.db2', 0x97A0A286, 6001,
        '549e49cc7225bae399aa351333c8efe2f81c0ba2e1fafd21341a3e3cf0334f86'), 6001,
        columns('ID ProfessionTraitID ProfessionEffectID Field_10_0_0_44649_003')),
    SourceTable(Table('profession_effect', '4505297.db2', 0x04C16FA9, 382,
        '52e95370f604f53f41b377ab125df931e9d2df0ae2c4e6be408a38e6bf95fd25'), 249,
        columns('ID ProfessionEffectTypeEnumID Amount ModifiedCraftingReagentSlotID', ('Amount',))),
    SourceTable(Table('profession_effect_type', '4556715.db2', 0x39C33588, 31,
        'f7f084e015edc86f8543502a3d502b2f699d5980adca8cc931fd97a975e7f6f0'), 31,
        (('ID', 'INTEGER'), ('Name_lang', 'TEXT'), ('EnumID', 'INTEGER'))),
)


def decode_source(table, path):
    raw = path.read_bytes()
    pin = table.pin
    digest = hashlib.sha256(raw).hexdigest()
    if digest != pin.sha256:
        raise ValueError(f'{path}: SHA-256 {digest}, expected {pin.sha256}')
    if pin.layout is None:
        rows = decode_csv(pin, raw, table.columns)
        return validate_rows(pin, rows, table.columns)
    header = db2_casc.read_header(raw)
    if header['layout_hash'] != pin.layout or header['record_count'] != table.record_count:
        raise ValueError(f'{path}: incompatible layout or record count')
    text_columns = frozenset(i for i, (_, kind) in enumerate(table.columns) if kind == 'TEXT')
    _, decoded = db2_casc.decode(raw, text_columns)
    if len(decoded) != pin.count:
        raise ValueError(f'{path}: decoded {len(decoded)} rows, expected {pin.count}')
    rows = []
    for record_id, values in sorted(decoded.items()):
        if pin.name == 'profession_trait_x_effect':
            # DBD97A0A286 stores the parent inline and in the relationship map.
            if len(values) != len(table.columns) + 1 or values[-1] != values[1]:
                raise ValueError(f'{path}: ID {record_id} has inconsistent relationship parent')
            values = values[:-1]
        if len(values) != len(table.columns) or values[0] != record_id:
            raise ValueError(f'{path}: ID {record_id} has invalid record width or identity')
        rows.append([db2_casc.typed(value, kind) for value, (_, kind) in zip(values, table.columns)])
    return validate_rows(pin, rows, table.columns)


def load_sources(source_dir):
    loaded, report = {}, []
    for table in TABLES:
        pin = table.pin
        loaded[pin.name] = decode_source(table, Path(source_dir) / pin.filename)
        report.append({'table': pin.name, 'source': pin.filename, 'build': BUILD,
            'layout': f'{pin.layout:08X}' if pin.layout is not None else [name for name, _ in table.columns],
            'records': table.record_count, 'rows': len(loaded[pin.name]), 'sha256': pin.sha256,
            'source_kind': 'local-casc' if pin.layout is not None else 'local-casc-export'})
    return loaded, report


def create_statement(table):
    fields = ', '.join(f'"{name}" {kind} ' + ('PRIMARY KEY' if name == 'ID' else 'NOT NULL') for name, kind in table.columns)
    return f'CREATE TABLE "{table.pin.name}" ({fields})'


def import_data(database, source_dir):
    loaded, report = load_sources(source_dir)
    with closing(sqlite3.connect(database)) as db, db:
        db.execute('BEGIN IMMEDIATE')
        for table in TABLES:
            name = table.pin.name
            db.execute(f'DROP TABLE IF EXISTS "{name}"')
            db.execute(create_statement(table))
            parameters = ','.join('?' for _ in table.columns)
            db.executemany(f'INSERT INTO "{name}" VALUES ({parameters})', loaded[name])
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--db', type=Path, help='explicit staging database; no implicit world.db')
    mode.add_argument('--check', action='store_true', help='validate without opening a database')
    parser.add_argument('--source-dir', type=Path, required=True)
    args = parser.parse_args()
    try:
        report = load_sources(args.source_dir)[1] if args.check else import_data(args.db, args.source_dir)
    except (OSError, ValueError, sqlite3.Error) as error:
        parser.exit(1, f'profession reference import failed: {error}\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
