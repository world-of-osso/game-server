"""Observable census contracts, independent of production database contents."""
import sqlite3
import unittest
from contextlib import closing
from pathlib import Path

import encounter_coverage as coverage


class CoverageTests(unittest.TestCase):
    def test_union_keeps_unbound_journal_and_difficulty_records(self):
        encounters = [dict(ID=586, MapID=43, Bit=1, Name_lang='Lord Cobrahn', DifficultyID=1),
                      dict(ID=900, MapID=43, Bit=1, Name_lang='Lord Cobrahn', DifficultyID=2)]
        journals = [['Lord Cobrahn', '', 0, 0, 7, 8, 586],
                    ['Unbound council', '', 0, 0, 9, 8, 0]]
        records = coverage.catalog_records(encounters, journals, {8: 43}, {43})
        self.assertEqual([(r['encounter_id'], r['journal_ids']) for r in records],
                         [(0, [9]), (586, [7]), (900, [])])

    def test_unsupported_cast_never_becomes_implemented_ability(self):
        row = dict(id=2, event_type=0, action_type=11, target_type=2, event_flags=0,
                   action_param1=20805, action_param2=0, event_param2=0)
        self.assertEqual(coverage.row_gaps(row), [])
        row['event_type'] = 2  # HEALTH_PCT is not currently interpreted.
        self.assertEqual(coverage.row_gaps(row), ['event 2'])
        row['event_type'] = 0
        row['target_type'] = 4
        self.assertEqual(coverage.row_gaps(row), ['target 4'])

    def test_authored_respawn_cast_has_no_row_vocabulary_gap(self):
        row = dict(id=0, event_type=11, action_type=11, target_type=1, event_flags=0,
                   action_param1=1264106, action_param2=0, event_param2=0)
        self.assertEqual(coverage.row_gaps(row), [])

    def test_golden_partial_census_keeps_its_own_source_gaps(self):
        source = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
        with closing(sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)) as connection:
            order = {row[0]: dict(tier=3, sort=(3,row[0]), reason='test source inventory')
                     for row in connection.execute('SELECT ID FROM map')}
            records = coverage.collect(connection, [], [], {}, [], order)
        golden = next(row for row in records if row['map_id']==1762 and row['encounter_id']==2139)
        actor = next(actor for actor in golden['actors'] if actor['entry']==135322)
        self.assertFalse(actor['cpp_gap'])
        self.assertTrue(actor['runtime_ai'].startswith('partial TC C++ port'))
        gaps = [reason for gap in actor['ai_gaps'] for reason in gap['reasons']]
        self.assertTrue(any('Retail 12.1 timeline inputs unavailable' in reason for reason in gaps))
        self.assertTrue(any('group1256 spawns on last TempleEvent death' in reason and 'Serpentine Seal gossip summons Zul' in reason for reason in gaps))
        self.assertFalse(any('chain unported' in reason for reason in gaps))
        self.assertFalse(any('intro remains closed' in reason for reason in gaps))
        self.assertFalse(any('missing emerge271598' in reason for reason in gaps))
        self.assertTrue(any('delayed follow/absorption/announcement supported' in reason for reason in gaps))
        self.assertTrue(any('near-point' in reason for reason in gaps))
        self.assertFalse(any('infusion' in reason or 'Fel Spray' in reason for reason in gaps))
        self.assertEqual(actor['proof'], 'structural inspection only')

    def test_landed_maker_port_is_partial_with_its_named_gaps(self):
        source = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
        with closing(sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)) as connection:
            order = {row[0]: dict(tier=3, sort=(3,row[0]), reason='test source inventory')
                     for row in connection.execute('SELECT ID FROM map')}
            records = coverage.collect(connection, [], [], {}, [], order)
        maker = next(row for row in records if row['map_id']==542 and row['encounter_id']==1922)
        actor = next(actor for actor in maker['actors'] if actor['entry']==17381)
        self.assertFalse(actor['cpp_gap'])
        self.assertEqual(actor['runtime_ai'], 'partial TC C++ port')
        self.assertEqual(actor['admitted_casts'], [40059])
        gaps = [reason for gap in actor['ai_gaps'] for reason in gap['reasons']]
        self.assertTrue(any('MAKER_NORMAL_BEAKER_30925_ABSENT_70448' in reason for reason in gaps))
        self.assertFalse(any('Herald' in reason or 'Toxic Volley' in reason for reason in gaps))

    def test_deadmines_source_primary_is_not_its_journal_companion(self):
        source = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
        journals = [['Helix Gearbreaker', '', 0, 0, 90, 1, 2977]]
        actors = [['Lumbering Oaf', '', 101, 90, 33003, 0, 0, 0, 0]]
        with closing(sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)) as connection:
            order = {row[0]: dict(tier=3, sort=(3,row[0]), reason='test source inventory')
                     for row in connection.execute('SELECT ID FROM map')}
            records = coverage.collect(connection, journals, actors, {1: 36}, [], order)
        expected = {2976: 47162, 2981: 47162, 2977: 47296, 2982: 47296,
                    2975: 43778, 2980: 43778, 2974: 47626, 2979: 47626,
                    2973: 47739, 2978: 47739, 1081: 49541}
        deadmines = {r['encounter_id']: r for r in records if r['map_id'] == 36}
        for encounter, primary in expected.items():
            self.assertIn(primary, {a['entry'] for a in deadmines[encounter]['actors']})
        helix = deadmines[2977]
        by_entry = {a['entry']: a for a in helix['actors']}
        self.assertEqual(set(by_entry), {47296, 47297})
        self.assertEqual(by_entry[47296]['name'], 'Helix Gearbreaker')
        self.assertEqual(by_entry[47297]['name'], 'Lumbering Oaf')
        for actor in by_entry.values():
            self.assertEqual(actor['kill_credit'], 'blocked: no binding/spawn')
            self.assertEqual(actor['admitted_casts'], [])
        rendered = coverage.render([helix], source, 'fixture')
        self.assertIn('Lumbering Oaf [47297]', rendered)
        self.assertNotIn('Helix Gearbreaker [47297]', rendered)
        self.assertIn('pinned TC has no Deadmines boss AI', rendered)

    def test_slagmaw_cpp_identity_does_not_require_journal_preview_model_equality(self):
        source = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
        journals = [['Slagmaw', '', 0, 0, 696, 1, 1445]]
        actors = [['Slagmaw', '', 1344, 696, 38391, 0, 0, 0, 0]]
        with closing(sqlite3.connect(source.as_uri()+'?mode=ro', uri=True)) as connection:
            # A TEMP view makes the missing-binding boundary stable even after lead publication.
            connection.execute('CREATE TEMP VIEW raid_encounter_creature AS '
                               'SELECT * FROM main.raid_encounter_creature WHERE creature_entry!=61463')
            order = {row[0]: dict(tier=3, sort=(3,row[0]), reason='test source inventory')
                     for row in connection.execute('SELECT ID FROM map')}
            records = coverage.collect(connection, journals, actors, {1: 389}, [], order)
        slagmaw = next(row for row in records if row['map_id']==389 and row['encounter_id']==1445)
        by_entry = {actor['entry']: actor for actor in slagmaw['actors']}
        self.assertIn(61463, by_entry)
        actor = by_entry[61463]
        self.assertEqual(actor['source_cpp'], 'boss_slagmaw')
        self.assertFalse(actor['cpp_gap'])
        self.assertEqual(actor['runtime_ai'], 'partial TC C++ port blocked by missing binding')
        self.assertEqual(actor['kill_credit'], 'blocked: no binding/spawn')
        self.assertEqual(actor['proof'], 'structural inspection only')
        self.assertIn('journal actor 1344: display 38391 has 0 same-map candidates', slagmaw['binding_gaps'])

    def test_wailing_instance_state_action_is_not_universal_support(self):
        row = dict(id=0, event_type=6, action_type=34, target_type=1,
                   event_flags=0, action_param1=3, action_param2=3, action_param3=1)
        self.assertEqual(coverage.row_gaps(row, map_id=43), [])
        self.assertEqual(coverage.row_gaps(row, map_id=36), ['action 34'])
        row['action_param3'] = 0
        self.assertEqual(coverage.row_gaps(row, map_id=43), ['action 34'])
        row['action_param3'] = 1
        row['action_param1'] = 8
        self.assertEqual(coverage.row_gaps(row, map_id=43), ['action 34'])

    def test_ambiguous_display_does_not_assign_kill_credit(self):
        actors = [['Council', '', 30, 9, 123, 0, 0, 0, 9]]
        candidates = {(43, 123): [3669, 3670]}
        entries, gaps = coverage.resolve_actors(43, [9], actors, candidates)
        self.assertEqual(entries, [])
        self.assertEqual(gaps, ['journal actor 30: display 123 has 2 same-map candidates'])


if __name__ == '__main__':
    unittest.main()
