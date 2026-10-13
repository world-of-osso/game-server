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
        self.assertTrue(any('source intro supported' in reason and 'spawn-group' in reason for reason in gaps))
        self.assertFalse(any('intro remains closed' in reason for reason in gaps))
        self.assertFalse(any('missing emerge271598' in reason for reason in gaps))
        self.assertTrue(any('delayed follow/absorption/announcement supported' in reason for reason in gaps))
        self.assertTrue(any('near-point' in reason for reason in gaps))
        self.assertFalse(any('infusion' in reason or 'Fel Spray' in reason for reason in gaps))
        self.assertEqual(actor['proof'], 'structural inspection only')

    def test_ambiguous_display_does_not_assign_kill_credit(self):
        actors = [['Council', '', 30, 9, 123, 0, 0, 0, 9]]
        candidates = {(43, 123): [3669, 3670]}
        entries, gaps = coverage.resolve_actors(43, [9], actors, candidates)
        self.assertEqual(entries, [])
        self.assertEqual(gaps, ['journal actor 30: display 123 has 2 same-map candidates'])


if __name__ == '__main__':
    unittest.main()
