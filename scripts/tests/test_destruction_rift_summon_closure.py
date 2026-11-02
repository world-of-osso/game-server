"""Dimensional Rift guardians are part of the class-summon closure."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import retail_content


class RiftSummonClosureTests(unittest.TestCase):
    def test_native_class_closure_reaches_each_tear(self):
        source = Path(os.environ['DPS_PARITY_WORLD_DB']).resolve()
        with closing(sqlite3.connect(source.as_uri() + '?mode=ro', uri=True)) as db:
            expected = {196280: 'Unstable Tear', 198547: 'Shadowy Tear', 198555: 'Chaos Tear'}
            rows = dict(db.execute('SELECT entry,name FROM tdb_creature_template WHERE entry IN (196280,198547,198555)'))
            self.assertEqual(rows, expected)
            reached = {row[0] for row in db.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertTrue(set(expected).issubset(reached))


if __name__ == '__main__':
    unittest.main()
