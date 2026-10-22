"""Read actual authenticated70448/cached NPC evidence without network requests."""
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import read_cached_npc, read_db2, select_assignment, transform_marker


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires the authenticated70448 and cached NPC evidence paths')
class FollowerPlacementSources(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tables, cls.proof = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        cls.cache = Path(os.environ['FOLLOWER_PLACEMENT_CACHE'])

    def test_actual_neltharus_markers_keep_sourced_floor_and_react(self):
        cases = [(181861, 0, 2080, -211.1, -932.7),
                 (189340, 0, 2080, -232.125, -674.9625),
                 (189478, 1, 2081, -264.4580637024, -503.5),
                 (189901, 0, 2080, -380.025, -772.8375)]
        for entry, floor, ui_map, x, y in cases:
            with self.subTest(entry=entry):
                page = read_cached_npc(self.cache, entry)
                self.assertEqual(page['entity']['react'], [-1, -1])
                self.assertEqual(len(page['markers']), 1)
                marker = page['markers'][0]
                self.assertEqual((marker['zone'], marker['floor']), (14011, floor))
                assignments = select_assignment(self.tables, 2519, marker)
                self.assertEqual({int(a['UiMapID']) for a in assignments}, {ui_map})
                actual_x, actual_y = transform_marker(marker, assignments)
                self.assertAlmostEqual(actual_x, x, places=6)
                self.assertAlmostEqual(actual_y, y, places=6)

    def test_populated_ruby_calibration_keeps_native_lower_and_upper_floor_transforms(self):
        for entry, ui_map, expected in [(188252, 2095, (1549.2, -247.35)),
                                         (189232, 2094, (1645.46314292, -132.0835936))]:
            with self.subTest(entry=entry):
                page = read_cached_npc(self.cache, entry)
                assignment = select_assignment(self.tables, 2521, page['markers'][0])
                self.assertEqual({int(a['UiMapID']) for a in assignment}, {ui_map})
                actual = transform_marker(page['markers'][0], assignment)
                self.assertAlmostEqual(actual[0], expected[0], places=6)
                self.assertAlmostEqual(actual[1], expected[1], places=6)

    def test_teera_actual_empty_mapper_is_an_explicit_source_gap_not_a_crash(self):
        try:
            page = read_cached_npc(self.cache, 186339)
        except AttributeError as error:
            self.fail(f'actual empty Wowhead mapper crashed rather than reporting no markers: {error}')
        self.assertEqual(page['entity']['id'], 186339)
        self.assertEqual(page['markers'], [])

    def test_foreign_map_marker_cannot_be_reinterpreted_as_neltharus(self):
        page = read_cached_npc(self.cache, 188252)
        with self.assertRaisesRegex(ValueError, 'does not belong'):
            select_assignment(self.tables, 2519, page['markers'][0])


if __name__ == '__main__':
    unittest.main()
