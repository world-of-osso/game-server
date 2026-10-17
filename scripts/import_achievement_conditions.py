#!/usr/bin/env python3
"""Compile supported pinned difficulty, reference-player and native event-fact trees."""
import argparse
import collections
import hashlib
from pathlib import Path

from import_achievement_single_modifiers import read_rows
from import_achievement_node_counts import qualified_constant_expressions

LEGACY_DUNGEON_DIFFICULTY = 20
DUNGEON_DIFFICULTY = 68
DIFFICULTY_TYPES = {LEGACY_DUNGEON_DIFFICULTY, DUNGEON_DIFFICULTY}
# TC CriteriaHandler.cpp ModifierTreeType leaves read from the reference player alone:
# PlayerMeetsCondition2 (1768; bounded native identity conditions only),
# PlayerHasAura8 (1798), PlayerIsInArea17 (1842), PlayerRace25 (1886), PlayerClass26
# (1890), LessThanTappers29 (1902), PlayerMap32 (1922), EarnedAchievementOnPlayer87 (2200–2204),
# PlayerLevelEqualOrGreaterThan69 (2094),
# PlayerLevelEqualOrLessThan71 (2102), PlayerKnowsSpell104 (2282–2285).
PLAYER_FACTS = {2, 8, 17, 25, 26, 29, 32, 41, 69, 71, 87, 104}
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


def some_leaf_alternatives(root, modifiers, children, kinds):
    """Plain Some requiring one of multiple complete positive leaves of allowed kinds."""
    parent = modifiers[root]
    if int(parent['Operator']) != SOME:
        return None
    if max(int(parent['Amount']), 1) != 1 or len(children[root]) < 2:
        return None
    assets = []
    for child in children[root]:
        leaf = modifiers[child]
        if (children[child] or int(leaf['Operator']) != SINGLE_TRUE
                or int(leaf['Type']) not in kinds or int(leaf['Asset']) <= 0 or not plain(leaf)):
            return None
        assets.append((int(leaf['Type']), int(leaf['Asset'])))
    return tuple(sorted(assets))


