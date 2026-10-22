"""Windrunner source-floor and actual database placement proofs."""
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import read_db2, read_surface_polygons
from follower_placements import derive_binding, derive_placement, qualify_journal_hostility, read_binding_tables, validate_sourced_duo

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless('WINDRUNNER_PLACEMENT_DB' in os.environ, 'requires explicit source/prepared database')
class WindrunnerPlacementAcceptance(unittest.TestCase):
    def test_emberdawn_keeps_native_identity_budget_and_source_selected_floor(self):
        path = Path(os.environ['WINDRUNNER_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=2805').fetchall(),
                             [(63805000, 231606, 5318.19327832, -3306.8839218000003, 90.35607999999999)])
            own = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier '
                             'FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry '
                             'WHERE t.entry=231606 AND d.DifficultyID=0').fetchone()
            self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier '
                                        'FROM content_creature_template WHERE entry=231606 AND faction=16').fetchone(), own)
            self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=231606').fetchall(),
                             db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=231606').fetchall())
            binding = db.execute('SELECT creature_entry,journal_encounter_id,display_id FROM raid_encounter_creature WHERE map_id=2805').fetchall()
            self.assertEqual(binding, [(231606, 2655, 123453)])
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        point = receipt['placements'][0]
        self.assertEqual(point['all_surface_heights'], [87.95608, 90.35607999999999])
        self.assertEqual(point['wow_xyz'][-1], 90.35607999999999)
        self.assertEqual(point['source_floor_evidence']['policy'], 'FOLLOWER_PLACEMENT_Z_FROM_SOURCE_FLOOR_UPWARD_CONSENSUS')
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2805'])
        self.assertEqual(receipt['navigation_gaps'], [])
        self.assertTrue(all(q['status'] == 'NOT_QUERIED_ENTRANCE_UNSOURCED' for q in point['surface_queries']))
        self.assertEqual(receipt['sql_replay_count'], 3)


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires authenticated source and fixed local geometry')
class WindrunnerFloorSource(unittest.TestCase):
    def test_source_consensus_single_z_without_entrance_does_not_query_route(self):
        manifest = json.loads((ROOT / 'content/follower-placements/windrunner-emberdawn.json').read_text())
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        binding, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        record = manifest['entries'][0]
        surfaces = read_surface_polygons(Path(manifest['surface_proof']['path']).with_suffix('.csv'))
        def place(actor):
            return derive_placement(actor, manifest, tables | binding, Path(os.environ['FOLLOWER_PLACEMENT_CACHE']),
                                    surfaces, Path('/route-must-not-run'), Path('/nav-must-not-open'), None)
        point = place(record)
        self.assertEqual(point['all_surface_heights'], [87.95608, 90.35607999999999])
        self.assertEqual(point['wow_xyz'], [5318.19327832, -3306.8839218000003, 90.35607999999999])
        unqualified = {k: v for k, v in record.items() if k != 'source_floor_evidence'}
        with self.assertRaisesRegex(ValueError, 'requires one'):
            place(unqualified)


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit native/source fixtures')
class WindrunnerPreviewIdentity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.path = Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve(strict=True)
        cls.tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('MapDifficulty', 'MapChallengeMode'))
        binding, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        cls.tables.update(binding)
        from follower_placement_sources import digest
        path = Path('/home/osso/.worktrees/mdt-0ec5f50/Midnight/WindrunnerSpire.lua')
        cls.mdt = dict(path=str(path), sha256=digest(path), revision='0ec5f50dea16cfc19f288de91125c70fa7455961')

    def identity(self, journal, actor):
        return dict(policy='JEC_EXACT_NAME_UNIQUE_DISPLAY_PREVIEW', journal_encounter_id=journal,
                    journal_actor_id=actor, mdt_boss_source=self.mdt)

    def test_four_unique_names_use_preview_display_without_replacing_native_model(self):
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            for entry, journal, actor, preview, native in ((231626,2656,5839,91068,125201),
                (231629,2656,5840,125198,124335),(231631,2657,5845,139034,122981),(231636,2658,5844,125200,125199)):
                with self.subTest(entry=entry):
                    identity = self.identity(journal, actor)
                    binding = derive_binding(db, {}, self.tables, 2805, entry, identity)
                    self.assertEqual(binding[0:2], (2805,entry))
                    self.assertEqual(binding[4:], (journal,actor,preview))
                    proof = qualify_journal_hostility(db, dict(entry=entry,map_id=2805,journal_actor_source=identity), self.tables)
                    evidence = proof['identity_source']
                    self.assertEqual(evidence['preview_display_id'], preview)
                    self.assertEqual(evidence['native_display_ids'], [native])
                    self.assertEqual(evidence['display_role'], 'JOURNAL_PREVIEW_ONLY_RUNTIME_MODELS_PRESERVED')
                    self.assertTrue(evidence['display_mismatch'])
                    self.assertEqual(db.execute('SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?',(entry,)).fetchall(), [(native,)])

    def test_name_alias_wrong_map_and_changed_pin_remain_refused(self):
        import copy
        identity = self.identity(2657,5845)
        bad = copy.deepcopy(identity)
        bad['mdt_boss_source']['sha256'] = '0'*64
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            for args in ((2805,231631,bad),(2915,231631,identity),(2915,241539,self.identity(2813,6000))):
                with self.subTest(args=args), self.assertRaises(ValueError):
                    derive_binding(db,{},self.tables,*args)

    def test_duplicate_native_name_with_different_model_is_not_disambiguated_by_preview(self):
        from scripts.tests.test_followup_placements import copy_native_actor_fixture
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as source, closing(copy_native_actor_fixture(source,231631)) as db:
            columns = [row[1] for row in db.execute('PRAGMA table_info(tdb_creature_template)')]
            row = list(db.execute('SELECT * FROM tdb_creature_template WHERE entry=231631').fetchone())
            row[columns.index('entry')] = 999999
            db.execute(f"INSERT INTO tdb_creature_template VALUES({','.join('?' for _ in row)})", row)
            with self.assertRaisesRegex(ValueError,'unique native name'):
                derive_binding(db,{},self.tables,2805,231631,self.identity(2657,5845))


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires authenticated source and fixed geometry')
class WindrunnerAdditionalFloor(unittest.TestCase):
    def test_duo_bounds_select_nav_floor_and_both_individual_pages(self):
        manifest = json.loads((ROOT / 'content/follower-placements/windrunner-duo.json').read_text())
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('JournalEncounter',))
        polygons = read_surface_polygons(Path(manifest['surface_proof']['path']).with_suffix('.csv'))
        points = [derive_placement(r,manifest,tables,Path(os.environ['FOLLOWER_PLACEMENT_CACHE']),polygons,Path('/route-must-not-run'),Path('/nav-must-not-open'),None) for r in manifest['entries']]
        self.assertEqual([p['wow_xyz'] for p in points], [[5044.337123386993,-3161.0082888735,101.4178188648423]]*2)
        for p in points:
            self.assertEqual(p['all_surface_heights'], [25.73321858713429,89.07200098565106,101.4178188648423])
            self.assertEqual([g['supported_baked_candidates'] for g in p['source_floor_evidence']['corroborating_assigned_group_bounds']], [[101.4178188648423]]*2)
        proof = validate_sourced_duo(manifest,points,[],Path(os.environ['FOLLOWER_PLACEMENT_CACHE']))
        self.assertEqual([p['entry'] for p in proof[0]['individual_pages']], [231626,231629])
        self.assertEqual(proof[0]['individual_pages'][0]['markers'], [dict(zone=15808,floor=4,percent=[52.4,88.5])])
        import copy
        changed = copy.deepcopy(manifest)
        changed['entries'][1]['cache_sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'marker source changed'):
            validate_sourced_duo(changed,points,[],Path(os.environ['FOLLOWER_PLACEMENT_CACHE']))
        unsupported = {k:v for k,v in manifest['entries'][0].items() if k not in ('source_floor_evidence','source_floor_z_bounds')}
        with self.assertRaisesRegex(ValueError,'requires one'):
            derive_placement(unsupported,manifest,tables,Path(os.environ['FOLLOWER_PLACEMENT_CACHE']),polygons,Path('/unused'),Path('/unused'),None)

    def test_kroluk_and_heart_upward_consensus_keep_raw_candidates(self):
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('JournalEncounter',))
        for label,z,raw_count in (('kroluk',100.26369,8),('heart',342.57331613991164,7)):
            with self.subTest(label=label):
                m=json.loads((ROOT/f'content/follower-placements/windrunner-{label}.json').read_text())
                polygons=read_surface_polygons(Path(m['surface_proof']['path']).with_suffix('.csv'))
                p=derive_placement(m['entries'][0],m,tables,Path(os.environ['FOLLOWER_PLACEMENT_CACHE']),polygons,Path('/unused'),Path('/unused'),None)
                self.assertEqual(p['wow_xyz'][2],z)
                self.assertEqual(len(p['all_surface_heights']),raw_count)
                self.assertEqual(p['status'],'SOURCE_PLACEMENT_ENTRANCE_UNSOURCED')


