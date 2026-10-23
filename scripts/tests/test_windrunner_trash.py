"""Windrunner own observations: exclude boss helper, cap actual positions by MDT counts."""
from collections import Counter
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from follower_placement_sources import digest, read_cached_npc, read_db2, read_surface_polygons
from follower_trash import read_trash_markers


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires authenticated source and fixed own-page witnesses')
class WindrunnerTrashSource(unittest.TestCase):
    def test_updraft_encounter_helper_is_excluded_despite_real_single_z_observations(self):
        manifest = json.loads((ROOT / 'content/follower-placements/windrunner-trash.json').read_text())
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('JournalEncounterSection',))
        cache = Path(os.environ['FOLLOWER_PLACEMENT_CACHE'])
        page = read_cached_npc(cache, 232118)
        self.assertEqual(len(page['markers']), 227)
        section = next(r for r in tables['JournalEncounterSection'] if r['ID'] == '30597')
        self.assertEqual((section['JournalEncounterID'], section['SpellID']), ('2655', '466556'))
        polygons = read_surface_polygons(Path(manifest['surface_proof']['path']).with_name('windrunner-trash-covering-surfaces.csv'))
        pages, points, gaps = read_trash_markers(manifest, tables, cache, polygons)
        self.assertEqual(len(points), 372)
        self.assertNotIn(232118, {p['entry'] for p in points})
        self.assertEqual(sum(len(p['markers']) for p in pages), 4539)
        excluded = next(g for g in gaps if g['entry'] == 232118)
        self.assertEqual(excluded['status'], 'FOLLOWER_TRASH_ENCOUNTER_ADD_EXCLUDED')
        self.assertEqual((excluded['excluded_single_z_count'], excluded['excluded_density_count']), (56, 3))
        self.assertEqual(digest(excluded['source_proof']), excluded['source_proof_sha256'])
        proof = json.loads(Path(excluded['source_proof']).read_text())
        self.assertEqual([(r['SpellID'], r['Effect'], r['EffectMiscValue_0']) for r in proof['native_summon_effects']], [(465954, 28, 232118)])
        self.assertIn('No native encoded edge', proof['open_proof_point'])


@unittest.skipUnless('WINDRUNNER_TRASH_DB' in os.environ, 'requires explicit source/prepared fixture')
class WindrunnerTrashAcceptance(unittest.TestCase):
    def test129_observed_poses_keep_own_native_stats_models_density_and_all_bosses(self):
        path = Path(os.environ['WINDRUNNER_TRASH_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                              'WHERE map=2805 AND guid BETWEEN 63805100 AND 63805999 ORDER BY guid').fetchall()
            self.assertEqual(len(rows), 129)
            self.assertEqual([r[0] for r in rows], list(range(63805100, 63805229)))
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            witness_path = Path(receipt['surface_proof']['path'])
            witness = json.loads(witness_path.read_text())
            oracle = json.loads(witness_path.with_name('windrunner-density-selection-reference.json').read_text())
            self.assertEqual(sorted([p['entry'], p['marker_index']] for p in receipt['placements']), oracle['selected_marker_keys'])
            expected_counts = {r['entry']: r['selected_count'] for r in oracle['per_npc'] if r['selected_count']}
            self.assertEqual(dict(Counter(r[1] for r in rows)), expected_counts)
            by_marker = {(p['entry'], p['marker_index']): p for p in witness['markers']}
            actual = {r[0]: r[1:] for r in rows}
            for point in receipt['placements']:
                source = by_marker[(point['entry'], point['marker_index'])]
                self.assertEqual(source['status'], 'UNIQUE_BAKED_SURFACE')
                self.assertEqual(point['marker'], source['marker'])
                self.assertEqual(actual[point['guid']], (point['entry'], *source['wow_xy'], source['all_surface_heights'][0]))
            for entry in expected_counts:
                own = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier '
                                 'FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry '
                                 'WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone()
                self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier '
                                            'FROM content_creature_template WHERE entry=? AND faction=16', (entry,)).fetchone(), own)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(),
                                 db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
            self.assertEqual(db.execute('SELECT guid,id1 FROM content_creature WHERE map=2805 AND guid BETWEEN 63805000 AND 63805099 ORDER BY guid').fetchall(),
                             [(63805000,231606),(63805001,231626),(63805002,231629),(63805003,231631),(63805004,231636)])
            self.assertEqual(db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=2805 ORDER BY creature_entry').fetchall(),
                             [(231606,),(231626,),(231629,),(231631,),(231636,)])
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2805 AND id1=232118').fetchone(), (0,))
        density = receipt['density']
        self.assertEqual((density['qualified_count'], density['selected_count'], density['excluded_count']), (372,129,243))
        self.assertEqual(sum(r['shortfall'] for r in density['per_npc']), 78)
        self.assertEqual(receipt['marker_count'], 4539)
        self.assertEqual(len(receipt['source_gaps']), 4170)
        self.assertEqual({g['entry'] for g in receipt['source_gaps'] if g.get('marker_count',0) is None}, {232121,232122})
        self.assertEqual(next(g for g in receipt['source_gaps'] if g['entry']==232118)['excluded_density_count'], 3)
        self.assertFalse(receipt['pack_groups_created'])
        self.assertEqual(receipt['preserved_group_tables']['baseline'], receipt['preserved_group_tables']['replayed'])
        self.assertEqual(receipt['changed_keys']['raid_encounter_creature'], [])
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2805'])
        self.assertEqual(receipt['route_queries'], [])
        self.assertEqual(receipt['sql_replay_count'], 3)


if __name__ == '__main__':
    unittest.main()