def tree_conditions(root, modifiers, children, legacy, criteria_type=0, literal_expressions=()):
    """(difficulty rule or None, player facts) of a supported tree, else None.

    A difficulty rule is (allowed?, IDs). All4 children must all hold: positive
    difficulty sets intersect, SingleFalse sets are excluded.
    """
    parent = modifiers[root]
    if (int(parent['Type']), int(parent['Asset'])) != (0, 0) or not plain(parent):
        return None
    auras = some_leaf_alternatives(root, modifiers, children, {8})
    if auras is not None:
        return None, auras
    locations = some_leaf_alternatives(root, modifiers, children, {17, 41})
    if locations is not None:
        return None, locations
    maps = some_leaf_alternatives(root, modifiers, children, {32})
    if maps is not None:
        return None, maps
    items = some_leaf_alternatives(root, modifiers, children, {19}) if criteria_type in (90, 91) else None
    if items is not None:
        return None, items
    targets = some_leaf_alternatives(root, modifiers, children, {4}) if criteria_type in (54, 110) else None
    if targets is not None:
        return None, targets
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
            if ids is not None:
                allowed = ids if allowed is None else allowed & ids
                continue
            if criteria_type != 165 or int(node['Operator']) != ALL:
                return None
            # TC All1733–1737: every nested conjunction must hold in full.
            nested = tree_conditions(child, modifiers, children, legacy, criteria_type, literal_expressions)
            if nested is None:
                return None
            difficulty, nested_facts = nested
            facts.extend(nested_facts)
            if difficulty is not None:
                positive, values = difficulty
                if positive:
                    ids = frozenset(values)
                    allowed = ids if allowed is None else allowed & ids
                else:
                    excluded |= frozenset(values)
            continue
        kind, operator = int(node['Type']), int(node['Operator'])
        if not plain(node):
            return None
        if kind in DIFFICULTY_TYPES and operator == SINGLE_TRUE:
            ids = difficulty_ids(node, legacy)
            allowed = ids if allowed is None else allowed & ids
        elif kind in DIFFICULTY_TYPES and operator == SINGLE_FALSE:
            excluded |= difficulty_ids(node, legacy)
        elif criteria_type == 0 and kind == 8 and operator == SINGLE_FALSE and int(node['Asset']) > 0:
            # TC SingleFalse1730 / PlayerHasAura1798 reads the actual killer's aura authority.
            facts.append((-8, int(node['Asset'])))
        elif criteria_type == 29 and kind == 87 and operator == SINGLE_FALSE and int(node['Asset']) > 0:
            # TC SingleFalse1730 negates scoped HasAchieved2200; signed compiled kind, not a raw DB2 kind.
            facts.append((-87, int(node['Asset'])))
        elif criteria_type == 15 and kind == 268 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC PlayerLevelWithinContentTuning3476–3485: inclusive min/max, not delta bounds.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 0 and kind == 82 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC PlayerScenarioStep2181–2187 compares authored OrderIndex to Asset-1.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 74 and kind == 110 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC PlayerHasCompletedQuest2313–2315 reads this character's rewarded quest bit.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 74 and kind == 104 and operator == SINGLE_FALSE and int(node['Asset']) > 0:
            # Login waits for complete native spell knowledge before HasSpell can be negated.
            facts.append((-104, int(node['Asset'])))
        elif kind in PLAYER_FACTS and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 54 and kind in (5, 6, 7) and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            # TC TargetIsPlayer/TargetIsDead/TargetIsOppositeFaction1786–1797 on the requested emote unit.
            facts.append((kind, 0))
        elif criteria_type == 110 and kind == 5 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            facts.append((kind, 0))
        elif criteria_type == 110 and kind == 28 and operator == SINGLE_TRUE and 1 <= int(node['Asset']) <= 13:
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 110 and kind == 27 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC TargetRace1894–1896 reads the actual unit passed by Spell::PreprocessSpellHit.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 110 and kind == 10 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC TargetHasAura1806–1809 reads the actual pre-effect hit unit.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 110 and kind == 46 and operator == SINGLE_TRUE and 1 <= int(node['Asset']) <= 100:
            # TC TargetHealthBelowPercent1998–2001 is inclusive and reads the hit unit.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 160 and kind == 78 and operator == SINGLE_TRUE and 0 <= int(node['Asset']) <= 9:
            # TC BattlePetType2157–2164 reads the actual earned-level event species' type.
            facts.append((kind, int(node['Asset'])))
        elif criteria_type == 78 and kind == 21 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            # TC1867–1870: inclusive actor GetLevel >= actual killed unit GetLevel.
            facts.append((kind, 0))
        elif criteria_type == 78 and kind == 30 and operator == SINGLE_TRUE and 1 <= int(node['Asset']) <= 15:
            # TC CreatureType1906–1912 / SharedDefines5016–5032: exact killed NPC type.
            facts.append((kind, int(node['Asset'])))
        elif (kind == 67 and operator == SINGLE_TRUE and int(node['Asset']) > 0
              and (criteria_type in (1, 130) or int(node['Asset']) in literal_expressions)):
            # New native kinds require an exactly decoded literal; node ownership stays win-only.
            facts.append((kind, int(node['Asset'])))
        elif kind == 34 and operator == SINGLE_TRUE and 1 <= int(node['Asset']) <= 25:
            # TC BattlePetTeamLevel1935–1939 reads every equipped journal slot.
            facts.append((kind, int(node['Asset'])))
        elif kind == 39 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC PlayerLevelEqual1956–1959 is exactly the existing inclusive bounds.
            level = int(node['Asset'])
            facts.extend(((69, level), (71, level)))
        elif kind == 32 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            # TC PlayerMap1922–1925: map0 is a real map, not missing authority.
            facts.append((kind, 0))
        elif criteria_type in (90, 91) and operator == SINGLE_TRUE and (
                (kind == 19 and int(node['Asset']) > 0)
                or (kind in (14, 15) and 0 <= int(node['Asset']) <= 8)):
            # TC reads these fields from miscValue1 itemID, never slot or spell ID.
            facts.append((kind, int(node['Asset'])))
        elif kind == 16 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            # TC PlayerIsAlive1838–1841; preserve the actual life-state predicate.
            facts.append((kind, 0))
        elif kind == 35 and operator == SINGLE_TRUE and int(node['Asset']) == 0:
            facts.append((kind, 0))
        elif kind == 122 and operator == SINGLE_TRUE and 0 <= int(node['Asset']) <= 127:
            facts.append((kind, int(node['Asset'])))
        elif criteria_type in (54, 110) and kind == 4 and operator == SINGLE_TRUE and int(node['Asset']) > 0:
            # TC TargetCreatureId4 reads the actual emote target or pre-effect spell hit unit.
            facts.append((kind, int(node['Asset'])))
        elif operator == SINGLE_TRUE and (
                (kind == 24 and int(node['Asset']) in (2, 3)) or
                (kind == 60 and int(node['Asset']) == 0)):
            facts.append((kind, int(node['Asset'])))
        elif kind == HAS_TIME_EVENT_PASSED and operator == SINGLE_TRUE and int(node['Asset']) not in TIMESTAMPED_TIME_EVENTS:
            continue  # Always holds; no runtime fact.
        else:
            return None
    has_target_class = any(kind == 28 for kind, _ in facts)
    has_player_target = any(kind in (5, 27) for kind, _ in facts)
    if has_target_class and not has_player_target:
        return None  # Native NPC class authority is not represented by this producer.
    if allowed is not None:
        allowed -= excluded
        if not allowed:
            return None
        rule = (True, tuple(sorted(allowed)))
    else:
        rule = (False, tuple(sorted(excluded))) if excluded else None
    return rule, tuple(sorted(facts))


