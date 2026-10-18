"""Telash rune source projection at the independent-copy producer boundary."""
from contextlib import closing
import hashlib
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
RUNE = 197204


def preserved(db):
    queries = [
        'SELECT * FROM content_creature_template WHERE entry!=197204 ORDER BY entry',
        'SELECT * FROM content_creature_template_model WHERE CreatureID!=197204 ORDER BY CreatureID,Idx',
        'SELECT * FROM content_creature_template_difficulty WHERE entry!=197204 ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature_template_addon ORDER BY entry',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
        'SELECT * FROM tdb_creature_summon_groups ORDER BY summonerId,summonerType,groupId,entry,position_x,position_y',
    ]
    return [hashlib.sha256(repr(db.execute(query).fetchall()).encode()).hexdigest()
            for query in queries]


class TelashTemplateTests(unittest.TestCase):
    def test_source_rune_projection_is_idempotent_and_refuses_identity_drift(self):
        fixtures = ROOT / 'target/encounter-producer-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            path = Path(directory) / 'telash.sqlite'
            source_uri = (ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro'
            with closing(sqlite3.connect(source_uri, uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db, db:
                self.assertEqual(db.execute('''SELECT groupId,entry,count(*)
                    FROM tdb_creature_summon_groups WHERE summonerId=186737
                    AND summonerType=0 GROUP BY groupId,entry''').fetchall(), [(0, RUNE, 3)])
                self.assertEqual(db.execute('''SELECT AIName,ScriptName FROM
                    tdb_creature_template WHERE entry=?''', (RUNE,)).fetchone(), ('', ''))
                for table, key in [('content_creature_template', 'entry'),
                                   ('content_creature_template_model', 'CreatureID'),
                                   ('content_creature_template_difficulty', 'entry')]:
                    db.execute(f'DELETE FROM {table} WHERE {key}=?', (RUNE,))
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_summons.py'),
                    '--encounter', 'telash', '--db', str(path)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    self.assertEqual(db.execute('''SELECT name,AIName,ScriptName,unit_class,
                        faction,scale,HealthModifier FROM content_creature_template
                        WHERE entry=?''', (RUNE,)).fetchone(), ('Vault Rune', '', '', 4, 35, 1.0, 1.0))
                    models = db.execute('''SELECT CreatureDisplayID,DisplayScale,Probability
                        FROM content_creature_template_model WHERE CreatureID=?
                        ORDER BY Idx''', (RUNE,)).fetchall()
                    source_models = db.execute('''SELECT CreatureDisplayID,DisplayScale,Probability
                        FROM tdb_creature_template_model WHERE CreatureID=?
                        ORDER BY Idx''', (RUNE,)).fetchall()
                    self.assertEqual(models, source_models)
                    self.assertEqual(models, [(22712, 1.0, 0.0), (100728, 0.2, 1.0)])
                    self.assertEqual(db.execute('''SELECT DifficultyID,ContentTuningID,HealthModifier
                        FROM content_creature_template_difficulty WHERE entry=?
                        ORDER BY DifficultyID''', (RUNE,)).fetchall(), [(1, 2156, 1.0), (23, 2157, 1.0)])
                    self.assertEqual(preserved(db), before)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE tdb_creature_template SET AIName='different' WHERE entry=?", (RUNE,))
                content_before = db.execute('SELECT * FROM content_creature_template WHERE entry=?',
                                            (RUNE,)).fetchall()
            result = subprocess.run(argv, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('197204 source AI identity changed', result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=?',
                                            (RUNE,)).fetchall(), content_before)
                self.assertEqual(preserved(db), before)


if __name__ == '__main__':
    unittest.main()
