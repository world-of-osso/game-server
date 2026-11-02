"""C'Thun guardian entries come from the native class-summon closure."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import retail_content


class CthunSummonClosureTests(unittest.TestCase):
    def test_native_class_closure_reaches_both_guardians(self):
        source = Path(os.environ['DPS_PARITY_WORLD_DB']).resolve()
        with closing(sqlite3.connect(source.as_uri() + '?mode=ro', uri=True)) as db:
            children = dict(db.execute('''SELECT SpellID, EffectMiscValue_0 FROM spell_effect
                WHERE DifficultyID=0 AND Effect=28 AND SpellID IN (377355,377357)'''))
            self.assertEqual(children, {377355: 192337, 377357: 198757})
            reached = {row[0] for row in db.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertTrue(set(children.values()).issubset(reached))


if __name__ == '__main__':
    unittest.main()
