#!/usr/bin/env python3
"""Pinned Mainline achievement coverage; never opens the world database writable."""
import argparse
import collections
import csv
import hashlib
import json
import sqlite3
from pathlib import Path

from import_achievement_modifiers import qualified_mythic_criteria
from import_achievement_single_modifiers import qualified_single_modifiers
from import_achievement_conditions import load_identity_condition_ids, legacy_difficulties, qualified_conditions
import db2_casc
from import_achievement_node_counts import qualified_constant_expressions, qualified_node_counts

BASE_TYPES = {0, 1, 5, 7, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18, 20, 23, 24, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 39, 40, 41, 42, 43, 46, 47, 48, 52, 53, 54, 55, 56, 57, 59, 62, 63, 65, 66, 67, 68, 69, 70, 73, 74, 75, 76, 77, 78, 80, 81, 82, 83, 84, 85, 86, 87, 88, 90, 91, 96, 97, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 112, 113, 114, 130, 151, 152, 155, 156, 157, 158, 160, 162, 163, 164, 165, 185, 186, 196, 216, 219, 220, 221, 225, 245, 246, 253, 260, 262}
CONDITIONS = ('ModifierTreeId', 'StartEvent', 'StartTimer', 'FailEvent', 'EligibilityWorldStateID')
# shared battleground_columns.rs PvpStat columns credited by native objectives.
MAX_LEVEL = 90  # crate::experience::MAX_LEVEL
# crate::race_class::PLAYABLE_RACES IDs.
PLAYABLE_RACES = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 52, 70, 84, 85, 86, 91, 95, 96}
NATIVE_PVP_STATS = {61, 63, 64, 65, 82, 183, 290, 291, 370, 371, 418, 419, 428, 744, 926, 927, 928, 929, 1020, 1021}


def read_rows(path):
    with path.open() as source:
        return {int(row['ID']): row for row in csv.DictReader(source)}


def index_children(rows):
    children = collections.defaultdict(list)
    for row_id, row in rows.items():
        children[int(row['Parent'])].append(row_id)
    return children


def walk_tree(root, rows, children, visiting=frozenset()):
    if root in visiting or root not in rows:
        return set(), {f'missing_or_cyclic_tree:{root}'}
    descendants, errors = {root}, set()
    for child in children.get(root, []):
        found, problems = walk_tree(child, rows, children, visiting | {root})
        descendants.update(found)
        errors.update(problems)
    return descendants, errors


