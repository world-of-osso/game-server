import copy
import sqlite3
import sys
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import import_achievement_conditions as rules  # noqa: E402

# Pinned Difficulty.db2 OldEnumValue 0 = Normal1, 10-player3, Normal38, Lorewalking236.
LEGACY = {0: {1, 3, 38, 236}, 1: {2, 4, 39}}


def node(parent, operator, kind=0, asset=0, amount=0):
    return {'Parent': str(parent), 'Operator': str(operator), 'Amount': str(amount),
            'Type': str(kind), 'Asset': str(asset), 'SecondaryAsset': '0', 'TertiaryAsset': '0'}


class Conditions(unittest.TestCase):
    def setUp(self):
        self.criteria = {1: {'Type': '0', 'ModifierTreeId': '10', 'StartEvent': '0',
                             'StartTimer': '0', 'FailEvent': '0', 'EligibilityWorldStateID': '0'}}

    def compile(self, modifiers, criteria=None):
        compiled = rules.qualified_conditions(criteria or self.criteria, modifiers, LEGACY)
        return {row: rest for row, (_, *rest) in compiled.items()}

    def test_native_battleground_participation_retains_complete_content_tuning_range(self):
        criteria = {45044: dict(self.criteria[1], Type='15', Asset='2106', ModifierTreeId='97933')}
        mods = {97933: node(0, 4, amount=1),
                97934: node(97933, 2, 268, 483, amount=1)}
        self.assertEqual(self.compile(mods, criteria), {45044: [None, ((268, 483),)]})
        self.assertEqual(self.compile({**mods, 97935: node(97933, 2, 25, 1)}, criteria),
                         {45044: [None, ((25, 1), (268, 483))]})
        self.assertEqual(self.compile({**mods, 97935: node(97933, 2, 999, 1)}, criteria), {})
        for kind in ('0', '28', '29', '54', '74', '152', '165'):
            criteria[45044]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[45044]['Type'] = '15'
        for field, value in [('Operator', '3'), ('Asset', '0'),
                             ('SecondaryAsset', '1'), ('TertiaryAsset', '1')]:
            invalid = copy.deepcopy(mods)
            invalid[97934][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_hug_retains_player_dead_and_hostile_target_companions(self):
        criteria = {2379: dict(self.criteria[1], Type='54', Asset='56', ModifierTreeId='1080')}
        mods = {1080: node(0, 4, amount=1),
                1081: node(1080, 2, 5, 0, amount=1),
                1082: node(1080, 2, 6, 0, amount=1),
                1083: node(1080, 2, 7, 0, amount=1)}
        self.assertEqual(self.compile(mods, criteria),
                         {2379: [None, ((5, 0), (6, 0), (7, 0))]})
        self.assertEqual(self.compile({**mods, 1084: node(1080, 2, 8, 196665)}, criteria),
                         {2379: [None, ((5, 0), (6, 0), (7, 0), (8, 196665))]})
        self.assertEqual(self.compile({**mods, 1084: node(1080, 2, 999, 1)}, criteria), {})
        for kind in ('0', '28', '29', '74', '110', '165'):
            criteria[2379]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[2379]['Type'] = '54'
        for leaf in (1081, 1082, 1083):
            for field, value in [('Operator', '3'), ('Asset', '1'),
                                 ('SecondaryAsset', '1'), ('TertiaryAsset', '1')]:
                invalid = copy.deepcopy(mods)
                invalid[leaf][field] = value
                self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_kill_scenario_step_preserves_complete_positive_companions(self):
        criteria = {60015: dict(self.criteria[1], Type='0', Asset='204799', ModifierTreeId='6710')}
        mods = {6710: node(0, 4), 6711: node(6710, 2, 82, 3)}
        self.assertEqual(self.compile(mods, criteria), {60015: [None, ((82, 3),)]})
        self.assertEqual(self.compile({**mods, 6712: node(6710, 2, 25, 1)}, criteria),
                         {60015: [None, ((25, 1), (82, 3))]})
        self.assertEqual(self.compile({**mods, 6712: node(6710, 2, 999, 1)}, criteria), {})
        for kind in ('8', '27', '29', '74', '110', '165'):
            criteria[60015]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[60015]['Type'] = '0'
        for field, value in [('Operator', '3'), ('Asset', '0'),
                             ('SecondaryAsset', '1'), ('TertiaryAsset', '1')]:
            invalid = copy.deepcopy(mods)
            invalid[6711][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_login_completed_quest_preserves_complete_positive_companions(self):
        criteria = {58438: dict(self.criteria[1], Type='74', Asset='0', ModifierTreeId='216649')}
        mods = {216649: node(0, 4), 216650: node(216649, 2, 110, 72360)}
        self.assertEqual(self.compile(mods, criteria), {58438: [None, ((110, 72360),)]})
        self.assertEqual(self.compile({**mods, 216651: node(216649, 2, 25, 1)}, criteria),
                         {58438: [None, ((25, 1), (110, 72360))]})
        self.assertEqual(self.compile({**mods, 216651: node(216649, 2, 999, 1)}, criteria), {})
        for kind in ('0', '8', '27', '29', '110', '165'):
            criteria[58438]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[58438]['Type'] = '74'
        for field, value in [('Operator', '3'), ('Asset', '0'),
                             ('SecondaryAsset', '1'), ('TertiaryAsset', '1')]:
            invalid = copy.deepcopy(mods)
            invalid[216650][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_encounter_nested_all_preserves_scoped_award_and_difficulty_some(self):
        criteria = {36050: dict(self.criteria[1], Type='165', Asset='2051', ModifierTreeId='56522')}
        mods = {56522: node(0, 4), 56523: node(56522, 4),
                63594: node(56523, 2, 87, 11611), 56524: node(56522, 8, amount=1),
                56525: node(56524, 2, 68, 15), 56526: node(56524, 2, 68, 16)}
        self.assertEqual(self.compile(mods, criteria),
                         {36050: [(True, (15, 16)), ((87, 11611),)]})
        for kind in ('0', '28', '29', '74', '110'):
            criteria[36050]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[36050]['Type'] = '165'
        for field, value in [('Type', '999'), ('SecondaryAsset', '1'), ('Asset', '1')]:
            invalid = copy.deepcopy(mods)
            invalid[56523][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})
        self.assertEqual(self.compile({**mods, 63595: node(56523, 2, 999, 1)}, criteria), {})
        self.assertEqual(self.compile({**mods, 63595: node(56523, 3, 87, 10460)}, criteria), {})
        mixed_some = copy.deepcopy(mods)
        mixed_some[56526] = node(56524, 2, 87, 10460)
        self.assertEqual(self.compile(mixed_some, criteria), {})

    def test_native_kill_absent_aura_keeps_alive_and_difficulty_companions(self):
        criteria = {20693: dict(self.criteria[1], ModifierTreeId='6635')}
        mods = {6635: node(0, 4), 6636: node(6635, 2, 16),
                6637: node(6635, 3, 8, 115013)}
        self.assertEqual(self.compile(mods, criteria),
                         {20693: [None, ((-8, 115013), (16, 0))]})
        self.assertEqual(self.compile({**mods, 6638: node(6635, 2, 999, 1)}, criteria), {})
        criteria = {29405: dict(self.criteria[1], ModifierTreeId='27075')}
        mods = {27075: node(0, 4), 27076: node(27075, 3, 8, 196665),
                27077: node(27075, 3, 8, 196666), 27078: node(27075, 3, 68, 1),
                27079: node(27075, 3, 68, 2)}
        expected = {29405: [(False, (1, 2)), ((-8, 196665), (-8, 196666))]}
        self.assertEqual(self.compile(mods, criteria), expected)
        for kind in ('8', '29', '74', '78', '110'):
            criteria[29405]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[29405]['Type'] = '0'
        for field, value in [('SecondaryAsset', '1'), ('TertiaryAsset', '1'),
                             ('Asset', '0')]:
            invalid = copy.deepcopy(mods)
            invalid[27076][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_login_unknown_spell_requires_plain_complete_negation(self):
        criteria = {43205: dict(self.criteria[1], Type='74', Asset='0',
                               ModifierTreeId='86619')}
        mods = {86619: node(0, 4), 86620: node(86619, 3, 104, 264211)}
        self.assertEqual(self.compile(mods, criteria), {43205: [None, ((-104, 264211),)]})
        self.assertEqual(self.compile({**mods, 86622: node(86619, 2, 25, 1)}, criteria),
                         {43205: [None, ((-104, 264211), (25, 1))]})
        self.assertEqual(self.compile({**mods, 86622: node(86619, 2, 999, 1)}, criteria), {})
        for kind in ('0', '8', '29', '34', '110'):
            criteria[43205]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[43205]['Type'] = '74'
        for field, value in [('SecondaryAsset', '1'), ('TertiaryAsset', '1'), ('Asset', '0')]:
            invalid = copy.deepcopy(mods)
            invalid[86620][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})

    def test_native_cast_unearned_achievement_keeps_complete_companions(self):
        criteria = {103451: dict(self.criteria[1], Type='29', Asset='1217637',
                                ModifierTreeId='392319')}
        mods = {392319: node(0, 4), 392320: node(392319, 3, 87, 41707)}
        self.assertEqual(self.compile(mods, criteria), {103451: [None, ((-87, 41707),)]})
        self.assertEqual(self.compile({**mods, 392321: node(392319, 2, 25, 1)}, criteria),
                         {103451: [None, ((-87, 41707), (25, 1))]})
        self.assertEqual(self.compile({**mods, 392321: node(392319, 2, 999, 1)}, criteria), {})
        for kind in ('0', '8', '74', '110'):
            criteria[103451]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[103451]['Type'] = '29'
        mods[392320]['SecondaryAsset'] = '1'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[392320]['SecondaryAsset'] = '0'
        mods[392320]['Asset'] = '0'
        self.assertEqual(self.compile(mods, criteria), {})

    def test_some_locations_preserve_area_and_zone_leaf_meanings(self):
        criteria = {27968: dict(self.criteria[1], Type='29', Asset='181842',
                               ModifierTreeId='20912')}
        mods = {20912: node(0, 8, amount=1),
                20913: node(20912, 2, 17, 1445),
                20914: node(20912, 2, 17, 254),
                20915: node(20912, 2, 41, 25)}
        self.assertEqual(self.compile(mods, criteria),
                         {27968: [None, ((17, 254), (17, 1445), (41, 25))]})
        mods[20913]['Type'] = '41'
        mods[20914]['Type'] = '41'
        self.assertEqual(self.compile(mods, criteria),
                         {27968: [None, ((41, 25), (41, 254), (41, 1445))]})
        for field, value in [('Type', '999'), ('Operator', '3'),
                             ('SecondaryAsset', '1'), ('Asset', '0')]:
            invalid = copy.deepcopy(mods)
            invalid[20915][field] = value
            self.assertEqual(self.compile(invalid, criteria), {})
        mods[20912]['Amount'] = '2'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[20912]['Amount'] = '1'
        mods[20916] = node(20915, 2, 17, 25)
        self.assertEqual(self.compile(mods, criteria), {})

    def test_plain_empty_all_is_true_but_empty_some_and_single_are_false(self):
        self.assertEqual(self.compile({10: node(0, 4)}), {1: [None, ()]})
        self.assertEqual(self.compile({10: node(0, 8)}), {})
        self.assertEqual(self.compile({10: node(0, 2)}), {})

    def test_battle_pet_type_earned_level_requires_complete_positive_companions(self):
        criteria = {21377: dict(self.criteria[1], Type='160', Asset='25', ModifierTreeId='4131')}
        mods = {4131: node(0, 4), 4132: node(4131, 2, 78, 9)}
        self.assertEqual(self.compile(mods, criteria), {21377: [None, ((78, 9),)]})
        self.assertEqual(self.compile({**mods, 4133: node(4131, 2, 32, 0)}, criteria),
                         {21377: [None, ((32, 0), (78, 9))]})
        self.assertEqual(self.compile({**mods, 4133: node(4131, 2, 999, 1)}, criteria), {})
        mods[4132]['Asset'] = '0'
        self.assertEqual(self.compile(mods, criteria), {21377: [None, ((78, 0),)]},
                         'Humanoid pet type0 is real, not missing authority')
        mods[4132]['Asset'] = '10'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[4132]['Asset'] = '9'
        mods[4132]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[4132]['Operator'] = '2'
        for level in ('0', '26'):
            criteria[21377]['Asset'] = level
            self.assertEqual(self.compile(mods, criteria), {})
        criteria[21377]['Asset'] = '25'
        for kind in ('157', '158', '162'):
            criteria[21377]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})

    def test_creature_type_kill_requires_complete_positive_companions(self):
        criteria = {4953: dict(self.criteria[1], Type='78', ModifierTreeId='72')}
        mods = {72: node(0, 4), 73: node(72, 2, 30, 7)}
        self.assertEqual(self.compile(mods, criteria), {4953: [None, ((30, 7),)]})
        self.assertEqual(self.compile({**mods, 74: node(72, 2, 32, 0)}, criteria),
                         {4953: [None, ((30, 7), (32, 0))]})
        self.assertEqual(self.compile({**mods, 74: node(72, 2, 999, 1)}, criteria), {})
        mods[73]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[73]['Operator'] = '2'
        mods[73]['Asset'] = '0'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[73]['Asset'] = '7'
        for kind in ('0', '70', '110'):
            criteria[4953]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})

    def test_node_world_state_win_requires_complete_positive_companions(self):
        criteria = {14943: dict(self.criteria[1], Type='1', ModifierTreeId='2061')}
        mods = {2061: node(0, 4), 2062: node(2061, 2, 67, 8656)}
        self.assertEqual(self.compile(mods, criteria), {14943: [None, ((67, 8656),)]})
        self.assertEqual(self.compile({**mods, 2063: node(2061, 2, 32, 761)}, criteria),
                         {14943: [None, ((32, 761), (67, 8656))]})
        self.assertEqual(self.compile({**mods, 2063: node(2061, 2, 999, 1)}, criteria), {})
        mods[2062]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[2062]['Operator'] = '2'
        for kind in ('0', '8', '15'):
            criteria[14943]['Type'] = kind
            self.assertEqual(self.compile(mods, criteria), {})

    def test_earned_player_achievement_requires_complete_positive_companions(self):
        mods = {10: node(0, 4), 11: node(10, 2, 87, 10460)}
        self.assertEqual(self.compile(mods), {1: [None, ((87, 10460),)]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 25, 1)}),
                         {1: [None, ((25, 1), (87, 10460))]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 999, 1)}), {})
        mods[10] = node(0, 8, amount=1)
        self.assertEqual(self.compile(mods), {1: [None, ((87, 10460),)]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 87, 11611)}), {},
                         'multi-child earned87 Some is outside the bounded contract')
        mods[11]['Operator'] = '3'
        self.assertEqual(self.compile(mods), {})
        mods[11]['Operator'] = '2'
        mods[11]['Asset'] = '0'
        self.assertEqual(self.compile(mods), {})
        mods[11]['Asset'] = '10460'
        mods[11]['SecondaryAsset'] = '1'
        self.assertEqual(self.compile(mods), {})
        mods[11]['SecondaryAsset'] = '0'
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '8'
        self.assertEqual(self.compile(mods, criteria), {}, 'ungated meta cannot read player state')

    def test_spell_hit_target_creature_keeps_complete_companions_and_alternatives(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '110'
        mods = {10: node(0, 4), 11: node(10, 2, 4, 16802)}
        self.assertEqual(self.compile(mods, criteria), {1: [None, ((4, 16802),)]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 25, 1)}, criteria),
                         {1: [None, ((4, 16802), (25, 1))]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 999, 1)}, criteria), {})
        mods[10] = node(0, 8, amount=1)
        self.assertEqual(self.compile(mods, criteria), {1: [None, ((4, 16802),)]})
        mods[12] = node(10, 2, 4, 10181)
        self.assertEqual(self.compile(mods, criteria),
                         {1: [None, ((4, 10181), (4, 16802))]})
        mods[12]['Type'] = '999'
        self.assertEqual(self.compile(mods, criteria), {})
        mods.pop(12)
        mods[11]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[11]['Operator'] = '2'
        mods[11]['Asset'] = '0'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[11]['Asset'] = '16802'
        criteria[1]['Type'] = '28'
        self.assertEqual(self.compile(mods, criteria), {})

    def test_target_aura_requires_native_hit_and_complete_companions(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '110'
        mods = {10: node(0, 4), 11: node(10, 2, 10, 65403)}
        self.assertEqual(self.compile(mods, criteria), {1: [None, ((10, 65403),)]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 25, 1)}, criteria),
                         {1: [None, ((10, 65403), (25, 1))]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 999, 1)}, criteria), {})
        mods[10]['Operator'] = '8'
        self.assertEqual(self.compile(mods, criteria), {1: [None, ((10, 65403),)]})
        mods[11]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[11]['Operator'] = '2'
        mods[11]['Asset'] = '0'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[11]['Asset'] = '65403'
        criteria[1]['Type'] = '29'
        self.assertEqual(self.compile(mods, criteria), {})

    def test_target_health_percent_requires_native_hit_and_complete_companions(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '110'
        mods = {10: node(0, 4), 11: node(10, 2, 46, 5)}
        self.assertEqual(self.compile(mods, criteria), {1: [None, ((46, 5),)]})
        for boundary in (1, 100):
            mods[11]['Asset'] = str(boundary)
            self.assertEqual(self.compile(mods, criteria), {1: [None, ((46, boundary),)]})
        for invalid in (0, 101):
            mods[11]['Asset'] = str(invalid)
            self.assertEqual(self.compile(mods, criteria), {})
        mods[11] = node(10, 2, 46, 5)
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 25, 1)}, criteria),
                         {1: [None, ((25, 1), (46, 5))]})
        self.assertEqual(self.compile({**mods, 12: node(10, 2, 999, 1)}, criteria), {})
        mods[11]['Operator'] = '3'
        self.assertEqual(self.compile(mods, criteria), {})
        mods[11]['Operator'] = '2'
        criteria[1]['Type'] = '29'
        self.assertEqual(self.compile(mods, criteria), {})

    def test_target_race_requires_a_native_spell_hit_target_and_keeps_companions(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[5777] = criteria.pop(1)
        criteria[5777]['Type'] = '110'
        modifiers = {10: node(0, 4), 11: node(10, 2, 27, 1)}
        self.assertEqual(self.compile(modifiers, criteria), {5777: [None, ((27, 1),)]})
        modifiers[10]['Operator'] = '8'
        self.assertEqual(self.compile(modifiers, criteria), {5777: [None, ((27, 1),)]})
        criteria[5777]['Type'] = '28'
        self.assertEqual(self.compile(modifiers, criteria), {}, 'recipient28 has no caster-race snapshot')
        criteria[5777]['Type'] = '110'
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers, criteria), {})
        modifiers[11]['Operator'] = '2'
        modifiers[11]['Asset'] = '0'
        self.assertEqual(self.compile(modifiers, criteria), {}, 'NPC RACE_NONE is not a playable race')
        modifiers[11]['Asset'] = '1'
        modifiers[12] = node(10, 2, 30, 1)
        self.assertEqual(self.compile(modifiers, criteria), {}, 'unknown creature-type companion remains required')

    def test_target_class_requires_actual_player_target_and_retains_companions(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '110'
        modifiers = {10: node(0, 4), 11: node(10, 2, 5, 0), 12: node(10, 2, 28, 6)}
        self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((5, 0), (28, 6))]})
        modifiers[11] = node(10, 2, 27, 1)
        self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((27, 1), (28, 6))]})
        del modifiers[11]
        self.assertEqual(self.compile(modifiers, criteria), {}, 'NPC class authority is not sourced')
        modifiers[11] = node(10, 2, 5, 0)
        criteria[1]['Type'] = '28'
        self.assertEqual(self.compile(modifiers, criteria), {})
        criteria[1]['Type'] = '110'
        modifiers[12]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers, criteria), {})
        modifiers[12]['Operator'] = '2'
        for asset in (0, 14):
            modifiers[12]['Asset'] = str(asset)
            self.assertEqual(self.compile(modifiers, criteria), {})
        modifiers[12]['Asset'] = '6'
        modifiers[13] = node(10, 2, 67, 1368)
        self.assertEqual(self.compile(modifiers, criteria), {}, 'world-state companion cannot be removed')

    def test_positive_battle_pet_team_level_preserves_every_companion(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[23257] = criteria.pop(1)
        criteria[23257]['Type'] = '158'
        modifiers = {10: node(0, 4), 11: node(10, 2, 34, 25)}
        self.assertEqual(self.compile(modifiers, criteria), {23257: [None, ((34, 25),)]})
        modifiers[10]['Operator'] = '8'
        self.assertEqual(self.compile(modifiers, criteria), {23257: [None, ((34, 25),)]})
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers, criteria), {})
        modifiers[11]['Operator'] = '2'
        modifiers[12] = node(10, 2, 90)
        self.assertEqual(self.compile(modifiers, criteria), {}, 'NYI PvP90 cannot be dropped')
        del modifiers[12]
        for asset in (0, 26):
            modifiers[11]['Asset'] = str(asset)
            self.assertEqual(self.compile(modifiers, criteria), {})

    def test_all_single_true_mythic_requires_that_difficulty(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 68, 23)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (23,)), ()]})

    def test_some_one_of_normal_or_heroic_allows_either(self):
        modifiers = {10: node(0, 8), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 2)), ()]})

    def test_all_single_false_excludes_listed_difficulties(self):
        modifiers = {10: node(0, 4), 11: node(10, 3, 68, 7), 12: node(10, 3, 68, 17)}
        self.assertEqual(self.compile(modifiers), {1: [(False, (7, 17)), ()]})

    def test_all_single_true_and_false_admits_the_difference(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 1), 12: node(10, 3, 68, 39)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (2, 4)), ()]})

    def test_legacy_value_admits_every_difficulty_with_that_old_enum(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 0)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 3, 38, 236)), ()]})

    def test_legacy_and_current_difficulty_leaves_intersect(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 1), 12: node(10, 2, 68, 4)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (4,)), ()]})

    def test_player_facts_alone_or_with_difficulty(self):
        alone = {10: node(0, 4), 11: node(10, 2, 32, 1), 12: node(10, 2, 69, 60), 13: node(10, 2, 17, 12)}
        self.assertEqual(self.compile(alone), {1: [None, ((17, 12), (32, 1), (69, 60))]})
        mixed = {10: node(0, 4), 11: node(10, 2, 8, 46598), 12: node(10, 0), 13: node(12, 2, 68, 1), 14: node(12, 2, 68, 2)}
        mixed[12] = node(10, 8)
        self.assertEqual(self.compile(mixed), {1: [(True, (1, 2)), ((8, 46598),)]})

    def test_untimestamped_time_event_always_passed(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 289, 600), 12: node(10, 2, 8, 46598)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((8, 46598),)]})
        timestamped = {10: node(0, 4), 11: node(10, 2, 289, 350)}
        self.assertEqual(self.compile(timestamped), {})
        window = {10: node(0, 4), 11: node(10, 2, 289, 350), 12: node(10, 3, 289, 353)}
        self.assertEqual(self.compile(window), {})

    def test_unsatisfiable_foreign_nested_or_negated_facts_stay_disabled(self):
        cases = {
            'all of two different difficulties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'difference empties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 3, 68, 1)},
            'unknown legacy value': {10: node(0, 4), 11: node(10, 2, 20, 7)},
            'some needing two': {10: node(0, 8, amount=2), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'some of mixed aura/map facts': {10: node(0, 8), 11: node(10, 2, 8, 1), 12: node(10, 2, 32, 2)},
            'negated map fact': {10: node(0, 4), 11: node(10, 3, 32, 1)},
            'target leaf': {10: node(0, 4), 11: node(10, 2, 4, 448)},
            'nested all': {10: node(0, 4), 11: node(10, 4), 12: node(11, 2, 8, 1)},
            'zero aura asset': {10: node(0, 4), 11: node(10, 2, 8, 0)},
        }
        for name, modifiers in cases.items():
            with self.subTest(name):
                self.assertEqual(self.compile(modifiers), {})

    def test_single_child_some_preserves_the_supported_target_condition(self):
        emote = copy.deepcopy(self.criteria)
        emote[1]['Type'] = '54'
        modifiers = {10: node(0, 8, amount=1), 11: node(10, 2, 4, 216058)}
        self.assertEqual(self.compile(modifiers, emote), {1: [None, ((4, 216058),)]})
        modifiers[10]['Amount'] = '0'
        self.assertEqual(self.compile(modifiers, emote), {1: [None, ((4, 216058),)]})
        modifiers[10]['Amount'] = '2'
        self.assertEqual(self.compile(modifiers, emote), {})
        modifiers[10]['Amount'] = '1'
        modifiers[11]['Type'] = '5'
        self.assertEqual(self.compile(modifiers, emote), {}, 'unknown target fields remain blocked')

    def test_target_creature_requires_a_per_request_emote_producer(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 4, 1412)}
        emote = copy.deepcopy(self.criteria)
        emote[1]['Type'] = '54'
        self.assertEqual(self.compile(modifiers, emote), {1: [None, ((4, 1412),)]})
        self.assertEqual(self.compile(modifiers), {}, 'other producers have no target snapshot')
        for kind in (5, 6, 7):
            modifiers[11] = node(10, 2, kind, 0)
            self.assertEqual(self.compile(modifiers, emote), {1: [None, ((kind, 0),)]},
                             'native emote retains the complete requested-unit qualifier')
            self.assertEqual(self.compile(modifiers), {}, 'kill event has no emote target authority')

    def test_arena_player_facts_on_all_gated_events(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 24, 2), 12: node(10, 2, 60, 0)}
        for kind in (0, 17, 32, 33, 56, 70):
            criteria = copy.deepcopy(self.criteria)
            criteria[1]['Type'] = str(kind)
            self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((24, 2), (60, 0))]})
            modifiers[11]['Asset'] = '5'
            self.assertEqual(self.compile(modifiers, criteria), {}, 'no native5v5')
            modifiers[11]['Asset'] = '2'
        derived = copy.deepcopy(self.criteria)
        derived[1]['Type'] = '8'
        self.assertEqual(self.compile(modifiers, derived), {}, 'structural recomputation has no player facts')

    def test_player_zone_preserves_the_authored_id_and_other_conditions(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 41, 1519), 12: node(10, 2, 69, 10)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((41, 1519), (69, 10))]})
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers), {}, 'negated zone remains unsupported')

    def test_exact_player_level_lowers_to_matching_minimum_and_maximum(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 39, 3)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((69, 3), (71, 3))]})
        modifiers[11]['Asset'] = '0'
        self.assertEqual(self.compile(modifiers), {}, 'no playable zero level')
        modifiers[11] = node(10, 3, 39, 3)
        self.assertEqual(self.compile(modifiers), {}, 'negated level stays unsupported')

    def test_known_spell_preserves_the_marker_and_all_companion_leaves(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 104, 406106), 12: node(10, 2, 8, 419239)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((8, 419239), (104, 406106))]})
        modifiers[11]['Asset'] = '0'
        self.assertEqual(self.compile(modifiers), {}, 'unknown zero spell is not knowledge')
        modifiers[11] = node(10, 3, 104, 406106)
        self.assertEqual(self.compile(modifiers), {}, 'negated knowledge stays unsupported')
        modifiers[11] = node(10, 2, 104, 406106)
        modifiers[13] = node(10, 2, 67, 4546)
        self.assertEqual(self.compile(modifiers), {}, 'unknown companion rejects the whole predicate')

    def test_native_item_facts_preserve_ids_quality_and_all_required_leaves(self):
        for kind in (90, 91):
            criteria = copy.deepcopy(self.criteria)
            criteria[1]['Type'] = str(kind)
            modifiers = {10: node(0, 4), 11: node(10, 2, 19, 124101), 12: node(10, 2, 8, 419239)}
            self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((8, 419239), (19, 124101))]})
            for quality_kind in (14, 15):
                modifiers = {10: node(0, 4), 11: node(10, 2, quality_kind, 4)}
                self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((quality_kind, 4),)]})
                modifiers[11]['Asset'] = '0'
                self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((quality_kind, 0),)]})
                modifiers[11]['Asset'] = '9'
                self.assertEqual(self.compile(modifiers, criteria), {})
            modifiers = {10: node(0, 8, amount=1), 11: node(10, 2, 19, 173032), 12: node(10, 2, 19, 45199)}
            self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((19, 45199), (19, 173032))]})
            modifiers[10]['Amount'] = '2'
            self.assertEqual(self.compile(modifiers, criteria), {})
            modifiers[10]['Amount'] = '1'
            modifiers[12]['Type'] = '359'
            self.assertEqual(self.compile(modifiers, criteria), {}, 'unknown alternatives reject the whole tree')
            modifiers = {10: node(0, 4), 11: node(10, 2, 19, 124101)}
            self.assertEqual(self.compile(modifiers), {}, 'other families have no declared native item receipt snapshot')

    def test_alive_condition_preserves_zero_asset_and_other_required_leaves(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 16, 0), 12: node(10, 2, 17, 6665)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((16, 0), (17, 6665))]})
        modifiers[11]['Asset'] = '1'
        self.assertEqual(self.compile(modifiers), {}, 'alive has no asset selector')
        modifiers[11] = node(10, 3, 16, 0)
        self.assertEqual(self.compile(modifiers), {}, 'negated alive remains unsupported')
        modifiers[11] = node(10, 2, 16, 0)
        modifiers[13] = node(10, 2, 67, 4546)
        self.assertEqual(self.compile(modifiers), {}, 'unknown world state rejects the whole tree')

    def test_known_solo_condition_admits_the_pinned_zero_asset(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 35, 0)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((35, 0),)]})
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers), {}, 'negated party conditions remain unsupported')

    def test_group_roster_limit_preserves_strict_threshold(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 0), 12: node(10, 2, 29, 9)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 3, 38, 236)), ((29, 9),)]})

    def test_map_instance_type_preserves_exact_type_including_world_zero(self):
        for instance_type in (0, 1, 2, 3, 4, 5):
            modifiers = {10: node(0, 4), 11: node(10, 2, 122, instance_type)}
            self.assertEqual(self.compile(modifiers), {1: [None, ((122, instance_type),)]})
        modifiers[11]['Asset'] = '128'
        self.assertEqual(self.compile(modifiers), {}, 'unknown out-of-range type stays disabled')

    def test_emote_some_accepts_every_plain_target_alternative_without_dropping_children(self):
        criteria = copy.deepcopy(self.criteria)
        criteria[1]['Type'] = '54'
        modifiers = {10: node(0, 8, amount=1),
                     11: node(10, 2, 4, 63288), 12: node(10, 2, 4, 64798)}
        self.assertEqual(self.compile(modifiers, criteria),
                         {1: [None, ((4, 63288), (4, 64798))]})
        modifiers[10]['Amount'] = '0'
        self.assertEqual(self.compile(modifiers, criteria),
                         {1: [None, ((4, 63288), (4, 64798))]})
        for change in ({'Amount': '2'}, {'Asset': '99'}, {'SecondaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[10].update(change)
            self.assertEqual(self.compile(rejected, criteria), {})
        for change in ({'Type': '8'}, {'Operator': '3'}, {'Asset': '0'},
                       {'SecondaryAsset': '1'}, {'TertiaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[12].update(change)
            self.assertEqual(self.compile(rejected, criteria), {})
        modifiers[13] = node(12, 2, 4, 1412)
        self.assertEqual(self.compile(modifiers, criteria), {}, 'nested unknown child is not dropped')
        del modifiers[13]
        self.assertEqual(self.compile(modifiers), {}, 'other event types have no target snapshot')

    def test_player_map_some_preserves_every_complete_alternative(self):
        modifiers = {10: node(0, 8, amount=1),
                     11: node(10, 2, 32, 489), 12: node(10, 2, 32, 2106)}
        for amount in ('0', '1'):
            modifiers[10]['Amount'] = amount
            self.assertEqual(self.compile(modifiers),
                             {1: [None, ((32, 489), (32, 2106))]})
        for change in ({'Amount': '2'}, {'Asset': '99'}, {'SecondaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[10].update(change)
            self.assertEqual(self.compile(rejected), {})
        for change in ({'Type': '8'}, {'Operator': '3'}, {'Asset': '0'},
                       {'SecondaryAsset': '1'}, {'TertiaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[12].update(change)
            self.assertEqual(self.compile(rejected), {})
        modifiers[13] = node(12, 2, 32, 1)
        self.assertEqual(self.compile(modifiers), {}, 'nested child cannot be dropped')

    def test_player_area_some_preserves_every_complete_alternative(self):
        modifiers = {10: node(0, 8, amount=1),
                     11: node(10, 2, 17, 1377), 12: node(10, 2, 17, 9310)}
        for amount in ('0', '1'):
            modifiers[10]['Amount'] = amount
            self.assertEqual(self.compile(modifiers),
                             {1: [None, ((17, 1377), (17, 9310))]})
        for change in ({'Amount': '2'}, {'Type': '32'}, {'Asset': '99'}, {'SecondaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[10].update(change)
            self.assertEqual(self.compile(rejected), {})
        for change in ({'Type': '999'}, {'Operator': '3'}, {'Asset': '0'},
                       {'SecondaryAsset': '1'}, {'TertiaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[12].update(change)
            self.assertEqual(self.compile(rejected), {})
        modifiers[13] = node(12, 2, 17, 2738)
        self.assertEqual(self.compile(modifiers), {}, 'nested child cannot be dropped')

    def test_player_aura_some_preserves_every_complete_spell_alternative(self):
        modifiers = {10: node(0, 8, amount=1),
                     11: node(10, 2, 8, 136775), 12: node(10, 2, 8, 136777),
                     13: node(10, 2, 8, 136778)}
        for amount in ('0', '1'):
            modifiers[10]['Amount'] = amount
            self.assertEqual(self.compile(modifiers),
                             {1: [None, ((8, 136775), (8, 136777), (8, 136778))]})
        for change in ({'Amount': '2'}, {'Asset': '99'}, {'SecondaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[10].update(change)
            self.assertEqual(self.compile(rejected), {})
        for change in ({'Type': '9'}, {'Operator': '3'}, {'Asset': '0'},
                       {'SecondaryAsset': '1'}, {'TertiaryAsset': '1'}):
            rejected = copy.deepcopy(modifiers)
            rejected[13].update(change)
            self.assertEqual(self.compile(rejected), {})
        modifiers[14] = node(13, 2, 8, 1459)
        self.assertEqual(self.compile(modifiers), {}, 'nested child cannot be dropped')

    def test_player_map_zero_is_a_real_world_map_not_missing_reference_state(self):
        for operator in (4, 8):
            modifiers = {10: node(0, operator, amount=1), 11: node(10, 2, 32, 0)}
            self.assertEqual(self.compile(modifiers), {1: [None, ((32, 0),)]})
            modifiers[11]['Operator'] = '3'
            self.assertEqual(self.compile(modifiers), {}, 'negated map remains unsupported')
        modifiers[11] = node(10, 2, 32, -1)
        self.assertEqual(self.compile(modifiers), {}, 'negative map is not a known ID')

    def test_native_identity_loader_rejects_extra_fields_flags_and_source34(self):
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('''
                CREATE TABLE tdb_conditions(SourceTypeOrReferenceId INTEGER, SourceEntry INTEGER);
                CREATE TABLE player_condition(
                    ID INTEGER, Flags INTEGER, ClassMask INTEGER,
                    RaceMasks_0 INTEGER, RaceMasks_1 INTEGER,
                    Gender INTEGER, NativeGender INTEGER, PrevQuestID_0 INTEGER,
                    Failure_description_lang TEXT);
                INSERT INTO player_condition VALUES
                    (923,1,0,1309324210,-1440044374,-1,-1,0,''),
                    (62,0,8,0,0,-1,-1,0,''),
                    (100,0,8,0,0,-1,-1,77,''),
                    (101,128,8,0,0,-1,-1,0,''),
                    (102,0,8,0,0,0,-1,0,''),
                    (103,0,8,0,0,-1,-1,0,'');
                INSERT INTO tdb_conditions VALUES(34,103);
            ''')
            self.assertEqual(rules.load_identity_condition_ids(db), {923, 62})

    def test_player_condition_leaf_preserves_the_authored_id(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 2, 923)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((2, 923),)]})

    def test_ungated_types_and_timed_criteria_stay_disabled(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 8, 46598)}
        for kind in sorted(rules.UNGATED_TYPES):
            with self.subTest(kind):
                criteria = copy.deepcopy(self.criteria)
                criteria[1]['Type'] = str(kind)
                self.assertEqual(self.compile(modifiers, criteria), {})
        for key in rules.CONDITIONS:
            with self.subTest(key):
                criteria = copy.deepcopy(self.criteria)
                criteria[1][key] = '5'
                self.assertEqual(self.compile(modifiers, criteria), {})


if __name__ == '__main__':
    unittest.main()
