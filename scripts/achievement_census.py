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
import db2_casc

BASE_TYPES = {0, 1, 5, 7, 8, 9, 11, 12, 14, 15, 16, 17, 20, 23, 26, 27, 28, 29, 34, 36, 40, 41, 42, 43, 46, 48, 57, 59, 62, 63, 65, 67, 68, 69, 73, 74, 75, 76, 77, 81, 96, 107, 108, 109, 110, 112, 130, 151, 152, 156, 157, 158, 160, 162, 165, 185, 186, 216, 219, 220, 245, 246, 260, 262}
CONDITIONS = ('ModifierTreeId', 'StartEvent', 'StartTimer', 'FailEvent', 'EligibilityWorldStateID')


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


def supported_criteria(criteria, types, spells, skills, mythic_ids, overlay_assets, pet_creatures, toy_items, scenarios, zones, taxi_nodes):
    supported = set()
    for row_id, row in criteria.items():
        kind, asset = int(row['Type']), int(row['Asset'])
        if kind not in types:
            continue
        if kind == 216:
            if row_id in mythic_ids:
                supported.add(row_id)
            continue
        if any(int(row[key]) for key in CONDITIONS):
            continue
        if kind in (28, 29, 34, 69, 110) and asset not in spells:
            continue
        if kind == 26 and asset not in (0, 1, 2, 6):
            continue  # Native fatigue/drowning/falling/void only, no lava/slime/fire producer.
        if kind == 20 and row_id == 111356:
            continue  # Ordinary61917 contradicts pinned TC's statistic-only type20 completion.
        if kind == 41 and 11134 <= row_id <= 11140:
            continue  # TDB1210 orphan aura66303 and holiday404 requirements.
        if kind == 29 and 11118 <= row_id <= 11127:
            continue  # TDB1210 holiday404 restrictions, independent of ModifierTree.
        if kind in (9, 14, 17, 23, 48, 59, 62, 63, 65, 67, 74, 76, 77, 81, 107, 108, 130, 151, 156, 157, 158, 186, 219, 220, 245) and asset != 0:
            continue
        if kind == 43 and asset not in overlay_assets:
            continue
        if kind in (160, 162) and not 1 <= asset <= 25:
            continue
        if kind == 185 and asset not in toy_items:
            continue
        if kind == 152 and asset not in scenarios:
            continue
        if kind == 11 and asset not in zones:
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


def census(db2, modifiers_path, world_db, types, overlays_path):
    achievements = read_rows(db2 / 'Achievement.csv')
    criteria = read_rows(db2 / 'Criteria.csv')
    trees = read_rows(db2 / 'CriteriaTree.csv')
    modifiers = read_rows(modifiers_path)
    children, modifier_children = index_children(trees), index_children(modifiers)
    with sqlite3.connect(f'{world_db.resolve().as_uri()}?mode=ro', uri=True) as db:
        spells = {row[0] for row in db.execute('SELECT ID FROM spell')}
        skills = {row[0] for row in db.execute('SELECT ID FROM skill_line')}
        pet_creatures = {row[0] for row in db.execute('SELECT creature_id FROM battle_pet_species WHERE creature_id != 0')}
        toy_items = {row[0] for row in db.execute('SELECT ItemID FROM toy')}
        areas = {row[0] for row in db.execute('SELECT ID FROM area_table WHERE ID != 0 AND AreaBit >= 0')}
        taxi_nodes = {row[0] for row in db.execute('SELECT ID FROM taxi_nodes')}
        scenarios = {row[0] for row in db.execute('SELECT ID FROM scenario')}
        zones = {row[0] for row in db.execute('SELECT ID FROM area_table')}
    header, overlays = db2_casc.decode(overlays_path.read_bytes())
    if header['layout_hash'] != 0x9A196494:
        raise ValueError('requires pinned WorldMapOverlay layout9A196494')
    overlay_assets = {row_id for row_id, row in overlays.items() if any(area in areas for area in row[12:16])}
    mythic_ids = qualified_mythic_criteria(criteria, modifiers)
    supported = supported_criteria(criteria, types, spells, skills, mythic_ids, overlay_assets, pet_creatures, toy_items, scenarios, zones, taxi_nodes)
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
        newly_supported = supported_criteria(criteria, types | {kind}, spells, skills, mythic_ids, overlay_assets, pet_creatures, toy_items, scenarios, zones, taxi_nodes)
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
    paths = [db2 / (name + '.csv') for name in ('Achievement', 'CriteriaTree', 'Criteria')]
    paths.extend([modifiers_path, overlays_path])
    return dict(
        build=db2.name, enabled_types=sorted(types),
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
