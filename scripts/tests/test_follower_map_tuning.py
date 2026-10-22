"""TC map difficulty scaling preserves native CT0 rows and their modifiers."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import stage_source_difficulties
from scripts.tests.test_followup_placements import copy_native_actor_fixture

POLICY = 'FOLLOWER_PLACEMENT_CONTENT_TUNING_FROM_NATIVE_MAP_DIFFICULTY'
CASES = ((207207, 2648, 5330, 2973), (207205, 2648, 5330, 2973),
         (211087, 2662, 5283, 2718), (211089, 2662, 5283, 2718),
         (213937, 2662, 5283, 2718), (226396, 2773, 5518, 2718),
         (234935, 2830, 5703, 2718))


@unittest.skipUnless('FOLLOWER_MAP_TUNING_DB' in os.environ, 'requires native source database')
class NativeMapTuning(unittest.TestCase):
    def open_source(self):
        path = Path(os.environ['FOLLOWER_MAP_TUNING_DB']).resolve(strict=True)
        return sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)

    def copy_actor(self, source, entry, map_id):
        target = copy_native_actor_fixture(source, entry)
        target.execute(source.execute("SELECT sql FROM sqlite_master WHERE name='map_difficulty'").fetchone()[0])
        rows = source.execute('SELECT * FROM map_difficulty WHERE MapID=?', (map_id,)).fetchall()
        target.executemany(f"INSERT INTO map_difficulty VALUES({','.join('?' for _ in rows[0])})", rows)
        target.execute('INSERT OR IGNORE INTO content_tuning_level VALUES(?,?,?,?,?)',
                       source.execute('SELECT * FROM content_tuning_level WHERE ID=2973').fetchone())
        return target

    def test_seven_native_zero_actors_select_map_tuning_without_rewriting_rows(self):
        with closing(self.open_source()) as source:
            for entry, map_id, row_id, tuning in CASES:
                with self.subTest(entry=entry), closing(self.copy_actor(source, entry, map_id)) as target:
                    native = target.execute('SELECT * FROM tdb_creature_template_difficulty ORDER BY Entry,DifficultyID').fetchall()
                    record = dict(entry=entry, map_id=map_id, content_tuning_policy=POLICY)
                    try:
                        proof = stage_source_difficulties(target, [record])[0]
                    except ValueError as error:
                        self.fail(f'Native map tuning is sourced, not missing: {error}')
                    self.assertEqual(proof['source_difficulty_id'], 0)
                    self.assertEqual(proof['source_row']['ContentTuningID'], 0)
                    self.assertEqual(proof['effective_content_tuning']['policy'], POLICY)
                    self.assertEqual(proof['effective_content_tuning']['map_difficulty_row']['ID'], row_id)
                    self.assertEqual(proof['effective_content_tuning']['content_tuning_id'], tuning)
                    self.assertEqual(target.execute('SELECT ContentTuningID FROM temp.tdb_difficulty').fetchone(), (0,))
                    self.assertEqual(target.execute('SELECT * FROM tdb_creature_template_difficulty ORDER BY Entry,DifficultyID').fetchall(), native)

    def test_missing_map_or_critter_never_acquires_map_tuning(self):
        with closing(self.open_source()) as source:
            for mutation in ('DELETE FROM map_difficulty',
                             'UPDATE map_difficulty SET ContentTuningID=0',
                             'UPDATE tdb_creature_template SET type=8'):
                with self.subTest(mutation=mutation), closing(self.copy_actor(source, 234935, 2830)) as target:
                    target.execute(mutation)
                    with self.assertRaisesRegex(ValueError, 'FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_234935'):
                        stage_source_difficulties(target, [dict(entry=234935, map_id=2830, content_tuning_policy=POLICY)])


if __name__ == '__main__':
    unittest.main()
