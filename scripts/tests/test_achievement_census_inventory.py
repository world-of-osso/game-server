import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import achievement_census


class RawTypeInventory(unittest.TestCase):
    def test_pet_level_species_requires_authored_species_valid_level_and_correct_type(self):
        criteria = {
            cid: {'Type': str(kind), 'Asset': str(asset), 'ModifierTreeId': str(root),
                  'StartEvent': '0', 'StartTimer': '0', 'FailEvent': '0',
                  'EligibilityWorldStateID': '0'}
            for cid, kind, asset, root in ((25123, 160, 25, 15698), (34667, 162, 25, 47510),
                                            (1, 160, 25, 900), (2, 160, 26, 901), (3, 157, 0, 902))
        }
        datasets = dict.fromkeys(('spells', 'skills', 'mythic_ids', 'overlay_assets',
                                 'pet_creatures', 'toy_items', 'scenarios', 'zones', 'taxi_nodes'), frozenset())
        supported = achievement_census.supported_criteria(
            criteria, {157, 160, 162}, **datasets, pet_species={1426, 1974},
            single_modifiers={25123: (15698, 1426), 34667: (47510, 1974),
                              1: (900, 999999), 2: (901, 1426), 3: (902, 1426)},
            condition_trees=set())
        self.assertEqual(supported, {25123, 34667})

    def test_personal_rating_accepts_only_authored_current_two_and_three_player_assets(self):
        criteria = {index: {'Type': '39', 'Asset': str(asset), 'ModifierTreeId': '0',
                            'StartEvent': '0', 'StartTimer': '0', 'FailEvent': '0',
                            'EligibilityWorldStateID': '0'}
                    for index, asset in enumerate((0, 1, 2, 3, 6, 8), 1)}
        datasets = dict.fromkeys(('spells', 'skills', 'mythic_ids', 'overlay_assets',
                                 'pet_creatures', 'pet_species', 'toy_items', 'scenarios',
                                 'zones', 'taxi_nodes'), frozenset())
        supported = achievement_census.supported_criteria(
            criteria, {39}, **datasets, single_modifiers={}, condition_trees=set())
        self.assertEqual(supported, {1, 2}, 'no retired5v5/RBG/Shuffle/Blitz mapping')

    def test_orphan_tree_reference_does_not_become_achievement_coverage(self):
        criteria = {
            162: {'Type': '13'},
            2400: {'Type': '13'},
            64625: {'Type': '221'},
            999: {'Type': '283'},
        }
        trees = {
            8208: {'CriteriaID': '162'},
            100: {'CriteriaID': '2400'},
            200: {'CriteriaID': '64625'},
            201: {'CriteriaID': '64625'},
        }
        records = {
            227: {'criteria': [2400], 'flags': 0, 'trackable': False},
            7382: {'criteria': [64625], 'flags': 0, 'trackable': True},
        }
        inventory = achievement_census.raw_type_inventory(criteria, trees, records, {162, 64625})
        self.assertEqual(inventory[13], {
            'raw_rows': 2, 'supported_raw_rows': 1,
            'tree_referenced_rows': 2, 'achievement_root_rows': 1,
            'supported_achievement_root_rows': 0, 'supported_outside_achievement_roots': 1,
        })
        self.assertEqual(inventory[221]['supported_achievement_root_rows'], 1)
        self.assertEqual(inventory[283]['raw_rows'], 1)
        self.assertEqual(inventory[283]['tree_referenced_rows'], 0)


if __name__ == '__main__':
    unittest.main()
