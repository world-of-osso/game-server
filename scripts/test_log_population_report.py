import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('report', Path(__file__).with_name('log_population_report.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReportTest(unittest.TestCase):
    def test_reports_unknown_unique_spawns_and_retains_patrol_gap(self):
        populations = [{'map': 2521, 'runs': 3, 'npcs': [{}, {}], 'authored': {
            'tdb_creature': {'count': 2}, 'content_creature': {'count': 3}}}]
        clusters = [{'map': 2521, 'ambiguous_match': True}]
        safe = [{'map': 2521, 'npc_id': 123, 'classification': 'patrol_candidate',
                 'observations': [{}, {}, {}], 'spread_xy': 8.0, 'gap': 'unverified'}]
        report = module.coverage(populations, clusters, safe, {'2521': {'POSITION_Z_UNRESOLVED': 1}},
                                 {'2521': {'imported': 1, 'already_present': 2,
                                           'templates_from_tdb': [123], 'templates_absent': []}})[0]
        self.assertIsNone(report['unique_spawns_proven'])
        self.assertEqual((report['imported'], report['already_present_nearby']), (1, 2))
        self.assertEqual(report['patrol_candidates'][0]['sample_count'], 3)
        self.assertEqual(report['patrol_candidates'][0]['gap'], 'unverified')
        self.assertEqual(report['fitted_paths'], 0)


if __name__ == '__main__':
    unittest.main()
