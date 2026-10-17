"""Source-only Leymor summon preparation at the producer process boundary."""
from contextlib import closing
import hashlib
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPROUT = 190509


def preserved(db):
    queries = [
        'SELECT * FROM content_creature_template WHERE entry!=190509 ORDER BY entry',
        'SELECT * FROM content_creature_template_model WHERE CreatureID!=190509 ORDER BY CreatureID,Idx',
        'SELECT * FROM content_creature_template_difficulty WHERE entry!=190509 ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
    ]
    return [hashlib.sha256(repr(db.execute(query).fetchall()).encode()).hexdigest()
            for query in queries]


class LeymorTemplateTests(unittest.TestCase):
    def test_source_sprout_projection_is_idempotent_and_refuses_identity_drift(self):
        fixtures = ROOT / 'target/encounter-producer-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            path = Path(directory) / 'leymor.sqlite'
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri()
                                         + '?mode=ro', uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                self.assertEqual(db.execute('''SELECT Effect,EffectMiscValue_0,EffectMiscValue_1
                    FROM spell_effect WHERE SpellID=374360 AND EffectIndex=0
                    AND DifficultyID=0''').fetchone(), (28, SPROUT, 3557))
                for table, key in [('content_creature_template', 'entry'),
                                   ('content_creature_template_model', 'CreatureID'),
                                   ('content_creature_template_difficulty', 'entry')]:
                    db.execute(f'DELETE FROM {table} WHERE {key}=?', (SPROUT,))
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'),
                    '--encounter', 'leymor', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    self.assertEqual(db.execute('''SELECT name,AIName,ScriptName,HealthModifier
                        FROM content_creature_template WHERE entry=190509''').fetchone(),
                                     ('Ley-Line Sprout', '', 'npc_ley_line_sprouts', 1.0))
                    self.assertEqual(db.execute('''SELECT CreatureDisplayID,DisplayScale
                        FROM content_creature_template_model WHERE CreatureID=190509''').fetchall(),
                                     [(92726, 1.0)])
                    self.assertEqual(db.execute('''SELECT DifficultyID,ContentTuningID,HealthModifier
                        FROM content_creature_template_difficulty WHERE entry=190509
                        ORDER BY DifficultyID''').fetchall(), [(1, 2156, 1.0), (23, 2157, 1.0)])
                    self.assertEqual(preserved(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=190509")
                db.execute('UPDATE content_creature_template SET HealthModifier=8 WHERE entry=190509')
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('190509 source AI identity changed', result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(db.execute('SELECT HealthModifier FROM content_creature_template '
                                            'WHERE entry=190509').fetchone(), (8.0,))
                self.assertEqual(preserved(db), before)


if __name__ == '__main__':
    unittest.main()
