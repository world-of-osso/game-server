import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import import_achievement_player_facts as rules  # noqa: E402


def node(parent, operator, kind=0, asset=0):
    return {'Parent': str(parent), 'Operator': str(operator), 'Amount': '0',
            'Type': str(kind), 'Asset': str(asset), 'SecondaryAsset': '0', 'TertiaryAsset': '0'}


class PlayerFacts(unittest.TestCase):
    def setUp(self):
        self.criteria = {1: {'Type': '0', 'ModifierTreeId': '10', 'StartEvent': '0',
                             'StartTimer': '0', 'FailEvent': '0', 'EligibilityWorldStateID': '0'}}

    def compile(self, modifiers, criteria=None):
        return rules.qualified_player_facts(criteria or self.criteria, modifiers)

    def test_single_aura_leaf_requires_that_spell(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 8, 46598)}
        self.assertEqual(self.compile(modifiers), {1: (10, ((8, 46598),))})

    def test_all_of_player_facts_requires_every_fact(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 32, 1), 12: node(10, 2, 69, 60), 13: node(10, 2, 17, 12)}
        self.assertEqual(self.compile(modifiers), {1: (10, ((17, 12), (32, 1), (69, 60)))})

    def test_negated_foreign_nested_some_or_zero_leaves_stay_disabled(self):
        cases = {
            'single false': {10: node(0, 4), 11: node(10, 3, 8, 46598)},
            'foreign difficulty leaf': {10: node(0, 4), 11: node(10, 2, 8, 1), 12: node(10, 2, 68, 2)},
            'target leaf': {10: node(0, 4), 11: node(10, 2, 4, 448)},
            'nested': {10: node(0, 4), 11: node(10, 2, 8, 1), 12: node(11, 2, 8, 1)},
            'some root': {10: node(0, 8), 11: node(10, 2, 8, 1)},
            'zero asset': {10: node(0, 4), 11: node(10, 2, 32, 0)},
            'empty root': {10: node(0, 4)},
        }
        for name, modifiers in cases.items():
            with self.subTest(name):
                self.assertEqual(self.compile(modifiers), {})

    def test_structural_or_account_recomputed_types_stay_disabled(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 8, 46598)}
        for kind in sorted(rules.UNGATED_TYPES):
            with self.subTest(kind):
                criteria = copy.deepcopy(self.criteria)
                criteria[1]['Type'] = str(kind)
                self.assertEqual(self.compile(modifiers, criteria), {})

    def test_timed_failed_or_worldstate_criteria_stay_disabled(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 8, 46598)}
        for key in rules.CONDITIONS:
            with self.subTest(key):
                criteria = copy.deepcopy(self.criteria)
                criteria[1][key] = '5'
                self.assertEqual(self.compile(modifiers, criteria), {})


if __name__ == '__main__':
    unittest.main()
