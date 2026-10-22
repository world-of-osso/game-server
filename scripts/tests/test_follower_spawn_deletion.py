"""Exact owned-spawn deletion guards preserve unrelated published actors."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import make_sql


TABLES = ('content_creature_template', 'content_creature_template_model', 'content_creature', 'raid_encounter_creature')


def fixture(source):
    target = sqlite3.connect(':memory:')
    for table in TABLES:
        target.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
    rows = source.execute('SELECT * FROM content_creature WHERE guid IN(63859000,63859100) ORDER BY guid').fetchall()
    if len(rows) != 2:
        raise ValueError('requires actually published Ikuzz plus the first Blinding trash actor')
    target.executemany(f"INSERT INTO content_creature VALUES({','.join('?' for _ in rows[0])})", rows)
    target.commit()
    return target


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ, 'requires the published immutable baseline rows')
class GuardedSpawnDeletion(unittest.TestCase):
    def setUp(self):
        self.source = sqlite3.connect(Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve().as_uri() + '?mode=ro', uri=True)
        self.addCleanup(self.source.close)

    def test_exact_owned_row_is_removed_once_and_unrelated_boss_survives_three_replays(self):
        with closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
            boss = before.execute('SELECT * FROM content_creature WHERE guid=63859000').fetchone()
            prepared.execute('DELETE FROM content_creature WHERE guid=63859100')
            sql, _ = make_sql(before, prepared, spawn_deletion_guids={63859100})
            for _ in range(3):
                before.executescript(sql)
                self.assertEqual(before.execute('SELECT count(*) FROM content_creature WHERE guid=63859100').fetchone(), (0,))
                self.assertEqual(before.execute('SELECT * FROM content_creature WHERE guid=63859000').fetchone(), boss)

    def test_intervening_pose_map_or_owner_changes_refuse_deletion(self):
        for change in ('position_x=position_x+1', 'map=2811', "Comment='another_owner'"):
            with self.subTest(change=change), closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
                prepared.execute('DELETE FROM content_creature WHERE guid=63859100')
                sql, _ = make_sql(before, prepared, spawn_deletion_guids={63859100})
                before.execute(f'UPDATE content_creature SET {change} WHERE guid=63859100')
                before.commit()
                changed = before.execute('SELECT * FROM content_creature WHERE guid=63859100').fetchone()
                with self.assertRaises(sqlite3.IntegrityError):
                    before.executescript(sql)
                before.rollback()
                self.assertEqual(before.execute('SELECT * FROM content_creature WHERE guid=63859100').fetchone(), changed)

    def test_unapproved_deletion_is_not_silently_omitted(self):
        with closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
            prepared.execute('DELETE FROM content_creature WHERE guid=63859000')
            with self.assertRaisesRegex(ValueError, 'must not remove'):
                make_sql(before, prepared, spawn_deletion_guids={63859100})


if __name__ == '__main__':
    unittest.main()
