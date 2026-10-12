"""Observable census contracts, independent of production database contents."""
import sqlite3
import unittest

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

    def test_ambiguous_display_does_not_assign_kill_credit(self):
        actors = [['Council', '', 30, 9, 123, 0, 0, 0, 9]]
        candidates = {(43, 123): [3669, 3670]}
        entries, gaps = coverage.resolve_actors(43, [9], actors, candidates)
        self.assertEqual(entries, [])
        self.assertEqual(gaps, ['journal actor 30: display 123 has 2 same-map candidates'])


if __name__ == '__main__':
    unittest.main()
