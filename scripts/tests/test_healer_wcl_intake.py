import importlib.util
import json
import sqlite3
import subprocess
import sys
import tempfile
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
    def test_opening_shield_balance_requires_a_final_partial_hit(self):
        aura = {'source_actor': 1, 'target_actor': 1, 'spell_id': 1235568,
                'stacks': 1, 'evidence': 'first_removebuff', 'remove_ms': 100}
        damage = [{'sourceID': 10, 'targetID': 1, 'abilityGameID': 1, 'fight_ms': 90,
                   'timestamp': 90, 'amount': 0, 'absorbed': 60, 'maxHitPoints': 1000},
                  {'sourceID': 10, 'targetID': 1, 'abilityGameID': 1, 'fight_ms': 100,
                   'timestamp': 100, 'amount': 40, 'absorbed': 40, 'maxHitPoints': 1000}]
        healing = [{'type': 'absorbed', 'sourceID': 1, 'targetID': 1, 'abilityGameID': 1235568,
                    'fight_ms': 90, 'timestamp': 90, 'amount': 60, 'attackerID': 10, 'extraAbilityGameID': 1},
                   {'type': 'absorbed', 'sourceID': 1, 'targetID': 1, 'abilityGameID': 1235568,
                    'fight_ms': 100, 'timestamp': 100, 'amount': 40, 'attackerID': 10, 'extraAbilityGameID': 1}]
        config = {'actors': [1, 2, 3, 4, 5]}
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE spell_effect (SpellID INTEGER, DifficultyID INTEGER, EffectAura INTEGER, EffectAuraPeriod INTEGER)')
        db.execute('INSERT INTO spell_effect VALUES (1235568, 0, 69, 0)')
        buffs = [{'type': 'removebuff', 'sourceID': 1, 'targetID': 1, 'abilityGameID': 1235568,
                  'fight_ms': 100, 'absorb': 0}]
        result = intake.infer_exhausted_opening_absorbs([aura], damage, healing, config, db, buffs)
        self.assertEqual(result[0]['absorb_balance']['amount'], 100)
        self.assertEqual(result[0]['absorb_balance']['max_hp'], 1000)
        self.assertNotIn('absorb_balance', aura)
        damage[-1]['amount'] = 0
        result = intake.infer_exhausted_opening_absorbs([aura], damage, healing, config, db, buffs)
        self.assertNotIn('absorb_balance', result[0])
        aura['remove_ms'] = 300
        damage[-1]['amount'] = 40
        result = intake.infer_exhausted_opening_absorbs([aura], damage, healing, config, db, buffs)
        self.assertNotIn('absorb_balance', result[0])

    def test_external_shield_grants_and_static_opening_remainders(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE spell_effect (SpellID INTEGER, DifficultyID INTEGER, EffectAura INTEGER, EffectAuraPeriod INTEGER)')
        db.executemany('INSERT INTO spell_effect VALUES (?, 0, ?, ?)',
                       [(1255200, 69, 0), (1266619, 69, 0), (1266619, 226, 200)])
        config = {'actors': [1, 2, 3, 4, 5], 'duration_ms': 5000}
        damage = [{'sourceID': 10, 'targetID': 1, 'fight_ms': 0, 'maxHitPoints': 100000}]
        buffs = [{'type': 'applybuff', 'sourceID': 2, 'targetID': 1, 'abilityGameID': 1255200,
                  'fight_ms': 1000, 'absorb': 9000},
                 {'type': 'refreshbuff', 'sourceID': 2, 'targetID': 1, 'abilityGameID': 1255200,
                  'fight_ms': 2000, 'absorb': 12000},
                 {'type': 'applybuff', 'sourceID': 1, 'targetID': 1, 'abilityGameID': 1255200,
                  'fight_ms': 2500, 'absorb': 15000}]
        grants = intake.logged_external_shields(buffs, damage, [], db, config)
        self.assertEqual([(g['time_ms'], g['source_actor'], g['amount']) for g in grants],
                         [(1000, 2, 9000), (2000, 2, 12000)])
        auras = [{'source_actor': 2, 'target_actor': 1, 'spell_id': spell, 'stacks': 1,
                  'evidence': 'first_removebuff', 'remove_ms': 100} for spell in (1255200, 1266619)]
        removed = [{'type': 'removebuff', 'sourceID': 2, 'targetID': 1, 'abilityGameID': spell,
                    'fight_ms': 100, 'absorb': 2000} for spell in (1255200, 1266619)]
        consumed = [{'type': 'absorbed', 'sourceID': 2, 'targetID': 1, 'abilityGameID': 1255200,
                     'fight_ms': 50, 'amount': 4000}]
        opening = intake.infer_exhausted_opening_absorbs(auras, damage, consumed, config, db, removed)
        self.assertEqual(opening[0]['absorb_balance']['amount'], 6000)
        self.assertEqual(opening[0]['absorb_balance']['remaining_on_removal'], 2000)
        self.assertNotIn('absorb_balance', opening[1], 'do not infer an unobserved decaying pool')
        empty = dict(auras[0], remove_ms=0)
        empty_removed = dict(removed[0], fight_ms=0, absorb=0)
        opening = intake.infer_exhausted_opening_absorbs([empty], damage, [], config, db, [empty_removed])
        self.assertEqual(opening[0]['skip_reason'], 'observed_empty_shield_removed_at_pull_start')

    def test_default_healer_output_is_byte_identical(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            sqlite3.connect(path / 'world.db').close()
            events = [{'targetID': actor, 'sourceID': 10, 'abilityGameID': 1,
                       'timestamp': 100, 'fight_ms': 0, 'hitType': 1, 'amount': 50,
                       'absorbed': 10, 'unmitigatedAmount': 100, 'maxHitPoints': 1000,
                       'hitPoints': 950, 'resourceActor': 2} for actor in range(1, 6)]
            combatants = [{'sourceID': actor, 'auras': []} for actor in range(1, 6)]
            for name, rows in [('events', events), ('healing', []),
                               ('combatants', combatants), ('buffs', []), ('debuffs', [])]:
                (path / f'{name}.jsonl').write_text(''.join(json.dumps(row) + '\n' for row in rows))
            config = {'actors': [1, 2, 3, 4, 5], 'party_specs': [73, 264, 70, 103, 261],
                      'provenance': 'compatibility fixture', 'bloodlust': [], 'duration_ms': 5000}
            (path / 'config.json').write_text(json.dumps(config))
            args = [sys.executable, str(SCRIPTS / 'healer_wcl_intake.py'),
                    str(path / 'events.jsonl'), str(path / 'healing.jsonl'),
                    str(path / 'world.db'), str(path / 'config.json'), str(path / 'output.json'),
                    '--combatants', str(path / 'combatants.jsonl'), '--buffs', str(path / 'buffs.jsonl'),
                    '--debuffs', str(path / 'debuffs.jsonl')]
            expected = (SCRIPTS / 'tests/fixtures/healer-wcl-output.json').read_bytes()
            for mode in [[], ['--mode', 'healer']]:
                subprocess.run(args + mode, check=True, capture_output=True)
                self.assertEqual((path / 'output.json').read_bytes(), expected)

    def test_tank_raw_intake_keeps_outcomes_and_counts_exclusions(self):
        db = sqlite3.connect(':memory:')
        config = {'actors': [1, 2, 3, 4, 5], 'party_specs': [73, 264, 70, 103, 261],
                  'provenance': 'test', 'bloodlust': [], 'duration_ms': 5000}
        common = {'targetID': 1, 'sourceID': 10, 'abilityGameID': 1,
                  'fight_ms': 100, 'hitType': 1, 'maxHitPoints': 1000}
        events = [dict(common, amount=50, absorbed=20, mitigated=30, unmitigatedAmount=100),
                  dict(common, fight_ms=200, hitType=7, amount=0),
                  dict(common, fight_ms=300, amount=50, unmitigatedAmount=40),
                  dict(common, fight_ms=400, sourceID=2, amount=15)]
        healing = [{'type': 'heal', 'fight_ms': 150, 'targetID': 1, 'sourceID': 2,
                    'abilityGameID': 123, 'amount': 25},
                   {'type': 'heal', 'fight_ms': 160, 'targetID': 1, 'sourceID': 1,
                    'abilityGameID': 456, 'amount': 10}]
        result = intake.convert_tank(events, healing, db, config)
        self.assertEqual([hit['raw'] for hit in result['hits']], [100, None])
        self.assertEqual([hit['hit_type'] for hit in result['hits']], [1, 7])
        self.assertEqual(result['excluded']['known_raw'], 40)
        self.assertEqual(result['excluded']['friendly_observed_pressure'], 15)
        self.assertEqual(result['included_raw'], 100)
        self.assertEqual(result['external_heals'][0]['amount'], 25)
        self.assertEqual(len(result['external_heals']), 1)

    def test_replayed_casts_take_self_applied_vampiric_blood_times(self):
        buffs = [
            {'fight_ms': 38927, 'sourceID': 4, 'targetID': 4, 'abilityGameID': 55233, 'type': 'applybuff'},
            {'fight_ms': 4323, 'sourceID': 4, 'targetID': 4, 'abilityGameID': 55233, 'type': 'applybuff'},
            {'fight_ms': 14323, 'sourceID': 4, 'targetID': 4, 'abilityGameID': 55233, 'type': 'removebuff'},
            {'fight_ms': 5000, 'sourceID': 2, 'targetID': 4, 'abilityGameID': 55233, 'type': 'applybuff'},
            {'fight_ms': 6000, 'sourceID': 4, 'targetID': 4, 'abilityGameID': 48707, 'type': 'applybuff'},
            {'fight_ms': 80000, 'sourceID': 4, 'targetID': 4, 'abilityGameID': 55233, 'type': 'applybuff'},
        ]
        casts = intake.replayed_casts(buffs, [4, 2, 1, 5, 3], 75000)
        self.assertEqual(casts, [
            {'time_ms': 4323, 'slot': 0, 'spell_id': 55233, 'evidence': 'applybuff'},
            {'time_ms': 38927, 'slot': 0, 'spell_id': 55233, 'evidence': 'applybuff'},
        ])

    def test_opening_auras_combine_combatants_and_first_removals(self):
        combatants = [{'sourceID': 2, 'auras': [{'source': 1, 'ability': 465, 'stacks': 1}]}]
        buffs = [
            {'fight_ms': 2, 'sourceID': 2, 'targetID': 2, 'abilityGameID': 1256579, 'type': 'removebuff'},
            {'fight_ms': 3, 'sourceID': 1, 'targetID': 1, 'abilityGameID': 195181, 'type': 'removebuffstack', 'stack': 6},
            {'fight_ms': 4, 'sourceID': 2, 'targetID': 2, 'abilityGameID': 123, 'type': 'applybuff'},
            {'fight_ms': 5, 'sourceID': 2, 'targetID': 2, 'abilityGameID': 123, 'type': 'removebuff'},
            {'fight_ms': 6, 'sourceID': 1, 'targetID': 2, 'abilityGameID': 465, 'type': 'removebuff'},
        ]
        auras = intake.opening_auras(combatants, buffs, [1, 2, 3, 4, 5])
        self.assertEqual(auras, [
            {'source_actor': 1, 'target_actor': 2, 'spell_id': 465, 'stacks': 1, 'evidence': 'combatant_info', 'remove_ms': 6},
            {'source_actor': 2, 'target_actor': 2, 'spell_id': 1256579, 'stacks': 1, 'evidence': 'first_removebuff', 'remove_ms': 2},
            {'source_actor': 1, 'target_actor': 1, 'spell_id': 195181, 'stacks': 7, 'evidence': 'first_removebuffstack'},
        ])

    def test_subtracts_observed_stagger_from_initial_hit_and_replays_tick(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE spell_misc (SpellID INTEGER, DifficultyID INTEGER, SchoolMask INTEGER)')
        db.execute('CREATE TABLE spell_categories (SpellID INTEGER, DifficultyID INTEGER, DefenseType INTEGER)')
        db.executemany('INSERT INTO spell_misc VALUES (?, 0, 1)', [(1306338,), (124255,)])
        db.execute('INSERT INTO spell_categories VALUES (1306338, 0, 2)')
        config = {'actors': [6, 221, 1, 225, 231], 'party_specs': [268, 1468, 62, 262, 262],
                  'provenance': 'INFERRED_WCL test fight 1 pull 1', 'bloodlust': [],
                  'duration_ms': 1000}
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
                  'timestamp': 99, 'fight_ms': 0, 'amount': 200}]
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

    def test_excludes_unknown_source_priest_shadow_word_death_backlash(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE spell_misc (SpellID INTEGER, DifficultyID INTEGER, SchoolMask INTEGER)')
        db.execute('CREATE TABLE spell_categories (SpellID INTEGER, DifficultyID INTEGER, DefenseType INTEGER)')
        db.execute('INSERT INTO spell_misc VALUES (1306890, 0, 8)')
        config = {'actors': [1, 2, 3, 4, 5], 'party_specs': [104, 270, 258, 253, 252],
                  'provenance': 'INFERRED_WCL test fight 1 pull 1', 'bloodlust': [],
                  'duration_ms': 1000}
        events = [{'targetID': actor, 'sourceID': -1, 'abilityGameID': 1306890,
                   'timestamp': 100, 'fight_ms': 0, 'hitType': 1, 'amount': 10,
                   'maxHitPoints': 1000} for actor in config['actors']]
        events.append({'targetID': 3, 'sourceID': -1, 'abilityGameID': 32409,
                       'timestamp': 101, 'fight_ms': 1, 'hitType': 1, 'amount': 50})
        events.append({'targetID': 3, 'sourceID': -1, 'abilityGameID': 999999,
                       'timestamp': 1100, 'fight_ms': 1000, 'hitType': 2, 'amount': 50})
        result = intake.convert(events, [], db, config)
        self.assertEqual(result['excluded_self'], 1)
        self.assertEqual([hit['spell_id'] for hit in result['hits']], [1306890] * 5)
        self.assertEqual(result['observed_self'][0]['spell_id'], 32409)

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
        report['metrics']['first_death_seconds'] = 10
        trace['hits'].append({'time_ms': 11000, 'slot': 0, 'amount': 10, 'absorbed': 0, 'max_hp': 100})
        result = gate.evaluate(trace, report)
        self.assertTrue(result['passed'])
        self.assertEqual(result['cutoff_ms'], 10000)
        self.assertEqual(result['rows'][0]['end_ms'], 10000)
        self.assertEqual(result['dead_target_events'], 1)
        trace['hits'].append({'time_ms': 9000, 'slot': 0, 'amount': 100, 'absorbed': 0, 'max_hp': 100})
        report['wcl_intake']['health_samples'].append({'time_ms': 9000, 'slot': 0, 'native_hp_pct': 0,
            'wcl_hp_pct': 50, 'native_loss_pct': 5, 'native_absorbed_pct': 0})
        result = gate.evaluate(trace, report)
        self.assertTrue(result['passed'])
        self.assertEqual(result['cutoff_ms'], 9000)


if __name__ == '__main__':
    unittest.main()
