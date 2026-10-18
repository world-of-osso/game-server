"""Produce a COPY of world.db with native starter-spellbook follower priorities.

First run follower_kit_export_native_source (ignored, explicit output) through
agent-run. That exporter uses the production starter_purchases/build_config
validator: TraitCurrencySource budgets, grants, TraitCond gates and native order.
No talent-point, spell-ID, timer or coefficient is invented here. Only priorities
and condition thresholds are named project policies (follower-dungeons.md).
"""
import argparse
from collections import defaultdict
from contextlib import closing
import csv
import hashlib
import json
from pathlib import Path
import re
import sqlite3
import subprocess

BUILD = '12.1.5.70448'
POLICY = 'FOLLOWER_BATTLEFIELD_PRIORITY_V1'
SOURCE_TABLES = {
    'SpecializationSpells': 'specialization_spells', 'SpellLevels': 'spell_levels',
    'SkillLineAbility': 'skill_line_ability', 'TraitDefinition': 'trait_definition',
    'TraitTreeLoadout': 'trait_tree_loadout', 'TraitTreeLoadoutEntry': 'trait_tree_loadout_entry',
    'TraitCurrency': 'trait_currency', 'TraitCurrencySource': 'trait_currency_source',
    'TraitCond': 'trait_cond', 'TraitNode': 'trait_node', 'TraitNodeEntry': 'trait_node_entry',
    'TraitNodeXTraitNodeEntry': 'trait_node_x_trait_node_entry',
}
SCHEMA = '''CREATE TABLE follower_spec_kit (
    spec_id INTEGER NOT NULL, spell_id INTEGER NOT NULL,
    min_level INTEGER NOT NULL CHECK(min_level BETWEEN 1 AND 90),
    max_level INTEGER NOT NULL CHECK(max_level BETWEEN min_level AND 90),
    role_slot TEXT NOT NULL, priority INTEGER NOT NULL,
    conditions TEXT NOT NULL, policy TEXT NOT NULL,
    PRIMARY KEY(spec_id,spell_id,min_level)
)'''


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def policy(slot, priority, target='enemy', **conditions):
    return slot, priority, dict(target=target, **conditions)