@unittest.skipUnless('WINDRUNNER_PREVIEW_DB' in os.environ,'requires explicit source/prepared database')
class WindrunnerPreviewPlacementAcceptance(unittest.TestCase):
    def test_packet_keeps_native_models_modifiers_preview_binding_and_source_pose(self):
        path=Path(os.environ['WINDRUNNER_PREVIEW_DB']).resolve(strict=True)
        expected={231626:(63805001,5044.337123386993,-3161.0082888735,101.4178188648423,91068),
                  231629:(63805002,5044.337123386993,-3161.0082888735,101.4178188648423,125198),
                  231631:(63805003,5216.794991273334,-3116.47772139,100.26369,139034),
                  231636:(63805004,5216.4426994,-3137.70755135,342.57331613991164,125200)}
        label=os.environ['WINDRUNNER_PACKET_LABEL']
        entries={'duo':[231626,231629],'kroluk':[231631],'heart':[231636]}[label]
        with closing(sqlite3.connect(path.as_uri()+'?mode=ro',uri=True)) as db:
            for entry in entries:
                guid,x,y,z,preview=expected[entry]
                self.assertEqual(db.execute('SELECT guid,position_x,position_y,position_z FROM content_creature WHERE map=2805 AND id1=?',(entry,)).fetchall(),[(guid,x,y,z)])
                self.assertEqual(db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=? AND d.DifficultyID=0',(entry,)).fetchone(),
                                 db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=? AND faction=16',(entry,)).fetchone())
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=?',(entry,)).fetchall(),db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=?',(entry,)).fetchall())
                self.assertEqual(db.execute('SELECT display_id FROM raid_encounter_creature WHERE map_id=2805 AND creature_entry=?',(entry,)).fetchone(),(preview,))
                self.assertNotIn(preview,[r[0] for r in db.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=?',(entry,))])
        receipt=json.loads((path.parent/'receipt.json').read_text())
        self.assertEqual(receipt['sql_replay_count'],3)
        self.assertEqual(receipt['entrance_gaps'],['FOLLOWER_ENTRANCE_UNSOURCED_2805'])
        self.assertEqual(receipt['navigation_gaps'],[])
        for p in receipt['placements']:
            self.assertTrue(p['preview_identity']['display_mismatch'])
            self.assertEqual(p['preview_identity']['display_role'],'JOURNAL_PREVIEW_ONLY_RUNTIME_MODELS_PRESERVED')
        if label=='duo':self.assertEqual(len(receipt['source_relationships'][0]['individual_pages']),2)


if __name__ == '__main__':
    unittest.main()
