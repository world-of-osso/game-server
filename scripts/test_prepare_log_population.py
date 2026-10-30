import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('prepare', Path(__file__).with_name('prepare_log_population.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PrepareTest(unittest.TestCase):
    def test_missing_and_ambiguous_boss_positions_remain_named_gaps(self):
        run = {'file': 'a', 'start': 'start', 'map': 2813,
               'creatures': {'g': {'npc_id': 237415, 'seen_outside_encounter': True,
                                   'position_gap': 'BOSS_PRE_PULL_POSITION_UNOBSERVED'}},
               'encounters': [{'known_boss_npc_ids': [237415]}]}
        cluster = {'map': 2813, 'npc_id': 237415, 'ambiguous_match': True,
                   'candidate_centroid': {'x': 9151.12, 'y': -4944.43}, 'observations': []}
        records, gaps = module.prepare([run], [cluster], [], [])
        self.assertEqual(records, [])
        self.assertEqual(gaps['2813']['BOSS_PRE_PULL_POSITION_UNOBSERVED'], 1)
        self.assertEqual(gaps['2813']['BOSS_PRE_PULL_IDENTITY_AMBIGUOUS'], 1)

    def test_cli_combines_key_and_zone_floor_batches(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            run = {'file': 'a', 'start': 'zone', 'map': 2521,
                   'creatures': {'g': {'npc_id': 123, 'seen_outside_encounter': True}}, 'encounters': []}
            obs = {'file': 'a', 'run_start': 'zone', 'guid': 'g', 'x': 10, 'y': 20,
                   'time': '10/3/2026 20:23:32.000-5', 'event': 'SPELL_DAMAGE',
                   'facing': 1.5, 'level': 90, 'summoned': False}
            cluster = {'map': 2521, 'npc_id': 123, 'ui_map_id': 9, 'ambiguous_match': False, 'observations': [obs]}
            request = {'file': 'a', 'run_start': 'zone', 'guid': 'g', 'map': 2521, 'x': 10, 'y': 20, 'ui_map_id': 9}
            files = {'census': {'runs': [run]}, 'clusters': [cluster], 'requests': [], 'resolved': [],
                     'extra-requests': [request], 'extra-resolved': [{'z': 30, 'method': 'single_candidate'}]}
            args = []
            for name, data in files.items():
                path = root / (name + '.json')
                path.write_text(json.dumps(data))
                args += ['--' + name, str(path)]
            output, gaps = root / 'output.json', root / 'gaps.json'
            subprocess.run([sys.executable, '-I', str(Path(__file__).with_name('prepare_log_population.py')),
                            *args, '--output', str(output), '--gaps', str(gaps)], check=True)
            self.assertEqual(json.loads(output.read_text())[0]['z'], 30)

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