def combat_policy(spec, name, spell, dispel_effects, control_effects):
    """Editable project decisions; names resolve only from the sourced spellbook."""
    if dispel_effects and name in {'Cleanse Toxins', "Nature's Cure", 'Remove Corruption', 'Purify', 'Purify Disease', 'Remove Curse', 'Expunge', 'Cauterizing Flame', 'Dispel Magic', 'Purge', 'Soothe'}:
        return policy('dispel', 42, 'party')
    if control_effects and name in {'Polymorph', 'Hammer of Justice', 'Leg Sweep', 'Paralysis', 'Fear', 'Hex'}:
        area = any(15 in targets or 16 in targets for targets in control_effects)
        source = any(15 in targets for targets in control_effects)
        return policy('control', 43, requires_cast=True, min_enemies=1 if area else 0, source_center=source)
    emergency = {'Lay on Hands', 'Guardian Spirit'}
    if name in emergency:
        return policy('heal', 10, 'party', health_below=0.20)
    if name in {'Word of Glory', 'Holy Word: Serenity', 'Swiftmend', 'Impending Victory', 'Exhilaration', 'Expel Harm', 'Verdant Embrace'}:
        target = 'self' if name in {'Impending Victory', 'Exhilaration', 'Expel Harm'} else 'party'
        return policy('heal', 20, target, health_below=0.40)
    if name == 'Ironbark':
        return policy('defensive', 25, 'party', health_below=0.50, attacked=True, missing_aura=spell)
    if name in {'Divine Protection', 'Ardent Defender', 'Guardian of Ancient Kings', 'Shield Wall', 'Barkskin', 'Desperate Prayer', 'Astral Shift', 'Ice Block', 'Blazing Barrier', 'Survival of the Fittest', 'Aspect of the Turtle', 'Obsidian Scales', 'Touch of Karma', 'Fortifying Brew', 'Unending Resolve', 'Dark Pact', 'Survival Instincts'}:
        return policy('defensive', 30, 'self', health_below=0.50, attacked=True, missing_aura=spell)
    if name in {'Hand of Reckoning', 'Taunt'} and spec in {66, 73}:
        return policy('taunt', 35, lost_threat=True)
    if name in {'Shield of the Righteous', 'Shield Block', 'Ignore Pain', 'Demoralizing Shout'}:
        return policy('mitigation', 40, 'enemy' if name == 'Shield of the Righteous' else 'self',
                      attacked=True, missing_aura=spell,
                      min_enemies=1 if name == 'Shield of the Righteous' else 0,
                      source_center=name == 'Shield of the Righteous')
    if spec in {105, 257}:
        if name == 'Lifebloom':
            return policy('hot', 45, 'tank', health_below=0.95, missing_aura=spell, exclusive_aura=True)
        if name in {'Rejuvenation', 'Renew', 'Prayer of Mending'}:
            return policy('hot', 50, 'party', health_below=0.90, missing_aura=spell)
        if name in {'Regrowth', 'Flash Heal', 'Heal'}:
            return policy('heal', 65, 'party', health_below=0.80)
        if name in {'Wild Growth', 'Prayer of Healing', 'Holy Word: Sanctify', 'Tranquility', 'Divine Hymn', 'Efflorescence'}:
            return policy('heal', 55, 'party', health_below=0.70, min_injured=3)
    if name in {'Healing Surge', 'Flash of Light', 'Regrowth', 'Vivify', 'Emerald Blossom', 'Drain Life'}:
        return policy('heal', 70, 'self' if name == 'Drain Life' else 'party', health_below=0.35)
    if name in {'Counterspell', 'Rebuke', 'Pummel', 'Wind Shear', 'Counter Shot', 'Quell', 'Spear Hand Strike', 'Skull Bash'}:
        return policy('interrupt', 5)
    if name in {'Mark of the Wild', 'Arcane Intellect', 'Power Word: Fortitude', 'Battle Shout', 'Blessing of the Bronze', 'Skyfury', 'Lightning Shield', 'Defensive Stance'}:
        return policy('buff', 75, 'self', missing_aura=spell)
    if name in {'Avenging Wrath', 'Combustion', 'Dragonrage', 'Zenith', 'Bestial Wrath', "Tiger's Fury", 'Stormkeeper', 'Summon Demonic Tyrant', 'Divine Toll', 'Mirror Image'}:
        return policy('buff', 80, 'self', combat=True, missing_aura=spell)
    if name in {'Consecration', 'Earthquake', 'Flamestrike', 'Meteor'}:
        return policy('aoe', 85, 'self' if name == 'Consecration' else 'enemy', min_enemies=1 if spec == 66 else 3, ground=True)
    if name in {'Thunder Clap', 'Revenge', 'Whirlwind', 'Chain Lightning', 'Spinning Crane Kick', 'Azure Strike', 'Pyre', 'Swipe', 'Primal Wrath', 'Thrash', 'Holy Nova', 'Arcane Explosion'}:
        return policy('aoe', 90, min_enemies=1 if name == 'Thunder Clap' else 3,
                      source_center=name not in {'Chain Lightning', 'Azure Strike', 'Pyre'})
    if name in {'Moonfire', 'Sunfire', 'Flame Shock', 'Holy Fire', 'Barbed Shot', 'Rake', 'Rip', 'Corruption', 'Curse of Weakness', "Hunter's Mark"}:
        return policy('attack', 100, missing_aura=spell)
    if name in {"Avenger's Shield", 'Shield Slam', 'Lava Burst', 'Pyroblast', 'Fire Blast', 'Kill Command', 'Wild Thrash', 'Disintegrate', 'Fire Breath', 'Eternity Surge', 'Fists of Fury', 'Rising Sun Kick', 'Strike of the Windlord', 'Blackout Kick', 'Ferocious Bite', 'Demonbolt', 'Call Dreadstalkers', 'Summon Vilefiend', 'Demonic Strength', 'Execute', 'Touch of Death', 'Earth Shock'}:
        return policy('attack', 110)
    if name in {'Judgment', 'Crusader Strike', 'Blessed Hammer', 'Devastate', 'Wrath', 'Smite', 'Fireball', 'Lightning Bolt', 'Cobra Shot', 'Steady Shot', 'Living Flame', 'Tiger Palm', 'Shadow Bolt', 'Shred'}:
        return policy('attack', 150)
    return policy('utility', 1000)


