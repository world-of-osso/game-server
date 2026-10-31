import unittest

from scripts.dawnbreaker_wcl_positions import positions


class DawnbreakerPositionsTest(unittest.TestCase):
    def test_preserves_logged_actor_positions_and_boss_windows(self):
        events = [
            {'sourceID': 5, 'fight_ms': 3877, 'x': 17847, 'y': 137377},
            {'sourceID': 5, 'fight_ms': 4000, 'x': 17847, 'y': 137377},
            {'sourceID': 8, 'fight_ms': 5000, 'x': 10, 'y': 20},
            {'sourceID': 5, 'fight_ms': 602888, 'x': -96251, 'y': 213313},
            {'sourceID': 5, 'fight_ms': 1293912, 'x': -92904, 'y': 190463},
        ]
        result = positions(events)
        self.assertEqual(result['legs'][0]['points'], [[3877, 1373.77, 178.47]])
        self.assertEqual(result['legs'][1]['points'], [[602888, 2133.13, -962.51]])
        self.assertEqual(result['legs'][2]['points'], [[1293912, 1904.63, -929.04]])

    def test_missing_window_fails(self):
        with self.assertRaisesRegex(ValueError, 'encounter 2837'):
            positions([])
