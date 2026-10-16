"""Omor's actual Fiendish Hound base0; never fabricate a Normal difficulty row."""
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
        'SELECT * FROM content_creature_template WHERE entry!=17540 ORDER BY entry',
        'SELECT * FROM content_creature_template_difficulty ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
    ]
    return [hashlib.sha256(repr(db.execute(q).fetchall()).encode()).hexdigest() for q in queries]


class OmorTemplateTests(unittest.TestCase):
    def test_hound_source_base_resources_idempotence_and_identity_refusal(self):
        root = ROOT / 'target/encounter-producer-fixtures'
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root) as directory:
            path = Path(directory) / 'omor.sqlite'
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE content_creature_template SET ManaModifier=1 WHERE entry=17540')
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'), '--encounter', 'omor', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    self.assertEqual(db.execute('SELECT minlevel,maxlevel,HealthModifier,ManaModifier,DamageModifier,ContentTuningID,AIName,ScriptName FROM content_creature_template WHERE entry=17540').fetchone(), (10, 30, 0.55, 2.8568, 1.0, 220, 'SmartAI', ''))
                    self.assertEqual(db.execute('SELECT DifficultyID FROM content_creature_template_difficulty WHERE entry=17540 AND DifficultyID=1').fetchall(), [])
                    self.assertEqual(preserved(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=17540")
                db.execute('UPDATE content_creature_template SET ManaModifier=1 WHERE entry=17540')
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Fiendish Hound17540 source identity', result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(db.execute('SELECT ManaModifier FROM content_creature_template WHERE entry=17540').fetchone(), (1.0,))
                self.assertEqual(preserved(db), before)


if __name__ == '__main__':
    unittest.main()