def read_sources(db2, db):
    if db2.resolve().name != BUILD:
        raise ValueError(f'refusing DB2 build other than {BUILD}: {db2.resolve()}')
    evidence = {}
    for source, table in SOURCE_TABLES.items():
        path = db2 / (source + '.csv')
        with path.open() as stream:
            reader = csv.DictReader(stream)
            fields, count = reader.fieldnames, 0
            query = 'SELECT ' + ','.join('"' + field + '"' for field in fields) + f' FROM {table} WHERE ID=?'
            for row in reader:
                stored = db.execute(query, (row['ID'],)).fetchone()
                # Only referenced source rows are mandatory; encrypted data remains absent.
                if stored and any(str(value) != row[field] for field, value in zip(fields, stored)):
                    raise ValueError(f'{source} row {row["ID"]} differs from installed world.db')
                count += 1
        evidence[source] = dict(path=str(path.resolve()), sha256=digest(path), rows=count)
    return evidence


def selected_source_rows(db2, source):
    """Literal DB2 rows behind selected loadouts, currencies and definitions."""
    def read(name):
        with (db2 / (name + '.csv')).open() as stream:
            return list(csv.DictReader(stream))
    specs = {str(row['spec_id']) for row in source['loadouts']}
    configs = [entry for row in source['loadouts'] for entry in row['config']]
    nodes = {str(entry['node_id']) for entry in configs}
    entries = {str(entry['entry_id']) for entry in configs}
    spells = {spell for row in source['loadouts'] for spell in row['known']}
    currencies = {str(currency['id']) for tree in source['trees'] for currency in tree['tree']['currencies']}
    loadouts = [row for row in read('TraitTreeLoadout') if row['ChrSpecializationID'] in specs]
    loadout_ids = {row['ID'] for row in loadouts}
    node_entries = [row for row in read('TraitNodeEntry') if row['ID'] in entries]
    definitions = {row['TraitDefinitionID'] for row in node_entries}
    return {
        'TraitTreeLoadout': loadouts,
        'TraitTreeLoadoutEntry': [row for row in read('TraitTreeLoadoutEntry') if row['TraitTreeLoadoutID'] in loadout_ids],
        'TraitNode': [row for row in read('TraitNode') if row['ID'] in nodes],
        'TraitNodeEntry': node_entries,
        'TraitDefinition': [row for row in read('TraitDefinition') if row['ID'] in definitions],
        'TraitCurrency': [row for row in read('TraitCurrency') if row['ID'] in currencies],
        'TraitCurrencySource': [row for row in read('TraitCurrencySource') if row['TraitCurrencyID'] in currencies],
        'SpecializationSpells': [row for row in read('SpecializationSpells') if row['SpecID'] in specs],
        'SkillLineAbility': [row for row in read('SkillLineAbility') if row['Spell'] in spells],
        'SpellLevels': [row for row in read('SpellLevels') if row['SpellID'] in spells],
    }


def read_periodic_description_child(db, spell, name):
    """Read the unique native same-name damage aura named by a dummy root."""
    description = db.execute('SELECT Description_lang FROM spell WHERE ID=?', (spell,)).fetchone()
    if not description:
        raise ValueError(f'{name} {spell}: native description missing')
    description = description[0]
    fields = ['ID', 'SpellID', 'DifficultyID', 'Effect', 'EffectAura', 'EffectTriggerSpell']
    columns = ','.join(fields)
    root_rows = [dict(zip(fields, row)) for row in db.execute(
        f'SELECT {columns} FROM spell_effect WHERE SpellID=? AND DifficultyID=0 ORDER BY ID', (spell,))]
    if not root_rows or any(row['EffectTriggerSpell'] for row in root_rows):
        raise ValueError(f'{name} {spell}: expected native dummy root without trigger edge')
    candidates = []
    for child in sorted({int(value) for value in re.findall(r'\$(\d+)', description)}):
        child_name = db.execute('SELECT Name_lang FROM spell_name WHERE ID=?', (child,)).fetchone()
        if child_name != (name,):
            continue
        periodic = [dict(zip(fields, row)) for row in db.execute(
            f'SELECT {columns} FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND Effect=6 AND EffectAura=3 ORDER BY ID', (child,))]
        if periodic:
            candidates.append((child, periodic))
    if len(candidates) != 1:
        raise ValueError(f'{name} {spell}: description must reference exactly one native same-name periodic damage aura; found {len(candidates)}')
    child, periodic = candidates[0]
    return dict(spell_id=spell, name=name, child_spell_id=child, description=description,
                root_effect_rows=root_rows, child_periodic_effect_rows=periodic,
                fact='ROOT_CHILD_LINK_DESCRIPTION_ONLY')