def qualified_conditions(criteria, modifiers, legacy, literal_expressions=()):
    """Criteria ID -> (modifier root, difficulty rule or None, player facts)."""
    children = collections.defaultdict(list)
    for row_id, row in modifiers.items():
        children[int(row['Parent'])].append(row_id)
    qualified = {}
    for row_id, row in criteria.items():
        root = int(row['ModifierTreeId'])
        if int(row['Type']) in UNGATED_TYPES or root not in modifiers or any(int(row[key]) for key in CONDITIONS):
            continue
        compiled = tree_conditions(root, modifiers, children, legacy, int(row['Type']), literal_expressions)
        if not compiled:
            continue
        if any(kind == 78 for kind, _ in compiled[1]) and not 1 <= int(row['Asset']) <= 25:
            continue
        qualified[row_id] = (root, *compiled)
    return qualified


def generate_source(criteria_path, modifiers_path, difficulty_path, expressions_path, output):
    legacy = legacy_difficulties(difficulty_path)
    criteria, modifiers = read_rows(criteria_path), read_rows(modifiers_path)
    literals = qualified_constant_expressions(read_rows(expressions_path))
    qualified = qualified_conditions(criteria, modifiers, legacy, literals)
    lines = [
        '// Generated by scripts/import_achievement_conditions.py; do not edit.',
        '// TC CriteriaHandler.cpp1723-1751 operators over difficulty (20/68) and',
        '// player-only (2/8/17/25/26/29/32/34/41/69/71/87/104; exact39 lowers to69+71), emote/native110 TargetCreatureId4,',
        '// including plain Some requiring one of complete TargetCreatureId4, PlayerMap32, PlayerHasAura8 or typed area17/zone41 alternatives,',
        '// native loot/obtain event ItemId19 and item-quality14/15; complete Some ItemId19 alternatives.',
        '// native type110 spell-hit TargetIsPlayer5/TargetHasAura10/TargetRace27/TargetClass28/TargetHealthBelowPercent46; class requires player/race authority.',
        '// Native DoEmote54 retains complete positive TargetIsPlayer5/TargetIsDead6/TargetIsOppositeFaction7 companions on the requested unit.',
        '// alive16, known-solo35, native-arena player (24/60), map instance type122, and always-passed HasTimeEventPassed289 leaves.',
        '// Native KillCreature0 SingleFalse8, CastSpell29 SingleFalse87 and Login74 SingleFalse104 use signed compiled kinds; raw DB2 kinds stay unchanged.',
        '// Native earned-level160 BattlePetType78 resolves actual changed-pet species;0 is Humanoid.',
        '// Native kill78 CreatureType30 and positive level-delta21/0 preserve the actual eligible victim type/level.',
        '// Native participation15 PlayerLevelWithinContentTuning268 uses authored inclusive min/max.',
        '// Positive WorldStateExpression67 is load-bounded to exact literal comparisons; native battleground-win1/130 also admits occupied-node equality OR.',
        f'// Criteria SHA256 {hashlib.sha256(criteria_path.read_bytes()).hexdigest()}.',
        f'// ModifierTree SHA256 {hashlib.sha256(modifiers_path.read_bytes()).hexdigest()}.',
        f'// Difficulty SHA256 {hashlib.sha256(difficulty_path.read_bytes()).hexdigest()}.',
        f'// WorldStateExpression SHA256 {hashlib.sha256(expressions_path.read_bytes()).hexdigest()}.',
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
        variant = {4: 'TargetCreatures', 32: 'PlayerMaps',
                   8: 'PlayerAuras', 19: 'EventItems'}.get(alternative_kind)
        some_alternatives = int(modifiers[modifier]['Operator']) == SOME and len(facts) > 1
        location_alternatives = some_alternatives and all(kind in (17, 41) for kind, _ in facts)
        complete_some = some_alternatives and all(kind == alternative_kind for kind, _ in facts)
        if location_alternatives:
            leaves = ', '.join(f'({kind}, {asset})' for kind, asset in facts)
            compiled = f'CompiledConditions::PlayerLocations(&[{leaves}])'
        elif complete_some and variant is not None:
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
    parser.add_argument('--expressions', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    generate_source(args.criteria, args.modifiers, args.difficulty, args.expressions, args.output)


if __name__ == '__main__':
    main()
