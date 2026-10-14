#!/usr/bin/env python3
"""Compile supported pinned difficulty, reference-player and native event-fact trees."""
import argparse
import collections
import hashlib
from pathlib import Path

from import_achievement_single_modifiers import read_rows

LEGACY_DUNGEON_DIFFICULTY = 20
DUNGEON_DIFFICULTY = 68
DIFFICULTY_TYPES = {LEGACY_DUNGEON_DIFFICULTY, DUNGEON_DIFFICULTY}
# TC CriteriaHandler.cpp ModifierTreeType leaves read from the reference player alone:
# PlayerMeetsCondition2 (1768; bounded native identity conditions only),
# PlayerHasAura8 (1798), PlayerIsInArea17 (1842), PlayerRace25 (1886), PlayerClass26
# (1890), LessThanTappers29 (1902), PlayerMap32 (1922), PlayerLevelEqualOrGreaterThan69 (2094),
# PlayerLevelEqualOrLessThan71 (2102).
PLAYER_FACTS = {2, 8, 17, 25, 26, 29, 32, 41, 69, 71}
# HasTimeEventPassed289 (CriteriaHandler.cpp3578-3611): TC hard-codes these event
# timestamps; every other event defaults to the current time, so it has always passed.
HAS_TIME_EVENT_PASSED = 289
TIMESTAMPED_TIME_EVENTS = {111, 120, 121, 349, 350}
CONDITIONS = ('StartEvent', 'StartTimer', 'FailEvent', 'EligibilityWorldStateID')
# Structural (CompleteAchievement8) or account-recomputed (81/156/185/186) types are
# never evaluated at a player's criteria update, so no condition can gate them.
UNGATED_TYPES = {8, 81, 156, 185, 186}
SINGLE_TRUE, SINGLE_FALSE, ALL, SOME = 2, 3, 4, 8


def legacy_difficulties(difficulty_path):
    """Difficulty.db2 OldEnumValue -> Difficulty IDs (TC CriteriaHandler.cpp1860-1865)."""
    legacy = collections.defaultdict(set)
    for row_id, row in read_rows(difficulty_path).items():
        if int(row['OldEnumValue']) >= 0:
            legacy[int(row['OldEnumValue'])].add(row_id)
    return dict(legacy)


def load_identity_condition_ids(db):
    """Mirror game_objects/identity_conditions.rs's bounded native loader, not general PC2."""
    identity = {'ID', 'Flags', 'ClassMask', 'RaceMasks_0', 'RaceMasks_1', 'Failure_description_lang'}
    unset = {'Gender', 'NativeGender', 'MinExpansionLevel', 'MaxExpansionLevel',
             'ChrSpecializationIndex', 'ChrSpecializationRole', 'PowerType',
             'MaxExpansionTier', 'MinExpansionTier'}
    extra = {row[0] for row in db.execute(
        'SELECT SourceEntry FROM tdb_conditions WHERE SourceTypeOrReferenceId=34')}
    query = db.execute('SELECT * FROM player_condition')
    columns = [column[0] for column in query.description]
    supported = set()
    for values in query:
        row = dict(zip(columns, values))
        if row['ID'] in extra or row['Flags'] & ~1:
            continue
        if all(value == (-1 if name in unset else 0)
               for name, value in row.items() if name not in identity):
            supported.add(row['ID'])
    return supported


def plain(node):
    return not int(node['SecondaryAsset']) and not int(node['TertiaryAsset'])


def difficulty_ids(node, legacy):
    """IDs a plain DungeonDifficulty68 / LegacyDungeonDifficulty20 leaf names."""
    asset = int(node['Asset'])
    if int(node['Type']) == DUNGEON_DIFFICULTY:
        return frozenset({asset})
    return frozenset(legacy.get(asset, ()))


def some_difficulties(root, modifiers, children, legacy):
    """TC Some8 needing max(Amount,1)==1 of SingleTrue difficulty leaves: their union."""
    node = modifiers[root]
    if int(node['Operator']) != SOME or max(int(node['Amount']), 1) != 1:
        return None
    if (int(node['Type']), int(node['Asset'])) != (0, 0) or not plain(node) or not children[root]:
        return None
    ids = frozenset()
    for child in children[root]:
        leaf = modifiers[child]
        if children[child] or int(leaf['Operator']) != SINGLE_TRUE or int(leaf['Type']) not in DIFFICULTY_TYPES or not plain(leaf):
            return None
        ids |= difficulty_ids(leaf, legacy)
    return ids


