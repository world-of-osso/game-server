"""Celebras redeemed base projection across the actual producer process boundary."""
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
        'SELECT * FROM content_creature_template WHERE entry!=13716 ORDER BY entry',
        'SELECT * FROM content_creature_template_difficulty ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
    ]
    return [hashlib.sha256(repr(db.execute(q).fetchall()).encode()).hexdigest() for q in queries]


class CelebrasTemplateTests(unittest.TestCase):
    def test_redeemed_replaces_legacy_from_base_only_and_refuses_identity_drift(self):
        root = ROOT / 'target/encounter-producer-fixtures'
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root) as directory:
            path = Path(directory) / 'celebras.sqlite'
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE content_creature_template SET HealthModifier=8,ManaModifier=4 WHERE entry=13716')
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'), '--encounter', 'celebras', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    self.assertEqual(db.execute('SELECT HealthModifier,ManaModifier,DamageModifier,ContentTuningID,AIName,ScriptName FROM content_creature_template WHERE entry=13716').fetchone(), (35.0, 5.3615, 7.5, 0, 'SmartAI', ''))
                    self.assertEqual(db.execute('SELECT DifficultyID FROM content_creature_template_difficulty WHERE entry=13716').fetchall(), [])
                    self.assertEqual(preserved(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=13716")
                db.execute('UPDATE content_creature_template SET HealthModifier=8 WHERE entry=13716')
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Celebras redeemed13716 source identity', result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(db.execute('SELECT HealthModifier FROM content_creature_template WHERE entry=13716').fetchone(), (8.0,))
                self.assertEqual(preserved(db), before)


if __name__ == '__main__':
    unittest.main()
