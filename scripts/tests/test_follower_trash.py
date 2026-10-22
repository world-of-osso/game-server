"""Own native trash templates and independently sourced marker placements."""
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import prepare_templates
from follower_placement_sources import read_db2
from scripts.tests.test_followup_placements import copy_native_actor_fixture

MDT = Path('/home/osso/.worktrees/mdt-0ec5f50/Midnight/MagistersTerrace.lua')
POLICY = 'FOLLOWER_TRASH_HOSTILE_FROM_MDT_ENEMY_TABLE'


def fixture(source):
    target = copy_native_actor_fixture(source, 232369)
    for table, where in [('map_difficulty', 'MapID=2811'), ('content_tuning_level', 'ID=3026'),
                         ('map_challenge_mode', 'ID=558'), ('tdb_creature_template', 'entry=188252')]:
        if table in ('map_difficulty', 'map_challenge_mode'):
            target.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
        rows = source.execute(f'SELECT * FROM {table} WHERE {where}').fetchall()
        target.executemany(f"INSERT INTO {table} VALUES({','.join('?' for _ in rows[0])})", rows)
    return target


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit native/authenticated source fixtures')
class TrashSourceControls(unittest.TestCase):
    def setUp(self):
        self.source = sqlite3.connect(Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve().as_uri() + '?mode=ro', uri=True)
        self.addCleanup(self.source.close)
        self.tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('MapDifficulty', 'MapChallengeMode'))
        self.record = dict(entry=232369, map_id=2811, faction_policy=POLICY, faction_donor=188252,
                           donor_reason='MDT authored enemy table, faction only; no own stats/model substitution.',
                           content_tuning_policy='FOLLOWER_PLACEMENT_CONTENT_TUNING_FROM_NATIVE_MAP_DIFFICULTY',
                           mdt_enemy_source=dict(policy=POLICY, path=str(MDT), sha256=hashlib.sha256(MDT.read_bytes()).hexdigest(),
                                                 revision='0ec5f50dea16cfc19f288de91125c70fa7455961', source_line=60))

    def test_native_zero_trash_uses_mdt_hostility_not_absent_wowhead_react(self):
        with closing(fixture(self.source)) as target:
            donors, difficulties = prepare_templates(target, [self.record], Path(os.environ['FOLLOWER_PLACEMENT_CACHE']), self.tables)
            self.assertEqual(target.execute('SELECT faction,unit_class,ContentTuningID,HealthModifier FROM content_creature_template WHERE entry=232369').fetchone(), (16, 2, 0, 4.5))
            self.assertEqual(target.execute('SELECT faction FROM tdb_creature_template WHERE entry=232369').fetchone(), (0,))
            self.assertEqual(difficulties[0]['source_row']['ContentTuningID'], 0)
            proof = donors[0]['hostility_source']
            self.assertEqual(proof['policy'], POLICY)
            self.assertEqual(proof['mdt_enemy']['entry'], 232369)
            self.assertEqual(proof['mdt_enemy']['name'], 'Arcane Magister')
            self.assertEqual(proof['mdt_enemy']['source_line'], 60)
            self.assertEqual(proof['db2_rows']['MapChallengeMode']['MapID'], '2811')

    def test_mdt_source_drift_wrong_dungeon_nonmember_and_donor_not16_refuse(self):
        for mutation in ('hash', 'line', 'map', 'nonmember', 'donor'):
            with self.subTest(mutation=mutation), closing(fixture(self.source)) as target:
                record = copy.deepcopy(self.record)
                if mutation == 'hash':
                    record['mdt_enemy_source']['sha256'] = '0' * 64
                elif mutation == 'line':
                    record['mdt_enemy_source']['source_line'] = 149
                elif mutation == 'map':
                    record['map_id'] = 2993
                    target.execute('UPDATE tdb_creature_template_difficulty SET ContentTuningID=2718 WHERE Entry=232369')
                elif mutation == 'nonmember':
                    target.execute("UPDATE tdb_creature_template SET entry=1 WHERE entry=232369")
                    target.execute('UPDATE tdb_creature_template_difficulty SET Entry=1 WHERE Entry=232369')
                    record['entry'] = 1
                else:
                    target.execute('UPDATE tdb_creature_template SET faction=14 WHERE entry=188252')
                expected = {'hash': 'approved pinned file', 'line': 'unique enemy entry', 'map': 'requested dungeon',
                            'nonmember': 'unique enemy entry', 'donor': 'native faction16'}[mutation]
                with self.assertRaisesRegex(ValueError, expected):
                    prepare_templates(target, [record], Path(os.environ['FOLLOWER_PLACEMENT_CACHE']), self.tables)

    def test_native_nonzero_is_never_replaced_by_mdt_hostility(self):
        with closing(fixture(self.source)) as target:
            target.execute('UPDATE tdb_creature_template SET faction=14 WHERE entry=232369')
            donors, _ = prepare_templates(target, [self.record], Path('/unused-cache'), self.tables)
            self.assertEqual(donors, [])
            self.assertEqual(target.execute('SELECT faction FROM content_creature_template WHERE entry=232369').fetchone(), (14,))


@unittest.skipUnless('FOLLOWER_TRASH_DB' in os.environ, 'requires explicit private/prepared DB fixture')
class MagistersTrashAcceptance(unittest.TestCase):
    def test_three_unique_surface_markers_are_independent_native_template_spawns(self):
        path = Path(os.environ['FOLLOWER_TRASH_DB']).resolve()
        expected = [(63811100, 232369, 11443.16612, -5037.9, 91.92210097074138),
                    (63811101, 234089, 11562.92, -4962.4, 164.19668309117662),
                    (63811102, 234089, 11462.670297, -5024.6875, 91.8625331085667)]
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=2811 AND guid BETWEEN 63811100 AND 63811999 ORDER BY guid').fetchall(), expected)
            for entry in (232369, 234089):
                native = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone()
                actual = db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=? AND faction=16', (entry,)).fetchone()
                self.assertEqual(actual, native)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(), db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
            self.assertEqual(db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=2811 AND creature_entry IN(232369,234089)').fetchall(), [])


if __name__ == '__main__':
    unittest.main()
