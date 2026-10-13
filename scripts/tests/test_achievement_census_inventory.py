import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import achievement_census


class RawTypeInventory(unittest.TestCase):
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
