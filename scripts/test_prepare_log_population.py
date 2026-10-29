import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('prepare', Path(__file__).with_name('prepare_log_population.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PrepareTest(unittest.TestCase):
    def test_joins_resolved_floor_and_excludes_unresolved_and_summon(self):
        guid = 'Creature-0-1-2521-42-123-ABCD'
        run = {'file': 'a', 'start': 'start', 'map': 2521,
               'creatures': {guid: {'npc_id': 123, 'seen_outside_encounter': True}}, 'encounters': []}
        observation = {'file': 'a', 'run_start': 'start', 'guid': guid, 'x': 10, 'y': 20,
                       'time': '10/3/2026 20:23:32.000-5', 'event': 'SPELL_DAMAGE',
                       'facing': 1.5, 'level': 90, 'summoned': False}
        cluster = {'map': 2521, 'npc_id': 123, 'ui_map_id': 2095, 'ambiguous_match': False,
                   'observations': [observation]}
        request = {'file': 'a', 'run_start': 'start', 'guid': guid, 'map': 2521,
                   'x': 10, 'y': 20, 'ui_map_id': 2095}
        packs = [{'file': 'a', 'run_start': 'start', 'npc_ids': [123, 456], 'members': [{'guid': guid}]}]
        records, gaps = module.prepare([run], [cluster], [request], [{'z': 30, 'method': 'single_candidate'}], packs)
        self.assertEqual((records[0]['x'], records[0]['z'], records[0]['z_method']), (10, 30, 'single_candidate'))
        self.assertEqual(records[0]['pack_npc_ids'], [123, 456])
        self.assertEqual(records[0]['source_guid'], guid)
        self.assertEqual(records[0]['observation_event'], 'SPELL_DAMAGE')
        self.assertEqual(gaps, {})
        cluster.update(ambiguous_match=True, candidate_centroid={'x': 10.5, 'y': 20},
                       candidate_spread_yards=1, assignment_margin_yards=0.0)
        centroid_request = dict(request, cluster_index=0, x=10.5)
        anchor = [{'table': 'tdb_creature', 'guid': 123}]
        records, gaps = module.prepare([run], [cluster], [request], [{'gap': {'kind': 'named_gap'}}], packs,
                                       [centroid_request], [{'z': 31, 'method': 'authored_neighbor_anchor', 'anchor_rows': anchor}])
        self.assertEqual((records[0]['x'], records[0]['z'], records[0]['identity_ambiguous']), (10.5, 31, True))
        self.assertEqual(records[0]['floor_anchor_rows'], anchor)
        cluster['ambiguous_match'] = False
        records, gaps = module.prepare([run], [cluster], [request], [{'gap': {'kind': 'named_gap', 'candidates': []}}])
        self.assertEqual(records, [])
        self.assertEqual(gaps['2521']['POSITION_Z_UNRESOLVED'], 1)
        cluster['observations'][0]['summoned'] = True
        records, gaps = module.prepare([run], [cluster], [request], [{'z': 30, 'method': 'single_candidate'}])
        self.assertEqual(records, [])
        self.assertEqual(gaps['2521']['SUMMON_DESTINATION'], 1)
        with self.assertRaises(ValueError):
            module.prepare([run], [cluster], [request], [])


if __name__ == '__main__':
    unittest.main()
