import copy
import importlib.util
import unittest
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / 'import_achievement_single_modifiers.py'
SPEC = importlib.util.spec_from_file_location('single_modifiers', MODULE)
single_modifiers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(single_modifiers)


class SingleModifierPredicates(unittest.TestCase):
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
        self.assertEqual(single_modifiers.qualified_single_modifiers(self.criteria, self.modifiers),
                         {64625: (300, 2)})

    def test_unknown_predicates_and_timed_or_failed_credit_stay_disabled(self):
        for row_id, key, value in [(300, 'Operator', '3'), (301, 'Operator', '3'),
                                   (301, 'Type', '2'), (301, 'Asset', '14'),
                                   (301, 'SecondaryAsset', '1'), (300, 'TertiaryAsset', '1')]:
            with self.subTest(row=row_id, key=key):
                modifiers = copy.deepcopy(self.modifiers)
                modifiers[row_id][key] = value
                self.assertEqual(single_modifiers.qualified_single_modifiers(self.criteria, modifiers), {})
        for key in ('StartEvent', 'StartTimer', 'FailEvent', 'EligibilityWorldStateID'):
            with self.subTest(condition=key):
                criteria = copy.deepcopy(self.criteria)
                criteria[64625][key] = '1'
                self.assertEqual(single_modifiers.qualified_single_modifiers(criteria, self.modifiers), {})

    def test_pinned_hero_of_the_storm_species_is_qualified_only_for_learned_pet(self):
        criteria = {27884: dict(self.criteria[64625], Type='155', ModifierTreeId='20772')}
        modifiers = {
            20772: dict(self.modifiers[300]),
            20773: dict(self.modifiers[301], Parent='20772', Type='91', Asset='1639'),
        }
        self.assertEqual(single_modifiers.qualified_single_modifiers(criteria, modifiers),
                         {27884: (20772, 1639)})
        for kind, modifier_type in [('221', '91'), ('155', '26'), ('96', '91')]:
            with self.subTest(kind=kind, modifier=modifier_type):
                crossed = {27884: dict(criteria[27884], Type=kind)}
                swapped = copy.deepcopy(modifiers)
                swapped[20773]['Type'] = modifier_type
                self.assertEqual(single_modifiers.qualified_single_modifiers(crossed, swapped), {})

    def test_overstuffed_species_is_qualified_only_for_actual_pet_levels(self):
        criteria = {25123: dict(self.criteria[64625], Type='160', Asset='25', ModifierTreeId='15698')}
        modifiers = {
            15698: dict(self.modifiers[300]),
            15699: dict(self.modifiers[301], Parent='15698', Type='91', Asset='1426'),
        }
        for kind in ('160', '162'):
            with self.subTest(kind=kind):
                criteria[25123]['Type'] = kind
                self.assertEqual(single_modifiers.qualified_single_modifiers(criteria, modifiers),
                                 {25123: (15698, 1426)})
        for kind in ('157', '158', '96'):
            criteria[25123]['Type'] = kind
            self.assertEqual(single_modifiers.qualified_single_modifiers(criteria, modifiers), {})

    def test_unknown_sibling_and_nested_leaf_do_not_get_discarded(self):
        for parent in ('300', '301'):
            with self.subTest(parent=parent):
                modifiers = copy.deepcopy(self.modifiers)
                modifiers[302] = {'Parent': parent, 'Operator': '2', 'Type': '26',
                                  'Asset': '8', 'SecondaryAsset': '0', 'TertiaryAsset': '0'}
                self.assertEqual(single_modifiers.qualified_single_modifiers(self.criteria, modifiers), {})


if __name__ == '__main__':
    unittest.main()
