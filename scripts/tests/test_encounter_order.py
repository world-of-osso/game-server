import unittest
from encounter_order import build_order


class EncounterOrderTests(unittest.TestCase):
    def test_local_relations_and_leveling_then_expansion(self):
        maps = {2813: {'ExpansionID': 11, 'InstanceType': 1},
                3004: {'ExpansionID': 11, 'InstanceType': 2},
                43: {'ExpansionID': 0, 'InstanceType': 1},
                34: {'ExpansionID': 0, 'InstanceType': 1},
                2521: {'ExpansionID': 9, 'InstanceType': 1},
                2987: {'ExpansionID': 11, 'InstanceType': 2}}
        tracked = [[289, 585, 37]]
        challenges = [['Murder Row', 585, 2813]]
        tiers = [[1594, 505, 1304, 7, 156363, 505],
                 [1612, 505, 1320, 6, 156363, 505],
                 [1611, 505, 1317, 5, 156363, 505]]
        journal_maps = {1304: 2813, 1320: 3004, 1317: 2987}
        lfg = [dict(id=1, map_id=43, difficulty_id=1, tuning_id=193),
               dict(id=12, map_id=34, difficulty_id=1, tuning_id=215)]
        tuning = {193: 8, 215: 10}
        order = build_order(maps, tracked, challenges, tiers, journal_maps, lfg, tuning)
        self.assertEqual(sorted(order, key=lambda m: order[m]['sort']), [3004, 2813, 43, 34, 2987, 2521])
        self.assertEqual(order[2813]['tracked_row'], 289)
        self.assertEqual(order[43]['lfg_row'], 1)
        self.assertEqual(order[2521]['tier'], 3)


if __name__ == '__main__':
    unittest.main()
