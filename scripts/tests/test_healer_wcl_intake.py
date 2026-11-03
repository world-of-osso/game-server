import importlib.util
import sqlite3
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


intake = load('healer_wcl_intake')
gate = load('healer_wcl_intake_gate')


class WclIntakeTest(unittest.TestCase):
    def test_subtracts_observed_stagger_from_initial_hit_and_replays_tick(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE spell_misc (SpellID INTEGER, DifficultyID INTEGER, SchoolMask INTEGER)')
        db.execute('CREATE TABLE spell_categories (SpellID INTEGER, DifficultyID INTEGER, DefenseType INTEGER)')
        db.executemany('INSERT INTO spell_misc VALUES (?, 0, 1)', [(1306338,), (124255,)])
        db.execute('INSERT INTO spell_categories VALUES (1306338, 0, 2)')
        config = {'actors': [6, 221, 1, 225, 231], 'party_specs': [268, 1468, 62, 262, 262],
                  'provenance': 'INFERRED_WCL test fight 1 pull 1', 'bloodlust': []}
        events = []
        for actor in config['actors']:
            events.append({'targetID': actor, 'sourceID': 413, 'sourceInstance': 1,
                           'abilityGameID': 1306338, 'timestamp': 100, 'fight_ms': 0,
                           'hitType': 1, 'amount': 100, 'absorbed': 300, 'maxHitPoints': 1000})
        events.append({'targetID': 6, 'sourceID': 6, 'abilityGameID': 124255, 'timestamp': 200,
                       'fight_ms': 100, 'hitType': 1, 'tick': True, 'amount': 50,
                       'absorbed': 0, 'maxHitPoints': 1000})
        heals = [{'type': 'absorbed', 'abilityGameID': 115069, 'targetID': 6,
                  'attackerID': 413, 'attackerInstance': 1, 'extraAbilityGameID': 1306338,
                  'timestamp': 99, 'amount': 200}]
        result = intake.convert(events, heals, db, config)
        first, tick = result['hits'][0], result['hits'][-1]
        self.assertEqual((first['amount'], first['absorbed'], first['deferred_absorbed']), (100, 100, 200))
        self.assertEqual((tick['spell_id'], tick['amount'], tick['deferred']), (124255, 50, True))
        self.assertEqual(result['excluded_self'], 0)

    def test_uses_time_varying_max_health_and_marks_future_observation(self):
        events = [{'sourceID': 6, 'targetID': 413, 'resourceActor': 1, 'fight_ms': 100,
                   'maxHitPoints': 1000, 'hitPoints': 800},
                  {'sourceID': 413, 'targetID': 6, 'resourceActor': 2, 'fight_ms': 200,
                   'maxHitPoints': 1500, 'hitPoints': 900}]
        observations = intake.read_health_observations(events, {6: 0})
        self.assertEqual(intake.max_health_at(observations, 6, 50), (1000, 100))
        self.assertEqual(intake.max_health_at(observations, 6, 150), (1000, 100))
        self.assertEqual(intake.max_health_at(observations, 6, 200), (1500, 200))

    def test_wcl_falling_sentinel_is_not_db2_spell_three(self):
        db = sqlite3.connect(':memory:')
        self.assertEqual(intake.read_spell_metadata(db, 3), (1, None))

    def test_rejects_ambiguous_or_unmatched_absorb_receipts(self):
        event = {'targetID': 6, 'sourceID': 413, 'sourceInstance': 1,
                 'abilityGameID': 1306338, 'timestamp': 100, 'absorbed': 300}
        absorb = {'type': 'absorbed', 'abilityGameID': 115069, 'targetID': 6,
                  'attackerID': 413, 'attackerInstance': 1, 'extraAbilityGameID': 1306338,
                  'timestamp': 99, 'amount': 200}
        with self.assertRaises(ValueError):
            intake.match_deferred_absorbs([event, event], [absorb])
        with self.assertRaises(ValueError):
            intake.match_deferred_absorbs([], [absorb])

    def test_gate_requires_each_window_and_no_dead_target_events(self):
        trace = {'hits': [{'time_ms': 1, 'slot': 0, 'amount': 10, 'absorbed': 10, 'max_hp': 100}]}
        report = {'duration_seconds': 15, 'wcl_intake': {'events_on_dead_targets_not_replayed': 0,
                  'health_samples': [{'time_ms': 1, 'slot': 0, 'wcl_hp_pct': 80,
                                      'native_hp_pct': 80, 'native_loss_pct': 12, 'native_absorbed_pct': 8}]},
                  'metrics': {'first_death_seconds': None}}
        self.assertTrue(gate.evaluate(trace, report)['passed'])
        report['wcl_intake']['health_samples'][0]['native_loss_pct'] = 5
        self.assertFalse(gate.evaluate(trace, report)['passed'])
        report['wcl_intake']['health_samples'][0]['native_loss_pct'] = 12
        report['wcl_intake']['events_on_dead_targets_not_replayed'] = 1
        self.assertFalse(gate.evaluate(trace, report)['passed'])


if __name__ == '__main__':
    unittest.main()