def read_description_reference_sources(db2, references):
    """Verify the actual description/name/effect fields against pinned DB2 CSVs."""
    expected = {
        'Spell': {row['spell_id']: {'ID': row['spell_id'], 'Description_lang': row['description']} for row in references},
        'SpellName': {spell: {'ID': spell, 'Name_lang': row['name']} for row in references for spell in [row['spell_id'], row['child_spell_id']]},
        'SpellEffect': {effect['ID']: effect for row in references for key in ['root_effect_rows', 'child_periodic_effect_rows'] for effect in row[key]},
    }
    evidence = {}
    for name, records in expected.items():
        path = db2 / (name + '.csv')
        with path.open() as stream:
            selected = [row for row in csv.DictReader(stream) if int(row['ID']) in records]
        if len(selected) != len(records):
            raise ValueError(f'{name}: description-reference source rows missing')
        for row in selected:
            if any(str(value) != row[field] for field, value in records[int(row['ID'])].items()):
                raise ValueError(f'{name} {row["ID"]}: description-reference source differs from world.db')
        evidence[name] = dict(path=str(path.resolve()), sha256=digest(path), selected_rows=selected)
    return evidence


def kit_rows(db, source):
    availability = defaultdict(list)
    description_links = {}
    reference_rows = {}
    slots = {}
    inventory = []
    dispels = defaultdict(list)
    for spell, dispel_type, target in db.execute('SELECT SpellID,EffectMiscValue_0,ImplicitTarget_0 FROM spell_effect WHERE DifficultyID=0 AND Effect=38'):
        dispels[spell].append((dispel_type, target))
    controls = defaultdict(list)
    for spell, target_a, target_b in db.execute('SELECT SpellID,ImplicitTarget_0,ImplicitTarget_1 FROM spell_effect WHERE DifficultyID=0 AND Effect=6 AND EffectAura IN(5,7,12,27,60)'):
        controls[spell].append((target_a, target_b))
    for loadout in source['loadouts']:
        spec, level = loadout['spec_id'], loadout['level']
        for spell_text, passive in loadout['known'].items():
            spell = int(spell_text)
            native = db.execute('SELECT n.Name_lang,COALESCE(l.SpellLevel,0) FROM spell_name n LEFT JOIN spell_levels l ON l.SpellID=n.ID AND l.DifficultyID=0 WHERE n.ID=?', (spell,)).fetchone()
            if not native:
                raise ValueError(f'native spell {spell} absent')
            name, required = native
            if level < required:
                continue
            rule = policy('passive', 0, 'self') if passive else combat_policy(spec, name, spell, dispels[spell], controls[spell])
            if rule[0] == 'dispel':
                effects = dispels[spell]
                rule[2]['dispel_types'] = sorted({row[0] for row in effects})
                rule[2]['target'] = 'enemy' if all(row[1] == 6 for row in effects) else 'party'
            if name in {'Moonfire', 'Sunfire'} and not passive:
                if spell not in description_links:
                    description_links[spell] = read_periodic_description_child(db, spell, name)
                link = description_links[spell]
                rule[2]['missing_aura'] = link['child_spell_id']
                reference_rows[(spec, spell)] = dict(spec_id=spec, **link)
            # SoTR's armor aura is named literally by its source tooltip; the
            # native effect64 children are NOT that armor aura.
            if name == 'Shield of the Righteous' and not passive:
                description = db.execute('SELECT Description_lang FROM spell WHERE ID=?', (spell,)).fetchone()[0]
                references = {int(value) for value in re.findall(r'\$(\d+)[sd]', description)}
                armor = [value for value in references if db.execute(
                    'SELECT 1 FROM spell_name n JOIN spell_effect e ON e.SpellID=n.ID WHERE n.ID=? AND n.Name_lang=? AND e.EffectAura=268', (value, name)).fetchone()]
                if len(armor) != 1:
                    raise ValueError('SoTR tooltip must name exactly one native armor aura')
                rule[2]['missing_aura'] = armor[0]
            elif rule[2].get('missing_aura') == spell and not db.execute(
                    'SELECT 1 FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND Effect=6', (spell,)).fetchone():
                children = db.execute('''SELECT DISTINCT e.EffectTriggerSpell FROM spell_effect e
                    JOIN spell_effect child ON child.SpellID=e.EffectTriggerSpell AND child.DifficultyID=0 AND child.Effect=6
                    JOIN spell_name n ON n.ID=e.EffectTriggerSpell
                    WHERE e.SpellID=? AND e.DifficultyID=0 AND e.Effect=64 AND n.Name_lang=?''', (spell, name)).fetchall()
                if len(children) == 1:
                    rule[2]['missing_aura'] = children[0][0]
            key = spec, spell
            availability[key].append(level)
            slots[key] = rule
            if level == 32:
                inventory.append(dict(spec_id=spec, spell_id=spell, name=name, role_slot=rule[0], conditions=rule[2]))
    rows = []
    for (spec, spell), levels in sorted(availability.items()):
        slot, priority, conditions = slots[spec, spell]
        start = end = levels[0]
        for level in levels[1:] + [None]:
            if level is not None and level == end + 1:
                end = level
                continue
            rows.append((spec, spell, start, end, slot, priority, json.dumps(conditions, sort_keys=True), POLICY))
            start = end = level
    return rows, inventory, list(reference_rows.values())


