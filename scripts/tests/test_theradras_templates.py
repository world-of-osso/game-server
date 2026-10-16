"""Zaetar's Spirit: real Normal row through the copy-only producer process."""
from contextlib import closing
from pathlib import Path
import hashlib
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def preserved(db):
    queries = [
        'SELECT * FROM content_creature_template WHERE entry!=12238 ORDER BY entry',
        'SELECT * FROM content_creature_template_difficulty WHERE entry!=12238 ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
    ]
    return [hashlib.sha256(repr(db.execute(q).fetchall()).encode()).hexdigest() for q in queries]


class TheradrasTemplateTests(unittest.TestCase):
    def test_spirit_normal_source_stats_idempotence_and_identity_refusal(self):
        root = ROOT / 'target/encounter-producer-fixtures'
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root) as directory:
            path = Path(directory) / 'theradras.sqlite'
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE content_creature_template SET HealthModifier=1.35,minlevel=60,maxlevel=60 WHERE entry=12238')
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry=12238')
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'), '--encounter', 'theradras', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    self.assertEqual(db.execute('SELECT HealthModifier,ManaModifier,DamageModifier,ContentTuningID,AIName,ScriptName FROM content_creature_template WHERE entry=12238').fetchone(), (2.0, 1.0, 1.0, 0, 'SmartAI', ''))
                    self.assertEqual(db.execute('SELECT DifficultyID,minlevel,maxlevel,ContentTuningID,HealthModifier,ManaModifier,DamageModifier,LevelScalingDeltaMin,LevelScalingDeltaMax FROM content_creature_template_difficulty WHERE entry=12238 ORDER BY DifficultyID').fetchall(), [(1, 10, 30, 281, 2.0, 1.0, 7.5, 0, 0)])
                    self.assertEqual(preserved(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=12238")
                db.execute('UPDATE content_creature_template SET HealthModifier=1.35 WHERE entry=12238')
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Zaetar spirit12238 source identity', result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(db.execute('SELECT HealthModifier FROM content_creature_template WHERE entry=12238').fetchone(), (1.35,))
                self.assertEqual(preserved(db), before)


if __name__ == '__main__':
    unittest.main()
