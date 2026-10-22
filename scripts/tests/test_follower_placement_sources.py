"""Read actual authenticated70448/cached NPC evidence without network requests."""
import copy
import json
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import read_cached_npc, read_db2, read_source_floor_support, select_assignment, transform_marker


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

    def test_wratheye_source_wmo_floor_rejects_roofs_wrong_floor_and_changed_evidence(self):
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/brackenhide.json'
        record = next(r for r in json.loads(manifest_path.read_text())['entries'] if r['entry'] == 186121)
        marker = read_cached_npc(self.cache, 186121)['markers'][0]
        assignments = select_assignment(self.tables, 2520, marker)
        observed = [111.17662182231342, 246.987919, 263.096514]
        self.assertEqual(read_source_floor_support(record, marker, assignments, observed), [observed[0]])
        with self.assertRaisesRegex(ValueError, 'marker floor'):
            read_source_floor_support(record, dict(marker, floor=0), assignments, observed)
        damaged = copy.deepcopy(record)
        damaged['source_floor_evidence']['source_files'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'evidence file changed'):
            read_source_floor_support(damaged, marker, assignments, observed)
        damaged = copy.deepcopy(record)
        damaged['source_floor_z_bounds'][1] += 1
        with self.assertRaisesRegex(ValueError, 'recorded WMO transform'):
            read_source_floor_support(damaged, marker, assignments, observed)

    def test_journal_marker_wins_over_rounded_wowhead_and_reports_large_disagreement(self):
        from follower_placements import derive_placement, read_binding_tables
        binding_tables, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        tables = self.tables | binding_tables
        for entry, map_id, journal_id, expected in [
            (194181, 2526, 2509, (1815.2512440319297, -2783.9755457624997)),
            (190485, 2521, 2503, None),
        ]:
            with self.subTest(entry=entry):
                page = read_cached_npc(self.cache, entry)
                record = dict(entry=entry, journal_encounter_id=journal_id, cache_sha256=page['cache_sha256'])
                manifest = dict(map_id=map_id, placement_policy='FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP')
                result = derive_placement(record, manifest, tables, self.cache, [], Path('unused'), Path('unused'),
                                          dict(LocX=0, LocY=0, LocZ=0))
                if expected is not None:
                    self.assertEqual(tuple(result['wow_xy']), expected)
                else:
                    self.assertGreater(result['wowhead_comparison']['distance_yards'], 5)
                    self.assertEqual(result['wowhead_comparison']['finding'], 'FOLLOWER_JOURNAL_WOWHEAD_XY_DISAGREEMENT')
                self.assertEqual(result['page']['source_table'], 'JournalEncounter')
                self.assertEqual(result['page']['source_row']['ID'], str(journal_id))

    def test_halls_upward_floor_triangles_exclude_ceilings_and_ambiguous_nav_surfaces(self):
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/halls-of-infusion.json'
        records = {r['entry']: r for r in json.loads(manifest_path.read_text())['entries']}
        for entry, observed, expected in [
            (189722, [-190.51663351740984, -67.90195139447695, 25.439383706251334], -67.90195139447695),
            (189727, [-190.64583, -100.40612940495758, 9.387790624059264], -100.40612940495758),
        ]:
            with self.subTest(entry=entry):
                record = records[entry]
                marker = read_cached_npc(self.cache, entry)['markers'][0]
                assignments = select_assignment(self.tables, 2527, marker)
                self.assertEqual(read_source_floor_support(record, marker, assignments, observed), [expected])
                ambiguous = observed + [expected + 0.1]
                with self.assertRaisesRegex(ValueError, 'one baked candidate'):
                    read_source_floor_support(record, marker, assignments, ambiguous)
                two_upwards = copy.deepcopy(record)
                two_upwards['source_floor_evidence']['triangles'].append(copy.deepcopy(record['source_floor_evidence']['triangles'][1]))
                with self.assertRaisesRegex(ValueError, 'one upward'):
                    read_source_floor_support(two_upwards, marker, assignments, observed)
                damaged = copy.deepcopy(record)
                damaged['source_floor_evidence']['source_files'][0]['sha256'] = '0' * 64
                with self.assertRaisesRegex(ValueError, 'evidence file changed'):
                    read_source_floor_support(damaged, marker, assignments, observed)
                with self.assertRaisesRegex(ValueError, 'marker floor'):
                    read_source_floor_support(record, dict(marker, floor=0), assignments, observed)

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
