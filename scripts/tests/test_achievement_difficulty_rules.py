import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import import_achievement_difficulty_rules as rules  # noqa: E402


def node(parent, operator, kind=0, asset=0, amount=0):
    return {'Parent': str(parent), 'Operator': str(operator), 'Amount': str(amount),
            'Type': str(kind), 'Asset': str(asset), 'SecondaryAsset': '0', 'TertiaryAsset': '0'}


class DifficultyRules(unittest.TestCase):
    def setUp(self):
        self.criteria = {1: {'ModifierTreeId': '10', 'StartEvent': '0', 'StartTimer': '0',
                             'FailEvent': '0', 'EligibilityWorldStateID': '0'}}

    def compile(self, modifiers, criteria=None):
        # Pinned Difficulty.db2 OldEnumValue 0 = Normal1, 10-player3, Normal38, Lorewalking236.
        legacy = {0: {1, 3, 38, 236}, 1: {2, 4, 39}}
        return rules.qualified_difficulty_rules(criteria or self.criteria, modifiers, legacy)

    def test_legacy_value_admits_every_difficulty_with_that_old_enum(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 0)}
        self.assertEqual(self.compile(modifiers), {1: (10, True, (1, 3, 38, 236))})

    def test_legacy_and_current_difficulty_leaves_intersect(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 1), 12: node(10, 2, 68, 4)}
        self.assertEqual(self.compile(modifiers), {1: (10, True, (4,))})

    def test_legacy_single_false_excludes_its_difficulties(self):
        modifiers = {10: node(0, 4), 11: node(10, 3, 20, 1)}
        self.assertEqual(self.compile(modifiers), {1: (10, False, (2, 4, 39))})

    def test_unknown_legacy_value_is_unsatisfiable(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 7)}
        self.assertEqual(self.compile(modifiers), {})

    def test_all_single_true_mythic_requires_that_difficulty(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 68, 23)}
        self.assertEqual(self.compile(modifiers), {1: (10, True, (23,))})

    def test_some_one_of_normal_or_heroic_allows_either(self):
        modifiers = {10: node(0, 8), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)}
        self.assertEqual(self.compile(modifiers), {1: (10, True, (1, 2))})

    def test_all_single_false_excludes_listed_difficulties(self):
        modifiers = {10: node(0, 4), 11: node(10, 3, 68, 7), 12: node(10, 3, 68, 17)}
        self.assertEqual(self.compile(modifiers), {1: (10, False, (7, 17))})

    def test_unsatisfiable_or_mixed_or_foreign_trees_stay_disabled(self):
        cases = {
            'all of two different difficulties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'legacy normal and heroic': {10: node(0, 4), 11: node(10, 2, 20, 0), 12: node(10, 2, 68, 2)},
            'some needing two': {10: node(0, 8, amount=2), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'mixed operators': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 3, 68, 2)},
            'foreign leaf': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 2, 69, 10)},
            'nested leaf': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(11, 2, 68, 1)},
            'empty tree': {10: node(0, 4)},
        }
        for name, modifiers in cases.items():
            with self.subTest(name):
                self.assertEqual(self.compile(modifiers), {})

    def test_timed_failed_or_worldstate_criteria_stay_disabled(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 68, 23)}
        for key in rules.CONDITIONS:
            with self.subTest(key):
                criteria = copy.deepcopy(self.criteria)
                criteria[1][key] = '5'
                self.assertEqual(self.compile(modifiers, criteria), {})


if __name__ == '__main__':
    unittest.main()
