import importlib.util
import io
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('log_population_census', Path(__file__).with_name('log_population_census.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
census = module.census


class CensusTest(unittest.TestCase):
    def test_minimum_cost_matching_is_not_greedy(self):
        assignment, cost = module.minimum_assignment([[1.0, 2.0], [1.1, 20.0]])
        self.assertEqual(assignment, [1, 0])
        self.assertAlmostEqual(cost, 3.1)
        assignment, cost = module.minimum_assignment([[float('inf'), 2.0], [1.1, 20.0]])
        self.assertEqual(assignment, [1, 0])
        self.assertAlmostEqual(cost, 3.1)

    def test_advanced_position_belongs_to_described_unit(self):
        fields = ['SPELL_DAMAGE', 'Creature-0-1-2521-42-123-ABCD', 'Boss', '0', '0',
                  'Creature-0-1-2521-42-456-DCBA', 'Add', '0', '0', '1', 'Hit', '1',
                  'Creature-0-1-2521-42-456-DCBA', '0', '100', '100', '0', '0', '0',
                  '0', '0', '0', '0', '0', '0', '0', '1433.58', '-155.52', '2095',
                  '5.1539', '90', '1']
        pos = module.advanced_position(fields)
        self.assertEqual(pos['guid'], 'Creature-0-1-2521-42-456-DCBA')
        self.assertEqual((pos['x'], pos['y'], pos['ui_map_id'], pos['level']),
                         (1433.58, -155.52, 2095, 90))
        fields[12] = 'Creature-0-1-2521-42-789-AAAA'
        self.assertIsNone(module.advanced_position(fields))
        fields[12] = 'Creature-0-1-2521-42-456-DCBA'
        fields[0] = 'SPELL_AURA_APPLIED'
        fields.insert(12, 'DEBUFF')
        self.assertEqual(module.advanced_position(fields)['guid'], fields[5])

    def test_cross_run_candidate_and_ambiguous_matches(self):
        def run(file, points):
            units = {}
            for guid, x in points:
                units[guid] = {'npc_id': 123, 'spawn_uid': guid, 'summoned': False,
                    'first_position': {'x': x, 'y': 0, 'ui_map_id': 9, 'time': 't',
                                       'event': 'SPELL_DAMAGE', 'facing': 1, 'level': 90}}
            return {'map': 2521, 'file': file, 'start': 'start', 'creatures': units}
        clusters = module.cluster_population([run('a', [('a1', 0)]),
                                              run('b', [('b1', 7)])])
        self.assertEqual(len(clusters), 1)
        self.assertEqual(clusters[0]['classification'], 'patrol_candidate')
        self.assertEqual(len(clusters[0]['unverified_candidate_path']), 2)
        self.assertEqual(clusters[0]['gap'], 'PARTIAL_PATROL_NAVMESH_AND_PHASE_UNVERIFIED')
        summoned = run('b', [('b1', 7)])
        summoned['creatures']['b1']['summoned'] = True
        self.assertEqual(len(module.cluster_population([run('a', [('a1', 0)]), summoned])[0]['observations']), 1)
        ambiguous = module.cluster_population([run('a', [('a1', 0), ('a2', 1)]),
                                               run('b', [('b1', 0.5)])])
        self.assertEqual(len(ambiguous), 2)
        self.assertTrue(all(cluster['ambiguous_match'] for cluster in ambiguous))
        self.assertEqual(ambiguous[0]['assignment_margin_yards'], 0.0)
        self.assertEqual(ambiguous[0]['classification'], 'insufficient_or_ambiguous')
        self.assertEqual(len(module.centroid_requests(ambiguous)), 2)
        self.assertLessEqual(ambiguous[0]['candidate_spread_yards'], 5.0)
        self.assertEqual(len({x['guid'] for x in ambiguous[0]['candidate_observations']}),
                         len(ambiguous[0]['candidate_observations']))
        distinct = module.cluster_population([run('a', [('a1', 0), ('a2', 8)]),
                                              run('b', [('b1', 0.1), ('b2', 8.1)])])
        self.assertEqual(len(distinct), 2)
        self.assertTrue(all(not cluster['ambiguous_match'] for cluster in distinct))
        self.assertTrue(all(len(cluster['observations']) == 2 for cluster in distinct))
        runs = [run('a', [('a1', 0)]), run('b', [('b1', 7)])]
        samples = [dict(runs[i]['creatures'][guid]['first_position'], file=runs[i]['file'],
                        run_start='start', guid=guid, seconds_from_start=2)
                   for i, guid in enumerate(('a1', 'b1'))]
        safe_clusters = module.cluster_safe_samples(runs, io.StringIO('\n'.join(map(json.dumps, samples))))
        self.assertEqual(safe_clusters[0]['classification'], 'patrol_candidate')

    def test_encounter_casts_and_known_boss_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'combat.txt'
            path.write_text(
                '10/3/2026 20:23:31.000-5  CHALLENGE_MODE_START,"Ruby Life Pools",2521,399,15,[]\n'
                '10/3/2026 20:23:32.000-5  ENCOUNTER_START,2609,"Boss",8,5,2521\n'
                '10/3/2026 20:23:33.500-5  SPELL_CAST_START,Creature-0-1-2521-42-123-ABCD,"Boss",0xa48,0,Player-1,"Player",0x511,0,456,"Spell",1\n'
                '10/3/2026 20:23:40.000-5  ENCOUNTER_END,2609,"Boss",8,5,1,8000\n')
            db = Path(directory) / 'world.db'
            with sqlite3.connect(db) as conn:
                conn.execute('CREATE TABLE raid_encounter_creature (map_id, encounter_id, creature_entry)')
                conn.execute('INSERT INTO raid_encounter_creature VALUES (2521, 2609, 123)')
            runs = census([path], {2521})
            module.annotate_bosses(runs, db)
        encounter = runs[0]['encounters'][0]
        self.assertTrue(encounter['success'])
        self.assertEqual(encounter['known_boss_npc_ids'], [123])
        self.assertEqual(encounter['casts'][0]['seconds_from_pull'], 1.5)
        self.assertTrue(encounter['casts'][0]['known_boss'])
        second = dict(encounter['casts'][0], seconds_from_pull=6.5)
        encounter['casts'].append(second)
        summary = module.summarize_encounters(runs)[0]
        self.assertEqual(summary['spells'][0]['first_cast_seconds'], [1.5])
        self.assertEqual(summary['spells'][0]['interval_range_seconds'], [5.0, 5.0])

    def test_addon_timers_are_matched_by_challenge_and_npc(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'TrashCDData.lua'
            path.write_text('    [399] = {\n            [123] = {\n                    [456] = {\n'
                            '                        first = 6.8,\n'
                            '                        cd = { 18.4, 20 },\n')
            timers = module.read_trash_timers(path)
            runs = [{'challenge_id': 399, 'creatures': {'g': {'npc_id': 123}}},
                    {'challenge_id': 400, 'creatures': {'g': {'npc_id': 123}}}]
            module.annotate_trash(runs, timers)
            timer = runs[0]['creatures']['g']['addon_trash_timers'][0]
            self.assertEqual((timer['spell_id'], timer['first_seconds'], timer['interval_seconds']),
                             (456, 6.8, [18.4, 20.0]))
            self.assertEqual((timer['source'], timer['line']), (str(path), 3))
            self.assertNotIn('addon_trash_timers', runs[1]['creatures']['g'])

    def test_position_samples_stop_when_unit_is_engaged(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'combat.txt'
            guid = 'Creature-0-1-2521-42-123-ABCD'
            advanced = f'{guid},0,100,100,0,0,0,0,0,0,0,0,0,0,10.0,20.0,2095,1.5,90'
            path.write_text(
                '10/3/2026 20:23:31.000-5  CHALLENGE_MODE_START,"Ruby Life Pools",2521,399,15,[]\n'
                f'10/3/2026 20:23:32.000-5  SPELL_PERIODIC_HEAL,{guid},"Mob",0xa48,0,{guid},"Mob",0xa48,0,1,"Idle",1,{advanced}\n'
                f'10/3/2026 20:23:33.000-5  SPELL_DAMAGE,Player-1,"Player",0x511,0,{guid},"Mob",0xa48,0,1,"Hit",1,{advanced},1\n'
                f'10/3/2026 20:23:34.000-5  SPELL_CAST_SUCCESS,{guid},"Mob",0xa48,0,0000000000000000,nil,0,0,1,"Idle",1,{advanced}\n'
            )
            samples = io.StringIO()
            runs = census([path], {2521}, samples)
        rows = [json.loads(line) for line in samples.getvalue().splitlines()]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['guid'], guid)
        self.assertEqual(rows[0]['seconds_from_start'], 1.0)
        safe = io.StringIO()
        module.filter_safe_samples(runs, io.StringIO(samples.getvalue()), safe)
        self.assertEqual(safe.getvalue(), '')
        module.filter_safe_samples(runs, io.StringIO(samples.getvalue()), safe, margin_seconds=0.5)
        self.assertEqual(len(safe.getvalue().splitlines()), 1)

    def test_floor_requests_keep_run_identity_and_neighbor_hints(self):
        position = {'x': 10.0, 'y': 20.0, 'ui_map_id': 2095}
        neighbor = {'x': 11.0, 'y': 20.0, 'ui_map_id': 2095, 'time': '3', 'event': 'SPELL_DAMAGE'}
        runs = [{'file': 'a.txt', 'start': '1', 'map': 2521,
                 'creatures': {'Creature-0-1-2521-42-123-ABCD': {
                     'first_position': position, 'position_neighbors': [neighbor]},
                     'Creature-0-1-2521-42-456-DCBA': {}}}]
        self.assertEqual(module.floor_requests(runs), [{
            'file': 'a.txt', 'run_start': '1', 'guid': 'Creature-0-1-2521-42-123-ABCD',
            'map': 2521, 'x': 10.0, 'y': 20.0, 'ui_map_id': 2095,
            'neighbor_hints': [neighbor]}])

    def test_zone_observations_stay_separate_from_challenge_runs(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'combat.txt'
            path.write_text(
                '1  ZONE_CHANGE,2521,"Ruby Life Pools",23\n'
                '2  SPELL_DAMAGE,Player-1,"Player",0x511,0,Creature-0-1-2521-42-123-ABCD,"Before",0xa48,0,1,"Hit",1\n'
                '3  CHALLENGE_MODE_START,"Ruby Life Pools",2521,399,15,[]\n'
                '4  SPELL_DAMAGE,Player-1,"Player",0x511,0,Creature-0-1-2521-42-456-DCBA,"Key",0xa48,0,1,"Hit",1\n'
                '5  CHALLENGE_MODE_END,"Ruby Life Pools",2521\n')
            zones = census([path], {2521}, zones_only=True)
            keys = census([path], {2521})
        self.assertEqual(zones[0]['session_kind'], 'zone')
        self.assertEqual([unit['npc_id'] for unit in zones[0]['creatures'].values()], [123])
        self.assertEqual([unit['npc_id'] for unit in keys[0]['creatures'].values()], [456])

    def test_runs_creatures_and_summon_destinations(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'combat.txt'
            path.write_text(
                '1  CHALLENGE_MODE_START,"Ruby Life Pools",2521,399,15,[]\n'
                '2  SPELL_DAMAGE,Player-1,"Player",0,0,Creature-0-1-2521-42-123-ABCD,"Mob",0,0,1,"Hit",1\n'
                '3  SPELL_SUMMON,Creature-0-1-2521-42-123-ABCD,"Mob",0,0,Creature-0-1-2521-42-456-DCBA,"Add",0,0,2,"Summon",1\n'
                '4  CHALLENGE_MODE_END,"Ruby Life Pools",2521\n'
                '5  SPELL_DAMAGE,Player-1,"Player",0,0,Creature-0-1-2521-42-789-FFFF,"Excluded",0,0,1,"Hit",1\n'
            )
            runs = census([path], {2521})
        self.assertEqual(len(runs), 1)
        creatures = runs[0]['creatures']
        self.assertEqual(len(creatures), 2)
        self.assertEqual(creatures['Creature-0-1-2521-42-123-ABCD']['instance'], '42')
        self.assertEqual(creatures['Creature-0-1-2521-42-123-ABCD']['first_seen'], '2')
        self.assertTrue(creatures['Creature-0-1-2521-42-456-DCBA']['summoned'])
        self.assertFalse(creatures['Creature-0-1-2521-42-123-ABCD']['summoned'])
        summary = module.summarize_population(runs)[0]
        self.assertEqual(summary['runs'], 1)
        self.assertEqual(summary['npcs'][0]['observed_per_run'][0]['count'], 1)
        self.assertEqual(summary['npcs'][1]['summon_destination_count'], 1)
        self.assertEqual(summary['npcs'][1]['observed_outside_encounter_count'], 1)


if __name__ == '__main__':
    unittest.main()
