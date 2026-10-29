import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('merge', Path(__file__).with_name('merge_log_zone.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def unit(guid, time, x):
    return {'npc_id': 123, 'spawn_uid': guid, 'first_seen': time, 'first_event': 'SPELL_DAMAGE',
            'first_encounter_id': None, 'seen_outside_encounter': True, 'summoned': False,
            'player_controlled': False, 'control_flags_invalid': False,
            'first_position': {'guid': guid, 'x': x, 'y': 20, 'time': time, 'event': 'SPELL_DAMAGE',
                               'ui_map_id': 9, 'facing': 1, 'level': 90}}


class MergeTest(unittest.TestCase):
    def test_earliest_exact_guid_position_retains_zone_source(self):
        first = '10/3/2026 20:23:31.000-5'
        later = '10/3/2026 20:23:41.000-5'
        keys = [{'file': 'a', 'map': 2521, 'start': later, 'creatures': {'g': unit('g', later, 9)}}]
        zones = [{'file': 'a', 'map': 2521, 'start': first, 'creatures': {'g': unit('g', first, 3),
                                                                           'other': unit('other', first, 4)}}]
        merged, report = module.merge(keys, zones)
        self.assertEqual(merged[0]['creatures']['g']['first_position']['x'], 3)
        self.assertEqual(merged[0]['creatures']['g']['position_source_run_start'], first)
        self.assertEqual(list(merged[1]['creatures']), ['other'])
        self.assertEqual((report['2521']['matched_exact_guid'], report['2521']['zone_only_guids']), (1, 1))
        self.assertEqual(keys[0]['creatures']['g']['first_position']['x'], 9)


if __name__ == '__main__':
    unittest.main()
