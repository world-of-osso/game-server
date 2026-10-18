"""Copy-only source Nazan17536 CT0 data; not a canonical spawn or morph fixture."""
from contextlib import closing
from pathlib import Path
import hashlib
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def preserved_rows(connection):
    queries = [
        'SELECT * FROM content_creature_template WHERE entry!=17536 ORDER BY entry',
        'SELECT * FROM content_creature_template_model WHERE CreatureID!=17536 ORDER BY CreatureID,Idx',
        'SELECT * FROM content_creature_template_addon WHERE entry!=17536 ORDER BY entry',
        'SELECT * FROM content_creature_template_difficulty '
        'WHERE NOT(entry=17536 AND DifficultyID=2) ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY map_id,creature_entry',
        'SELECT * FROM tdb_spawn_group_template WHERE groupId=357',
        'SELECT * FROM zephras_row_source ORDER BY table_name,row_key',
    ]
    return [hashlib.sha256(repr(connection.execute(query).fetchall()).encode()).hexdigest()
            for query in queries]


class NazanTemplateTests(unittest.TestCase):
    def setUp(self):
        fixture_root = ROOT / 'target/encounter-producer-fixtures'
        fixture_root.mkdir(parents=True, exist_ok=True)
        directory = tempfile.TemporaryDirectory(dir=fixture_root)
        self.addCleanup(directory.cleanup)
        self.db = Path(directory.name) / 'nazan.sqlite'
        uri = (ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro'
        with closing(sqlite3.connect(uri, uri=True)) as source:
            source.execute('VACUUM INTO ?', (str(self.db),))

    def run_producer(self):
        return subprocess.run(
            [sys.executable, str(ROOT / 'scripts/encounter_summons.py'),
             '--encounter', 'herald-nazan', '--db', str(self.db)],
            capture_output=True, text=True, check=False,
        )

    def assert_source_nazan(self, connection):
        # CT0/cache bounds1 are retained as input, not declared as the runtime native level.
        self.assertEqual(connection.execute(
            'SELECT entry,name,subname,minlevel,maxlevel,exp,ContentTuningID,HealthScalingExpansion,'
            'LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,ArmorModifier,'
            'DamageModifier,lootid,mingold,maxgold,speed_walk,speed_run,faction,rank,unit_class,'
            'unit_flags,unit_flags2,AIName,ScriptName,movementId,type_flags,RegenHealth,flags_extra '
            'FROM content_creature_template WHERE entry=17536'
        ).fetchone(), (17536,'Nazan',"Vazruden's Mount",1,1,1,0,1,0,0,9.0,1.0,1.0,
                       1.0,0,0,0,4.0,2.57143,40,1,1,0,2048,'','',216,8,1,0))
        self.assertEqual(connection.execute(
            'SELECT entry,DifficultyID,minlevel,maxlevel,exp,ContentTuningID,HealthScalingExpansion,'
            'LevelScalingDeltaMin,LevelScalingDeltaMax,HealthModifier,ManaModifier,ArmorModifier,'
            'DamageModifier,lootid,mingold,maxgold FROM content_creature_template_difficulty '
            'WHERE entry=17536 ORDER BY DifficultyID'
        ).fetchall(), [(17536,2,1,1,1,0,1,0,0,12.0,1.0,1.0,13.0,18432,7770,10156)])
        self.assertEqual(connection.execute(
            'SELECT * FROM content_creature_template_model WHERE CreatureID=17536 ORDER BY Idx'
        ).fetchall(), connection.execute(
            'SELECT * FROM tdb_creature_template_model WHERE CreatureID=17536 ORDER BY Idx'
        ).fetchall())
        self.assertEqual(connection.execute(
            'SELECT * FROM content_creature_template_addon WHERE entry=17536'
        ).fetchall(), connection.execute(
            'SELECT entry,PathId,mount,StandState | (VisFlags << 16) | (AnimTier << 24),'
            'SheathState | (PvPFlags << 8),emote,visibilityDistanceType,auras '
            'FROM tdb_creature_template_addon WHERE entry=17536'
        ).fetchall())

    def assert_atomic_refusal(self, expected_message):
        before = hashlib.sha256(self.db.read_bytes()).hexdigest()
        result = self.run_producer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(expected_message, result.stderr)
        self.assertEqual(hashlib.sha256(self.db.read_bytes()).hexdigest(), before)

    def test_real_cli_projects_exact_ct0_base_heroic_models_addon_and_refuses_source_drift(self):
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('UPDATE content_creature_template SET HealthModifier=0.01 WHERE entry=17536')
            before = preserved_rows(connection)
        for _ in range(2):
            result = self.run_producer()
            self.assertEqual(result.returncode, 0, result.stderr)
            with closing(sqlite3.connect(self.db)) as connection:
                self.assert_source_nazan(connection)
                self.assertEqual(preserved_rows(connection), before,
                                 'other actors/difficulties, population, bindings, group357 and authored provenance stay unchanged')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute("UPDATE tdb_creature_template SET ScriptName='different' WHERE entry=17536")
        self.assert_atomic_refusal('Nazan17536 source identity')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=17536")
            connection.execute('UPDATE tdb_creature_template_difficulty SET ContentTuningID=220 '
                               'WHERE Entry=17536 AND DifficultyID=2')
        self.assert_atomic_refusal('Nazan17536 source difficulties')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('UPDATE tdb_creature_template_difficulty SET ContentTuningID=0 '
                               'WHERE Entry=17536 AND DifficultyID=2')
            model = connection.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=17536').fetchone()
            connection.execute('DELETE FROM tdb_creature_template_model WHERE CreatureID=17536')
        self.assert_atomic_refusal('Nazan17536 source model')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('INSERT INTO tdb_creature_template_model VALUES (' + ','.join('?' for _ in model) + ')', model)
            addon = connection.execute('SELECT * FROM tdb_creature_template_addon WHERE entry=17536').fetchone()
            connection.execute('DELETE FROM tdb_creature_template_addon WHERE entry=17536')
        self.assert_atomic_refusal('Nazan17536 missing source addon')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('INSERT INTO tdb_creature_template_addon VALUES (' + ','.join('?' for _ in addon) + ')', addon)
            heroic = connection.execute('SELECT * FROM tdb_creature_template_difficulty '
                                        'WHERE Entry=17536 AND DifficultyID=2').fetchone()
            connection.execute('DELETE FROM tdb_creature_template_difficulty WHERE Entry=17536 AND DifficultyID=2')
        self.assert_atomic_refusal('Nazan17536 source difficulties')
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute('INSERT INTO tdb_creature_template_difficulty VALUES (' + ','.join('?' for _ in heroic) + ')', heroic)
        with closing(sqlite3.connect(self.db)) as connection:
            self.assert_source_nazan(connection)
            self.assertEqual(preserved_rows(connection), before)


if __name__ == '__main__':
    unittest.main()