def some_leaf_alternatives(root, modifiers, children, kind):
    """Plain Some requiring one of multiple complete same-kind positive leaves."""
    parent = modifiers[root]
    if int(parent['Operator']) != SOME:
        return None
    if max(int(parent['Amount']), 1) != 1 or len(children[root]) < 2:
        return None
    assets = []
    for child in children[root]:
        leaf = modifiers[child]
        if (children[child] or int(leaf['Operator']) != SINGLE_TRUE
                or int(leaf['Type']) != kind or int(leaf['Asset']) <= 0 or not plain(leaf)):
            return None
        assets.append(int(leaf['Asset']))
    return tuple(sorted(assets))


def tree_conditions(root, modifiers, children, legacy, criteria_type=0):
    """(difficulty rule or None, player facts) of a supported tree, else None.

    A difficulty rule is (allowed?, IDs). All4 children must all hold: positive
    difficulty sets intersect, SingleFalse sets are excluded.
    """
    parent = modifiers[root]
    if (int(parent['Type']), int(parent['Asset'])) != (0, 0) or not plain(parent):
        return None
    auras = some_leaf_alternatives(root, modifiers, children, 8)
    if auras is not None:
        return None, tuple((8, spell) for spell in auras)
    areas = some_leaf_alternatives(root, modifiers, children, 17)
    if areas is not None:
        return None, tuple((17, area) for area in areas)
    maps = some_leaf_alternatives(root, modifiers, children, 32)
    if maps is not None:
        return None, tuple((32, map_id) for map_id in maps)
    targets = some_leaf_alternatives(root, modifiers, children, 4) if criteria_type == 54 else None
    if targets is not None:
        return None, tuple((4, target) for target in targets)
    some = some_difficulties(root, modifiers, children, legacy)
    if some is not None:
        return ((True, tuple(sorted(some))), ()) if some else None
    single_some = (int(parent['Operator']) == SOME
                   and max(int(parent['Amount']), 1) == 1
                   and len(children[root]) == 1)
    if int(parent['Operator']) != ALL and not single_some:
        return None
    # TC Some1738–1750 with one required/one child has the same child truth as All.
    # TC All1733–1737 returns true when its child list is empty.
    allowed, excluded, facts = None, frozenset(), []
    for child in children[root]:
        node = modifiers[child]
        if children[child]:
            ids = some_difficulties(child, modifiers, children, legacy)
            if ids is None:
                return None
            allowed = ids if allowed is None else allowed & ids
            continue
        kind, operator = int(node['Type']), int(node['Operator'])
        if not plain(node):
            return None
        if kind in DIFFICULTY_TYPES and operator == SINGLE_TRUE:
            ids = difficulty_ids(node, legacy)
            allowed = ids if allowed is None else allowed & ids
        elif kind in DIFFICULTY_TYPES and operator == SINGLE_FALSE:
            excluded |= difficulty_ids(node, legacy)
        elif kind in PLAYER_FACTS and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            facts.append((kind, int(node['Asset'])))
        elif kind == 39 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC PlayerLevelEqual1956–1959 is exactly the existing inclusive bounds.
            level = int(node['Asset'])
            facts.extend(((69, level), (71, level)))
        elif kind == 32 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            # TC PlayerMap1922–1925: map0 is a real map, not missing authority.
            facts.append((kind, 0))
        elif kind == 35 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            facts.append((kind, 0))
        elif kind == 122 and operator == SINGLE_TRUE and 0 <= int(node['Asset']) <= 127:
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 54 and kind == 4 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC TargetCreatureId4 needs the unit snapshotted by each text-emote request.
            facts.append((kind, int(node['Asset'])))
        elif operator == SINGLE_TRUE and (
                (kind == 24 and int(node['Asset']) in (2, 3)) or
                (kind == 60 and int(node['Asset']) == 0)):
            facts.append((kind, int(node['Asset'])))
        elif kind == HAS_TIME_EVENT_PASSED and operator == SINGLE_TRUE and int(node['Asset']) not in TIMESTAMPED_TIME_EVENTS:
            continue  # Always holds; no runtime fact.
        else:
            return None
    if allowed is not None:
        allowed -= excluded
        if not allowed:
            return None
        rule = (True, tuple(sorted(allowed)))
    else:
        rule = (False, tuple(sorted(excluded))) if excluded else None
    return rule, tuple(sorted(facts))