def prepare(args):
    inputs = [args.world_db.resolve(), args.source_json.resolve()]
    outputs = [args.output_db.resolve(), args.receipt.resolve(), args.sqldiff.resolve()]
    if len(set(outputs)) != 3 or any(path in inputs or path.exists() for path in outputs):
        raise ValueError('refusing existing, overlapping or input output path')
    for path in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
    before = digest(args.world_db)
    source = json.loads(args.source_json.read_text())
    if Path(source['world_db']).resolve() != args.world_db.resolve():
        raise ValueError('source export and input world.db differ')
    if len(source['loadouts']) != 11 * 90:
        raise ValueError('expected native loadouts for all eleven followers, levels1..90')
    with closing(sqlite3.connect(args.world_db.resolve().as_uri() + '?mode=ro', uri=True)) as db:
        evidence = read_sources(args.db2_root, db)
        rows, inventory, references = kit_rows(db, source)
        reference_sources = read_description_reference_sources(args.db2_root, references)
        with closing(sqlite3.connect(args.output_db)) as output:
            db.backup(output)
            output.execute('DROP TABLE IF EXISTS follower_spec_kit')
            output.execute(SCHEMA)
            output.executemany('INSERT INTO follower_spec_kit VALUES (?,?,?,?,?,?,?,?)', rows)
            output.commit()
    diff = subprocess.run(['sqldiff', str(args.world_db), str(args.output_db)], capture_output=True, text=True, check=True).stdout
    args.sqldiff.write_text(diff)
    if digest(args.world_db) != before:
        raise ValueError('input database changed during preparation; discard this copy')
    receipt = dict(build=BUILD, policy=POLICY, producer=str(Path(__file__).resolve()),
                   producer_sha256=digest(__file__), source_database=str(args.world_db.resolve()),
                   source_database_sha256=before, output_database=str(args.output_db.resolve()),
                   output_database_sha256=digest(args.output_db), source_json_sha256=digest(args.source_json),
                   changed_tables=['follower_spec_kit'] if diff.strip() else [], rows=len(rows), sources=evidence,
                   chosen_loadouts=source['loadouts'], trait_source_trees=source['trees'], level32_inventory=inventory,
                   aura_reference_rows=references, description_reference_sources=reference_sources,
                   selected_source_rows=selected_source_rows(args.db2_root, source),
                   sqldiff=str(args.sqldiff.resolve()), sqldiff_sha256=digest(args.sqldiff))
    args.receipt.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    print(f'Prepared {len(rows)} kit intervals in COPY {args.output_db}; canonical unchanged')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['world-db', 'db2-root', 'source-json', 'output-db', 'receipt', 'sqldiff']:
        parser.add_argument('--' + name, type=Path, required=True)
    prepare(parser.parse_args())


if __name__ == '__main__':
    main()
