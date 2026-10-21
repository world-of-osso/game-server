"""Native selected-pet records must be part of the reproducible utility census."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('follower_utility_census', ROOT / 'scripts/follower_utility_census.py')
census = importlib.util.module_from_spec(spec)
spec.loader.exec_module(census)


class NativePetCensus(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = census.census(ROOT / 'data/world.db')

    def test_native_felguard_action_intervals_are_not_owner_spell_grants(self):
        pets = self.data['pets']
        self.assertEqual([(row['owner_spec'], row['family'], row['spell_id'], row['min_level']) for row in pets], [
            (266, 29, 30213, 1), (266, 29, 30151, 14), (266, 29, 89766, 17),
        ])
        self.assertEqual([row['category'] for row in pets], ['rotation (excluded)', 'movement / gap closer', 'pet interrupt / control'])
        owner = [row['spell_id'] for row in self.data['spells'] if row['spec_id'] == 266]
        self.assertNotIn(89766, owner)

    def test_render_lists_all_native_pet_actions_and_execution_boundaries(self):
        report = census.render(self.data)
        for spell, level in [('Legion Strike `30213`', '1'), ('Pursuit `30151`', '14'), ('Axe Toss `89766`', '17')]:
            self.assertIn(f'| {spell} | {level} |', report)


if __name__ == '__main__':
    unittest.main()
