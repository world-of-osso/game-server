import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('packs', Path(__file__).with_name('log_population_packs.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def sample(file, guid, x, time):
    return {'file': file, 'run_start': 'start', 'map': 2521, 'guid': guid, 'x': x, 'y': 0,
            'ui_map_id': 9, 'seconds_from_start': time}


class PackTest(unittest.TestCase):
    def test_cross_run_unique_packs_and_no_same_run_ambiguity(self):
        a = sample('a', 'Creature-0-1-2521-1-100-1', 0, 10)
        b = sample('a', 'Creature-0-1-2521-1-101-2', 2, 11)
        c = sample('b', 'Creature-0-1-2521-2-100-3', 40, 20)
        d = sample('b', 'Creature-0-1-2521-2-101-4', 42, 21)
        packs = module.build_packs([a, b, c, d])
        candidates = module.match_packs(packs)
        self.assertEqual(len(candidates), 1)
        self.assertEqual(candidates[0]['npc_ids'], (100, 101))
        self.assertEqual(len(candidates[0]['observations']), 2)
        self.assertEqual(candidates[0]['gap'], 'PACK_IDENTITY_NAVMESH_AND_ENGAGEMENT_UNVERIFIED')
        extra = [sample('a', 'Creature-0-1-2521-1-100-5', 80, 30),
                 sample('a', 'Creature-0-1-2521-1-101-6', 82, 31)]
        self.assertEqual(module.match_packs(module.build_packs([a, b, c, d, *extra])), [])


if __name__ == '__main__':
    unittest.main()