def supported_criteria(criteria, types, spells, skills, mythic_ids, overlay_assets, pet_creatures, pet_species, toy_items, scenarios, zones, taxi_nodes, single_modifiers, condition_trees):
    supported = set()
    for row_id, row in criteria.items():
        kind, asset = int(row['Type']), int(row['Asset'])
        if kind not in types:
            continue
        if kind == 216:
            if row_id in mythic_ids:
                supported.add(row_id)
            continue
        if kind == 221 and row_id in single_modifiers:
            supported.add(row_id)
            continue
        if kind == 155 or (kind in (160, 162) and row_id in single_modifiers):
            level_valid = kind == 155 or 1 <= asset <= 25
            if level_valid and row_id in single_modifiers and single_modifiers[row_id][1] in pet_species:
                supported.add(row_id)
            continue
        # A compiled difficulty (20/68) or holdable player-fact tree may be the only condition.
        if any(int(row[key]) for key in CONDITIONS) and row_id not in condition_trees:
            continue
        if kind in (28, 29, 34, 69, 110) and asset not in spells:
            continue
        if kind == 30 and asset not in NATIVE_PVP_STATS:
            continue
        if kind in (32, 33, 97) and asset <= 0:
            continue  # TC requires exact map ID, not a zero/all-arena alias.
        if kind == 54 and asset not in (34, 101, 86, 87, 59, 225, 56):
            continue  # TC TextEmotes for the six native emotes, not animation IDs.
        if kind == 26 and asset not in (0, 1, 2, 6):
            continue  # Native fatigue/drowning/falling/void only, no lava/slime/fire producer.
        if kind == 91 and row_id in (41244, 45924):
            continue  # Pinned TC makes91 statistic-only; current ordinary12927/13707 contradict it.
        if kind == 20 and row_id == 111356:
            continue  # Ordinary61917 contradicts pinned TC's statistic-only type20 completion.
        if kind == 41 and 11134 <= row_id <= 11140:
            continue  # TDB1210 orphan aura66303 and holiday404 requirements.
        if kind == 29 and 11118 <= row_id <= 11127:
            continue  # TDB1210 holiday404 restrictions, independent of ModifierTree.
        if kind in (90, 91, 24, 35, 113, 37, 9, 13, 101, 102, 103, 14, 17, 23, 48, 55, 104, 105, 106, 59, 62, 63, 65, 66, 67, 74, 76, 77, 80, 81, 82, 83, 84, 85, 86, 107, 108, 114, 130, 151, 156, 157, 158, 186, 219, 220, 245, 221, 253, 47, 87, 88, 56, 78) and asset != 0:
            continue
        if kind == 31 and (asset <= 0 or asset not in zones):
            continue  # Recipient terrain area with authored ancestry; no all-area alias0.
        if kind == 52 and asset not in range(1, 14):
            continue  # Eligible native victim's exact positive authored class.
        if kind == 53 and asset not in PLAYABLE_RACES:
            continue  # Eligible native victim's positive authored playable race, not scoreboard identity.
        if kind == 39 and asset not in (0, 1):
            continue  # Current authored2v2/3v3; retired5v5/RBG/Shuffle/Blitz are not native rating sources.
        if kind == 43 and asset not in overlay_assets:
            continue
        if kind == 196 and not 1 <= asset <= 90:
            continue  # Native pinned gtXp cap; no level500 producer.
        if kind in (160, 162) and not 1 <= asset <= 25:
            continue
        if kind == 185 and asset not in toy_items:
            continue
        if kind == 152 and asset not in scenarios:
            continue
        if kind in (11, 163, 164, 225) and asset not in zones:
            continue
        if kind == 262 and asset not in taxi_nodes:
            continue
        if kind == 109 and asset not in (1, 4, 6, 9):
            continue  # Loot types the server produces; no fishing3.
        if kind == 246 and asset not in (0, 1, 2):
            continue  # Native Public/Guild/Personal orders; no NPC order producer.
        if kind == 260 and asset not in (0, 2):
            continue  # TC counts Character0 and Account2 bank tabs only.
        if kind == 96 and asset not in pet_creatures:
            continue
        if kind in (7, 40, 75, 112) and asset not in skills:
            continue
        supported.add(row_id)
    return supported


def fact_can_hold(kind, asset, spells, zones, maps, identities, map_types, authored_facts, achievement_ids, node_counts, pet_types):
    """Mirror of the catalog loader: a fact naming unknown data can never hold."""
    if kind in authored_facts:
        return asset in authored_facts[kind]
    if kind in (87, -87):
        return asset in achievement_ids
    if kind == 67:
        return asset in node_counts
    if kind == 21:
        return asset == 0
    if kind == 30:
        return 1 <= asset <= 15
    if kind == 78:
        return asset in pet_types
    return {10: asset in spells, 46: 1 <= asset <= 100, 5: asset == 0, 6: asset == 0, 7: asset == 0, 28: 1 <= asset <= 13, 34: 1 <= asset <= 25, 104: asset in spells, -104: asset in spells, 16: asset == 0, 2: asset in identities, 4: asset > 0, 24: asset in (2, 3), 29: asset > 0, 60: asset == 0, 8: asset in spells, -8: asset in spells, 17: asset in zones, 25: asset in PLAYABLE_RACES, 27: asset in PLAYABLE_RACES, 26: 1 <= asset <= 13,
            32: asset in maps, 35: asset == 0, 41: asset in zones, 122: asset in map_types, 69: 1 <= asset <= MAX_LEVEL, 71: asset >= 1}[kind]


def supports_tree(root, trees, children, supported, visiting=frozenset()):
    if root in visiting or root not in trees:
        return False
    row = trees[root]
    operator = int(row['Operator'])
    if operator == 0:
        return int(row['CriteriaID']) in supported
    branch = children.get(root, [])
    return operator in (4, 5, 6, 7, 8, 9) and bool(branch) and all(
        supports_tree(child, trees, children, supported, visiting | {root}) for child in branch
    )


def raw_type_inventory(criteria, trees, records, supported):
    by_type = collections.defaultdict(set)
    for row_id, row in criteria.items():
        by_type[int(row['Type'])].add(row_id)
    tree_rows = {int(row['CriteriaID']) for row in trees.values()} - {0}
    root_rows = {criterion for record in records.values() for criterion in record['criteria']}
    return {
        kind: dict(
            raw_rows=len(ids), supported_raw_rows=len(ids & supported),
            tree_referenced_rows=len(ids & tree_rows),
            achievement_root_rows=len(ids & root_rows),
            supported_achievement_root_rows=len(ids & supported & root_rows),
            supported_outside_achievement_roots=len((ids & supported) - root_rows),
        )
        for kind, ids in sorted(by_type.items())
    }