def qualified_conditions(criteria, modifiers, legacy):
    """Criteria ID -> (modifier root, difficulty rule or None, player facts)."""
    children = collections.defaultdict(list)
    for row_id, row in modifiers.items():
        children[int(row['Parent'])].append(row_id)
    qualified = {}
    for row_id, row in criteria.items():
        root = int(row['ModifierTreeId'])
        if int(row['Type']) in UNGATED_TYPES or root not in modifiers or any(int(row[key]) for key in CONDITIONS):
            continue
        compiled = tree_conditions(root, modifiers, children, legacy, int(row['Type']))
        if compiled:
            qualified[row_id] = (root, *compiled)
    return qualified


def generate_source(criteria_path, modifiers_path, difficulty_path, output):
    legacy = legacy_difficulties(difficulty_path)
    criteria, modifiers = read_rows(criteria_path), read_rows(modifiers_path)
    qualified = qualified_conditions(criteria, modifiers, legacy)
    lines = [
        '// Generated by scripts/import_achievement_conditions.py; do not edit.',
        '// TC CriteriaHandler.cpp1723-1751 operators over difficulty (20/68) and',
        '// player-only (2/8/17/25/26/29/32/41/69/71; exact39 lowers to69+71), emote-only TargetCreatureId4,',
        '// including plain Some requiring one of complete TargetCreatureId4, PlayerMap32, PlayerIsInArea17 or PlayerHasAura8 alternatives,',
        '// known-solo35, native-arena player (24/60), map instance type122, and always-passed HasTimeEventPassed289 leaves.',
        f'// Criteria SHA256 {hashlib.sha256(criteria_path.read_bytes()).hexdigest()}.',
        f'// ModifierTree SHA256 {hashlib.sha256(modifiers_path.read_bytes()).hexdigest()}.',
        f'// Difficulty SHA256 {hashlib.sha256(difficulty_path.read_bytes()).hexdigest()}.',
        'use super::{CompiledConditions, DifficultyRule};',
        '',
        'pub(super) fn criteria_conditions(criteria: u32, modifier: u32) -> Option<CompiledConditions> {',
        '    match (criteria, modifier) {',
    ]
    for criterion, (modifier, rule, facts) in sorted(qualified.items()):
        if rule:
            ids = ', '.join(str(difficulty) for difficulty in rule[1])
            difficulty = f'Some(DifficultyRule {{ allowed: {str(rule[0]).lower()}, ids: &[{ids}] }})'
        else:
            difficulty = 'None'
        alternative_kind = facts[0][0] if facts else None
        variant = {4: 'TargetCreatures', 32: 'PlayerMaps', 17: 'PlayerAreas',
                   8: 'PlayerAuras'}.get(alternative_kind)
        complete_some = (int(modifiers[modifier]['Operator']) == SOME
                         and len(facts) > 1
                         and all(kind == alternative_kind for kind, _ in facts))
        if complete_some and variant is not None:
            assets = ', '.join(str(asset) for _, asset in facts)
            compiled = f'CompiledConditions::{variant}(&[{assets}])'
        else:
            listed = ', '.join(f'({kind}, {asset})' for kind, asset in facts)
            compiled = f'CompiledConditions::All {{ difficulty: {difficulty}, facts: &[{listed}] }}'
        lines.append(f'        ({criterion}, {modifier}) => Some({compiled}),')
    lines += ['        _ => None,', '    }', '}', '']
    output.write_text('\n'.join(lines))
    print(f'Compiled {len(qualified)} condition predicates.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--criteria', type=Path, required=True)
    parser.add_argument('--modifiers', type=Path, required=True)
    parser.add_argument('--difficulty', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    generate_source(args.criteria, args.modifiers, args.difficulty, args.output)


if __name__ == '__main__':
    main()
