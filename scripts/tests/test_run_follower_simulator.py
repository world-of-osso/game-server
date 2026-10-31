"""The headless runner uses catalog levels and records initialization failures."""
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location('follower_simulator', SCRIPTS / 'run-follower-simulator.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class FollowerSimulatorInputs(unittest.TestCase):
    def test_catalog_level_comes_from_the_requested_map(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'world.db'
            with sqlite3.connect(path) as db:
                db.executescript('''CREATE TABLE follower_lfg_dungeons(id, map_id, content_tuning_id);
                    CREATE TABLE content_tuning(ID, MaxLevelSquish);
                    INSERT INTO follower_lfg_dungeons VALUES(1,2451,2938),(2,2651,2718);
                    INSERT INTO content_tuning VALUES(2938,70),(2718,80);''')
            self.assertEqual(runner.read_catalog_level(path, 2451), 70)
            self.assertEqual(runner.read_catalog_level(path, 2651), 80)
            with self.assertRaisesRegex(ValueError, 'map2993 missing valid catalog level'):
                runner.read_catalog_level(path, 2993)

    def test_missing_entrance_produces_an_explicit_blocked_result(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            runner.write_blocked_report(output, 2993, 1, 'map2993 missing entrance')
            report = json.loads((output / 'run-1.json').read_text())
            self.assertEqual(report['status'], 'blocked')
            self.assertEqual(report['detail'], 'map2993 missing entrance')
            self.assertEqual(report['simulated_seconds'], 0.0)
            self.assertIsNone(report['encounters_total'])

    def test_blocked_dawnbreaker_identifies_unsourced_deck_without_claiming_replay(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            runner.write_blocked_report(output, 2662, 1, 'missing flight path')
            report = json.loads((output / 'run-1.json').read_text())
            self.assertEqual(report['deck_population_unsourced'], {
                'report': 'x3HzDRn6yVjqCbt8', 'fight': 7, 'replay_executed': False,
                'transport_start_ms': 86000, 'commander_death_ms': 162380,
                'radiant_light_ms': 166807,
            })
            self.assertEqual(report['encounters_killed'], 0)


if __name__ == '__main__':
    unittest.main()
