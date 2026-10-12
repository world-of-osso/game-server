import copy
import importlib.util
import unittest
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / 'import_achievement_level_classes.py'
SPEC = importlib.util.spec_from_file_location('level_classes', MODULE)
level_classes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(level_classes)


class LevelClassPredicates(unittest.TestCase):
    def setUp(self):
        self.criteria = {64625: {
            'Type': '221', 'Asset': '0', 'ModifierTreeId': '300',
            'StartEvent': '0', 'StartTimer': '0', 'FailEvent': '0',
            'EligibilityWorldStateID': '0',
        }}
        self.modifiers = {
            300: {'Parent': '0', 'Operator': '4', 'Type': '0', 'Asset': '0',
                  'SecondaryAsset': '0', 'TertiaryAsset': '0'},
            301: {'Parent': '300', 'Operator': '2', 'Type': '26', 'Asset': '2',
                  'SecondaryAsset': '0', 'TertiaryAsset': '0'},
        }

    def test_pinned_paladin_max_level_is_qualified(self):
        self.assertEqual(level_classes.qualified_level_classes(self.criteria, self.modifiers),
                         {64625: (300, 2)})

    def test_unknown_predicates_and_timed_or_failed_credit_stay_disabled(self):
        for row_id, key, value in [(300, 'Operator', '3'), (301, 'Operator', '3'),
                                   (301, 'Type', '2'), (301, 'Asset', '14'),
                                   (301, 'SecondaryAsset', '1'), (300, 'TertiaryAsset', '1')]:
            with self.subTest(row=row_id, key=key):
                modifiers = copy.deepcopy(self.modifiers)
                modifiers[row_id][key] = value
                self.assertEqual(level_classes.qualified_level_classes(self.criteria, modifiers), {})
        for key in ('StartEvent', 'StartTimer', 'FailEvent', 'EligibilityWorldStateID'):
            with self.subTest(condition=key):
                criteria = copy.deepcopy(self.criteria)
                criteria[64625][key] = '1'
                self.assertEqual(level_classes.qualified_level_classes(criteria, self.modifiers), {})

    def test_unknown_sibling_and_nested_leaf_do_not_get_discarded(self):
        for parent in ('300', '301'):
            with self.subTest(parent=parent):
                modifiers = copy.deepcopy(self.modifiers)
                modifiers[302] = {'Parent': parent, 'Operator': '2', 'Type': '26',
                                  'Asset': '8', 'SecondaryAsset': '0', 'TertiaryAsset': '0'}
                self.assertEqual(level_classes.qualified_level_classes(self.criteria, modifiers), {})


if __name__ == '__main__':
    unittest.main()