def census(db2, modifiers_path, world_db, types, overlays_path):
    achievements = read_rows(db2 / 'Achievement.csv')
    criteria = read_rows(db2 / 'Criteria.csv')
    trees = read_rows(db2 / 'CriteriaTree.csv')
    modifiers = read_rows(modifiers_path)
    expressions = read_rows(db2 / 'WorldStateExpression.csv')
    literal_expressions = qualified_constant_expressions(expressions)
    node_counts = set(qualified_node_counts(expressions))
    node_counts.update(expression for expression, value in literal_expressions.items() if value)
    children, modifier_children = index_children(trees), index_children(modifiers)
    with sqlite3.connect(f'{world_db.resolve().as_uri()}?mode=ro', uri=True) as db:
        spells = {row[0] for row in db.execute('SELECT ID FROM spell')}
        skills = {row[0] for row in db.execute('SELECT ID FROM skill_line')}
        pet_creatures = {row[0] for row in db.execute('SELECT creature_id FROM battle_pet_species WHERE creature_id != 0')}
        pet_species = {row[0] for row in db.execute('SELECT species_id FROM battle_pet_species')}
        pet_types = {row[0] for row in db.execute(
            'SELECT DISTINCT m.PetTypeEnum FROM battle_pet_species s '
            'JOIN battle_pet_species_metadata m ON m.ID=s.species_id WHERE m.PetTypeEnum BETWEEN 0 AND 9')}
        toy_items = {row[0] for row in db.execute('SELECT ItemID FROM toy')}
        areas = {row[0] for row in db.execute('SELECT ID FROM area_table WHERE ID != 0 AND AreaBit >= 0')}
        taxi_nodes = {row[0] for row in db.execute('SELECT ID FROM taxi_nodes')}
        scenarios = {row[0] for row in db.execute('SELECT ID FROM scenario')}
        zones = {row[0] for row in db.execute('SELECT ID FROM area_table')}
        maps = {row[0] for row in db.execute('SELECT ID FROM map')}
        map_types = {row[0] for row in db.execute('SELECT DISTINCT InstanceType FROM map')}
        identities = load_identity_condition_ids(db)
        item_rows = dict(db.execute('SELECT ID, Quality FROM content_item'))
        qualities = set(item_rows.values())
        tuning_ranges = {row[0] for row in db.execute(
            'SELECT ID FROM content_tuning_level WHERE minlevel BETWEEN 1 AND 255 '
            'AND maxlevel BETWEEN minlevel AND 255')}
        authored_facts = {268: tuning_ranges, 19: set(item_rows), 15: qualities,
                          14: set(range(max(qualities, default=-1) + 1)),
                          110: {row[0] for row in db.execute('SELECT ID FROM content_quest_template')},
                          82: {row[0] + 1 for row in db.execute('SELECT DISTINCT OrderIndex FROM scenario_step WHERE OrderIndex >= 0')}}
    header, overlays = db2_casc.decode(overlays_path.read_bytes())
    if header['layout_hash'] != 0x9A196494:
        raise ValueError('requires pinned WorldMapOverlay layout9A196494')
    overlay_assets = {row_id for row_id, row in overlays.items() if any(area in areas for area in row[12:16])}
    mythic_ids = qualified_mythic_criteria(criteria, modifiers)
    single_modifiers = qualified_single_modifiers(criteria, modifiers)
    condition_trees = {
        row_id for row_id, (_, _, facts) in qualified_conditions(criteria, modifiers, legacy_difficulties(db2 / 'Difficulty.csv'), literal_expressions).items()
        if row_id not in single_modifiers
        and all(fact_can_hold(kind, asset, spells, zones, maps, identities, map_types, authored_facts, achievements, node_counts, pet_types) for kind, asset in facts)
    }
    supported = supported_criteria(criteria, types, spells, skills, mythic_ids, overlay_assets, pet_creatures, pet_species, toy_items, scenarios, zones, taxi_nodes, single_modifiers, condition_trees)
    by_type = collections.defaultdict(set)
    records = {}
    for row_id, row in achievements.items():
        root = int(row['CriteriaTree'])
        nodes, errors = walk_tree(root, trees, children) if root else (set(), {'no_root'})
        criterion_ids = {int(trees[node]['CriteriaID']) for node in nodes} - {0}
        kinds, modifier_ids, modifier_types = set(), set(), set()
        for criterion in criterion_ids:
            if criterion not in criteria:
                errors.add(f'missing_criterion:{criterion}')
                continue
            cr = criteria[criterion]
            kind = int(cr['Type'])
            kinds.add(kind)
            by_type[kind].add(row_id)
            if criterion not in supported:
                errors.add(f'unsupported_criterion:{criterion}:type:{kind}')
            modifier = int(cr['ModifierTreeId'])
            if modifier:
                found, problems = walk_tree(modifier, modifiers, modifier_children)
                modifier_ids.update(found)
                modifier_types.update(int(modifiers[node]['Type']) for node in found)
                errors.update(problems)
        structural = supports_tree(root, trees, children, supported)
        flags = int(row['Flags'])
        if flags & 0x4000:
            errors.add('actor:guild_requires_guild_progress')
        records[row_id] = dict(
            title=row['Title_lang'], root=root, flags=flags,
            types=sorted(kinds), criteria=sorted(criterion_ids), trees=sorted(nodes),
            modifiers=sorted(modifier_ids), modifier_types=sorted(modifier_types),
            structurally_evaluable=structural, trackable=structural and not flags & 0x4000,
            blockers=sorted(errors),
        )
    trackable = [row for row in records.values() if row['trackable']]
    ranking = []
    for kind, ids in sorted(by_type.items(), key=lambda item: (-len(item[1]), item[0])):
        newly_supported = supported_criteria(criteria, types | {kind}, spells, skills, mythic_ids, overlay_assets, pet_creatures, pet_species, toy_items, scenarios, zones, taxi_nodes, single_modifiers, condition_trees)
        gain = sum(not records[id]['trackable'] and not records[id]['flags'] & (1 | 0x4000)
                   and supports_tree(records[id]['root'], trees, children, newly_supported)
                   for id in ids)
        ranking.append(dict(type=kind, associated=len(ids),
                            achievements=sum(not records[id]['flags'] & 1 for id in ids),
                            statistics=sum(bool(records[id]['flags'] & 1) for id in ids),
                            event_type_enabled=kind in types,
                            player_trackable=sum(records[id]['trackable'] and not records[id]['flags'] & 1 for id in ids),
                            blocked_achievements=sum(not records[id]['trackable'] and not records[id]['flags'] & 1 for id in ids),
                            potential_gain=gain))
    ranking.sort(key=lambda row: (-row['achievements'], -row['statistics'], row['type']))
    paths = [db2 / (name + '.csv') for name in ('Achievement', 'CriteriaTree', 'Criteria', 'Difficulty')]
    paths.extend([modifiers_path, overlays_path, db2 / 'WorldStateExpression.csv'])
    return dict(
        build=db2.name, enabled_types=sorted(types),
        raw_types=raw_type_inventory(criteria, trees, records, supported),
        sources={str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths},
        row_counts=dict(achievement=len(achievements), criteria=len(criteria),
                        criteria_tree=len(trees), modifier_tree=len(modifiers)),
        guild_structurally_evaluable=sum(row['structurally_evaluable'] and bool(row['flags'] & 0x4000) and not row['flags'] & 1 for row in records.values()),
        trackable=dict(achievements=sum(not row['flags'] & 1 for row in trackable),
                       account=sum(not row['flags'] & 1 and bool(row['flags'] & 0x20000) for row in trackable),
                       character=sum(not row['flags'] & (1 | 0x20000) for row in trackable),
                       statistics=sum(bool(row['flags'] & 1) for row in trackable)),
        ranking=ranking, achievements=records,
        boundary='Structural, all-faction union; not playable-content or modifier support certification.',
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db2', type=Path, required=True)
    parser.add_argument('--modifiers', type=Path, required=True)
    parser.add_argument('--world-db', type=Path, required=True)
    parser.add_argument('--overlays-db2', type=Path, required=True)
    parser.add_argument('--types', default=','.join(map(str, sorted(BASE_TYPES))))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = census(args.db2, args.modifiers, args.world_db, {int(x) for x in args.types.split(',')}, args.overlays_db2)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ('row_counts', 'trackable', 'boundary')}))
    for row in result['ranking'][:20]:
        print(json.dumps(row))


if __name__ == '__main__':
    main()
