"""Process-boundary Normal wave projection on real independent source copies."""
import hashlib
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'
SCRIPT = ROOT / 'scripts/encounter_summons.py'


def snapshot_source(destination):
    with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as source:
        with sqlite3.connect(destination) as copy:
            source.backup(copy)


def run_projection(path):
    return subprocess.run(
        [sys.executable, str(SCRIPT), '--encounter', 'wailing-ritual', '--db', str(path)],
        capture_output=True, text=True, check=False,
    )


def hash_rows(db, query):
    digest = hashlib.sha256()
    for row in db.execute(query):
        digest.update(repr(row).encode())
        digest.update(b'\n')
    return digest.hexdigest()


def untouched_rows(db):
    return {
        'population': hash_rows(db, 'SELECT * FROM content_creature ORDER BY guid'),
        'base': hash_rows(db, 'SELECT * FROM content_creature_template ORDER BY entry'),
        'models': hash_rows(db, 'SELECT * FROM content_creature_template_model ORDER BY CreatureID,Idx'),
        'credit': hash_rows(db, 'SELECT * FROM raid_encounter_creature ORDER BY creature_entry'),
        'other_difficulties': hash_rows(db, 'SELECT * FROM content_creature_template_difficulty WHERE entry NOT IN (5762,5763) ORDER BY entry,DifficultyID'),
    }


class WailingRitualTemplatesTests(unittest.TestCase):
    def test_projects_only_authored_normal_rows_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'ritual.sqlite'
            snapshot_source(path)
            with sqlite3.connect(path) as db:
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry IN (5762,5763)')
                db.commit()
                before = untouched_rows(db)
            result = run_projection(path)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            with sqlite3.connect(path) as db:
                actual = db.execute('''SELECT entry,DifficultyID,minlevel,maxlevel,ContentTuningID,
                    LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,
                    ArmorModifier,DamageModifier,lootid,mingold,maxgold
                    FROM content_creature_template_difficulty WHERE entry IN (5762,5763)
                    ORDER BY entry,DifficultyID''').fetchall()
                self.assertEqual(actual, [
                    (5762, 1, 8, 30, 193, 1, 1, 5.0, 1.0, 1.0, 7.5, 5762, 0, 0),
                    (5763, 1, 8, 30, 193, 2, 2, 1.2, 1.0, 1.0, 1.0, 5763, 0, 0),
                ])
                self.assertEqual(untouched_rows(db), before)
                projected = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN (5762,5763) ORDER BY entry,DifficultyID').fetchall()
            result = run_projection(path)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            with sqlite3.connect(path) as db:
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN (5762,5763) ORDER BY entry,DifficultyID').fetchall(), projected)
                self.assertEqual(untouched_rows(db), before)

    def test_refuses_changed_group_identity_without_partial_normal_projection(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'wrong-wave.sqlite'
            snapshot_source(path)
            with sqlite3.connect(path) as db:
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry IN (5762,5763)')
                db.execute('UPDATE tdb_creature_summon_groups SET entry=5755 WHERE summonerId=3678 AND summonerType=0 AND groupId=3')
                db.commit()
                before = untouched_rows(db)
            result = run_projection(path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('ritual wave identity', result.stderr)
            with sqlite3.connect(path) as db:
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template_difficulty WHERE entry IN (5762,5763)').fetchone()[0], 0)
                self.assertEqual(untouched_rows(db), before)


if __name__ == '__main__':
    unittest.main()
