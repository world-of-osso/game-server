import copy
import sys
import unittest
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

    def test_unsatisfiable_foreign_nested_or_negated_facts_stay_disabled(self):
        cases = {
            'all of two different difficulties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'difference empties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 3, 68, 1)},
            'unknown legacy value': {10: node(0, 4), 11: node(10, 2, 20, 7)},
            'some needing two': {10: node(0, 8, amount=2), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'some of facts': {10: node(0, 8), 11: node(10, 2, 8, 1)},
            'negated fact': {10: node(0, 4), 11: node(10, 3, 8, 46598)},
            'target leaf': {10: node(0, 4), 11: node(10, 2, 4, 448)},
            'nested all': {10: node(0, 4), 11: node(10, 4), 12: node(11, 2, 8, 1)},
            'zero fact asset': {10: node(0, 4), 11: node(10, 2, 32, 0)},
            'empty tree': {10: node(0, 4)},
        }
        for name, modifiers in cases.items():
            with self.subTest(name):
                self.assertEqual(self.compile(modifiers), {})

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
