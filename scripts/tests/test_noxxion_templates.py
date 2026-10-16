"""Real process-boundary projection of the authored Noxxion Normal summon row."""
import hashlib
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'


def preserved_rows(db):
    queries = {
        'templates': 'SELECT * FROM content_creature_template ORDER BY entry',
        'models': 'SELECT * FROM content_creature_template_model ORDER BY CreatureID,Idx',
        'population': 'SELECT * FROM content_creature ORDER BY guid',
        'credit': 'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
        'other_difficulties': 'SELECT * FROM content_creature_template_difficulty WHERE entry!=13456 ORDER BY entry,DifficultyID',
    }
    return {name: hashlib.sha256(repr(db.execute(query).fetchall()).encode()).hexdigest()
            for name, query in queries.items()}


class NoxxionTemplateTests(unittest.TestCase):
    def test_actual_normal_projection_is_idempotent_and_refuses_changed_spell(self):
        root = ROOT / 'target/encounter-producer-fixtures'
        root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=root) as directory:
            path = Path(directory) / 'noxxion.sqlite'
            with closing(sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry=13456')
                before = preserved_rows(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'),
                    '--encounter', 'noxxion', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with closing(sqlite3.connect(path)) as db, db:
                    self.assertEqual(db.execute('''SELECT entry,DifficultyID,minlevel,maxlevel,
                        ContentTuningID,HealthModifier,ManaModifier,DamageModifier,lootid
                        FROM content_creature_template_difficulty WHERE entry=13456''').fetchall(),
                        [(13456, 1, 10, 30, 206, 0.4, 1.0, 1.0, 13456)])
                    self.assertEqual(preserved_rows(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry=13456')
                db.execute('UPDATE spell_effect SET EffectMiscValue_0=13596 WHERE SpellID=21707 AND EffectIndex=0')
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Noxxion21707 summon identity', result.stderr)
            with closing(sqlite3.connect(path)) as db, db:
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template_difficulty WHERE entry=13456').fetchone()[0], 0)
                self.assertEqual(preserved_rows(db), before)


if __name__ == '__main__':
    unittest.main()
