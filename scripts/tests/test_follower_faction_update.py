"""Guarded Brightscale faction-only correction, including exact idempotent replay."""
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
    row = source.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone()
    target.execute(f"INSERT INTO content_creature_template VALUES({','.join('?' for _ in row)})", row)
    target.execute('UPDATE content_creature_template SET faction=0 WHERE entry=232106')
    target.commit()
    return target


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ, 'requires explicit native runtime template fixture')
class GuardedFactionUpdate(unittest.TestCase):
    def setUp(self):
        self.source = sqlite3.connect(Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve().as_uri() + '?mode=ro', uri=True)
        self.addCleanup(self.source.close)

    def test_only_faction_changes_and_exact_replay_is_idempotent(self):
        with closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
            original = before.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone()
            prepared.execute('UPDATE content_creature_template SET faction=16 WHERE entry=232106')
            expected = prepared.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone()
            sql, changed = make_sql(before, prepared, {232106})
            self.assertEqual(changed['content_creature_template'], [(232106,)])
            self.assertNotEqual(original, expected)
            for _ in range(3):
                before.executescript(sql)
                self.assertEqual(before.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone(), expected)

    def test_intervening_faction_or_other_column_changes_abort_without_replacement(self):
        for change in ("faction=14", "HealthModifier=123", "ScriptName='other_owner'"):
            with self.subTest(change=change), closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
                prepared.execute('UPDATE content_creature_template SET faction=16 WHERE entry=232106')
                sql, _ = make_sql(before, prepared, {232106})
                before.execute(f'UPDATE content_creature_template SET {change} WHERE entry=232106')
                before.commit()
                intervening = before.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone()
                with self.assertRaises(sqlite3.IntegrityError):
                    before.executescript(sql)
                before.rollback()
                self.assertEqual(before.execute('SELECT * FROM content_creature_template WHERE entry=232106').fetchone(), intervening)

    def test_unapproved_entry_or_nonfaction_diff_is_refused(self):
        with closing(fixture(self.source)) as before, closing(fixture(self.source)) as prepared:
            prepared.execute('UPDATE content_creature_template SET faction=16 WHERE entry=232106')
            with self.assertRaisesRegex(ValueError, 'must not replace'):
                make_sql(before, prepared)
            prepared.execute('UPDATE content_creature_template SET HealthModifier=123 WHERE entry=232106')
            with self.assertRaisesRegex(ValueError, 'only native-zero faction16'):
                make_sql(before, prepared, {232106})


if __name__ == '__main__':
    unittest.main()
