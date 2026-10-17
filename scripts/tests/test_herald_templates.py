"""Herald's sourced summon/helper actors; unresolved Nazan CT0 stays untouched."""
from contextlib import closing
from pathlib import Path
import hashlib
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
ACTORS = '17537,21252'


def preserve_unrelated_data(connection):
    queries = [
        f'SELECT * FROM content_creature_template WHERE entry NOT IN({ACTORS}) ORDER BY entry',
        f'SELECT * FROM content_creature_template_model WHERE CreatureID NOT IN({ACTORS}) ORDER BY CreatureID,Idx',
        f'SELECT * FROM content_creature_template_addon WHERE entry NOT IN({ACTORS}) ORDER BY entry',
        'SELECT * FROM content_creature_template_difficulty '
        'WHERE NOT((entry=17537 AND DifficultyID=2) OR (entry=21252 AND DifficultyID IN(1,2))) '
        'ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY map_id,creature_entry',
        'SELECT * FROM tdb_spawn_group_template WHERE groupId=357',
    ]
    return [hashlib.sha256(repr(connection.execute(query).fetchall()).encode()).hexdigest()
            for query in queries]


class HeraldTemplateTests(unittest.TestCase):
    def setUp(self):
        fixture_root = ROOT / 'target/encounter-producer-fixtures'
        fixture_root.mkdir(parents=True, exist_ok=True)
        directory = tempfile.TemporaryDirectory(dir=fixture_root)
        self.addCleanup(directory.cleanup)
        self.db = Path(directory.name) / 'herald.sqlite'
        source_uri = (ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro'
        with closing(sqlite3.connect(source_uri, uri=True)) as source:
            source.execute('VACUUM INTO ?', (str(self.db),))

    def run_producer(self):
        return subprocess.run(
            [sys.executable, str(ROOT / 'scripts/encounter_summons.py'),
             '--encounter', 'herald', '--db', str(self.db)],
            capture_output=True, text=True, check=False,
        )

    def assert_source_actors(self, connection):
        self.assertEqual(connection.execute(
            'SELECT entry,minlevel,maxlevel,ContentTuningID,LevelScalingDeltaMin,LevelScalingDeltaMax,'
            'HealthModifier,ManaModifier,DamageModifier,'
            'lootid,mingold,maxgold,AIName,ScriptName FROM content_creature_template '
            f'WHERE entry IN({ACTORS}) ORDER BY entry').fetchall(),
            [(17537,10,30,220,2,2,5.4,1.0,7.5,0,2865,3746,'','boss_vazruden'),
             (21252,1,90,328,0,0,1.35,1.0,1.0,0,0,0,'','')])
        self.assertEqual(connection.execute(
            f'SELECT entry,DifficultyID,ContentTuningID,HealthModifier,DamageModifier,lootid,mingold,maxgold '
            'FROM content_creature_template_difficulty '
            'WHERE (entry=17537 AND DifficultyID=2) OR (entry=21252 AND DifficultyID IN(1,2)) '
            'ORDER BY entry,DifficultyID'
        ).fetchall(), [(17537,2,1065,8.5,13.0,0,5730,7492),
                       (21252,1,328,1.35,1.0,0,0,0),(21252,2,328,1.35,1.0,0,0,0)])
        self.assertEqual(connection.execute(
            'SELECT * FROM content_creature_template_difficulty WHERE entry=17537 AND DifficultyID=1'
        ).fetchall(), [], 'Vazruden has no authored Normal1 row')
        self.assertEqual(connection.execute(
            f'SELECT * FROM content_creature_template_model WHERE CreatureID IN({ACTORS}) ORDER BY CreatureID,Idx'
        ).fetchall(), connection.execute(
            f'SELECT * FROM tdb_creature_template_model WHERE CreatureID IN({ACTORS}) ORDER BY CreatureID,Idx'
        ).fetchall())
        self.assertEqual(connection.execute(
            f'SELECT * FROM content_creature_template_addon WHERE entry IN({ACTORS}) ORDER BY entry'
        ).fetchall(), connection.execute(
            'SELECT entry,PathId,mount,StandState | (VisFlags << 16) | (AnimTier << 24),'
            'SheathState | (PvPFlags << 8),emote,visibilityDistanceType,auras '
            f'FROM tdb_creature_template_addon WHERE entry IN({ACTORS}) ORDER BY entry'
        ).fetchall())

    def test_real_cli_replaces_legacy_summon_data_with_exact_base_and_authored_difficulties(self):
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('UPDATE content_creature_template SET HealthModifier=0.01,lootid=17537 '
                               f'WHERE entry IN({ACTORS})')
            before = preserve_unrelated_data(connection)
        for _ in range(2):
            result = self.run_producer()
            self.assertEqual(result.returncode, 0, result.stderr)
            with closing(sqlite3.connect(self.db)) as connection:
                self.assert_source_actors(connection)
                self.assertEqual(preserve_unrelated_data(connection), before,
                                 'Nazan, other actors, credit, population and ordinary group357 stay unchanged')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=17537")
            connection.execute('UPDATE content_creature_template SET HealthModifier=0.01 WHERE entry=17537')
        before_refusal = hashlib.sha256(self.db.read_bytes()).hexdigest()
        result = self.run_producer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(hashlib.sha256(self.db.read_bytes()).hexdigest(), before_refusal)
        self.assertIn('Herald summon17537 source identity', result.stderr)
        with closing(sqlite3.connect(self.db)) as connection:
            self.assertEqual(connection.execute(
                'SELECT HealthModifier FROM content_creature_template WHERE entry=17537').fetchone(), (0.01,))
            self.assertEqual(preserve_unrelated_data(connection), before)


if __name__ == '__main__':
    unittest.main()
