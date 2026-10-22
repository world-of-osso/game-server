"""Pinned MDT boss identity, without aliasing duplicate native templates."""
from contextlib import closing
import copy
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import read_db2
from follower_placements import derive_binding, derive_exact_native_binding, qualify_journal_hostility, read_binding_tables

POLICY = 'BOSS_IDENTITY_FROM_PINNED_MDT_BOSS_ENTRY'
MDT = dict(path='/home/osso/.worktrees/mdt-0ec5f50/Midnight/NexusPointXenas.lua',
           revision='0ec5f50dea16cfc19f288de91125c70fa7455961',
           sha256='6ac0c7f134cbc7e7a4e0c3d27e8e64a21db19f66f075c11a2f5e2ed49cbd9e01',
           source_line=1253)
IDENTITY = dict(policy=POLICY, journal_encounter_id=2814, journal_actor_id=6001, mdt_boss_source=MDT)


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit native/source database fixtures')
class XenasIdentitySource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.path = Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve(strict=True)
        cls.tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('MapDifficulty', 'MapChallengeMode'))
        binding, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        cls.tables.update(binding)

    def test_mdt_boss_disambiguates_nysarra_without_donating_conflicting_stats(self):
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            result = derive_exact_native_binding(db, self.tables, 2915, 241542, IDENTITY)
            self.assertEqual(result[0:2], (2915, 241542))
            self.assertEqual(result[4:], (2814, 6001, 131511))
            proof = qualify_journal_hostility(db, dict(entry=241542, map_id=2915, journal_actor_source=IDENTITY), self.tables)
        identity = proof['identity_source']
        self.assertEqual(identity['policy'], POLICY)
        self.assertEqual(identity['mdt_enemy']['entry'], 241542)
        self.assertTrue(identity['mdt_enemy']['is_boss'])
        self.assertEqual(identity['mdt_enemy']['source_line'], 1253)
        self.assertEqual(identity['conflict_candidates'], [254227])
        self.assertEqual(int(identity['db2_rows']['MapChallengeMode']['MapID']), 2915)

    def test_old_exact_policy_still_refuses_duplicate_and_conflicting_entry_is_not_alias(self):
        old = dict(IDENTITY, policy='FOLLOWER_BINDING_EXACT_NATIVE_NAME_DISPLAY_TO_JOURNAL_ACTOR')
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            with self.assertRaisesRegex(ValueError, 'ambiguous template'):
                derive_binding(db, {}, self.tables, 2915, 241542, old)
            with self.assertRaises(ValueError):
                derive_binding(db, {}, self.tables, 2915, 254227, IDENTITY)

    def test_pin_map_boss_membership_and_exact_actor_remain_required(self):
        bad_hash = copy.deepcopy(IDENTITY)
        bad_hash['mdt_boss_source']['sha256'] = '0' * 64
        bad_line = copy.deepcopy(IDENTITY)
        bad_line['mdt_boss_source']['source_line'] += 1
        for identity in (bad_hash, bad_line, dict(IDENTITY, journal_actor_id=6002)):
            with self.subTest(identity=identity), closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
                with self.assertRaises(ValueError):
                    derive_binding(db, {}, self.tables, 2915, 241542, identity)
        changed = copy.deepcopy(self.tables)
        next(a for a in changed['JournalEncounterCreature'] if a['ID'] == '6001')['CreatureDisplayInfoID'] = '1'
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            with self.assertRaisesRegex(ValueError, 'exact native name and display'):
                derive_binding(db, {}, changed, 2915, 241542, IDENTITY)
            with self.assertRaises(ValueError):
                derive_binding(db, {}, self.tables, 2805, 241542, IDENTITY)
            with self.assertRaises(ValueError):
                derive_binding(db, {}, self.tables, 2915, 241643, IDENTITY)


@unittest.skipUnless('XENAS_PLACEMENT_DB' in os.environ, 'requires explicit source/prepared database')
class XenasPlacementAcceptance(unittest.TestCase):
    def test_nysarra_own_template_model_pose_binding_and_conflict_exclusion(self):
        path = Path(os.environ['XENAS_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=2915').fetchall(),
                             [(63915000, 241542, 443.9852565, 7.346653899999978, 148.82808)])
            self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,faction FROM content_creature_template WHERE entry=241542').fetchone(),
                             (1, 0, 100.0, 16))
            self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=241542').fetchall(),
                             db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=241542').fetchall())
            self.assertEqual(db.execute('SELECT creature_entry,journal_encounter_id,display_id FROM raid_encounter_creature WHERE map_id=2915').fetchall(),
                             [(241542, 2814, 131511)])
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template WHERE entry=254227').fetchone()[0], 0)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual(receipt['sql_replay_count'], 3)
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2915'])
        self.assertEqual(receipt['placements'][0]['all_surface_heights'], [148.82808])
        self.assertEqual(receipt['donors'][0]['hostility_source']['identity_source']['conflict_candidates'], [254227])


if __name__ == '__main__':
    unittest.main()
