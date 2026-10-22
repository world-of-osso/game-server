"""Actual-map packet proofs against explicit canonical/prepared SQLite fixtures."""
from collections import Counter
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import read_cached_npc, read_db2
from follower_trash import project_marker


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires explicit authenticated source and own-page fixture')
class MurderMarkerSourceControls(unittest.TestCase):
    def test_observed_defiled_golem_negative_zone_remains_unprojected_gap(self):
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        page = read_cached_npc(Path(os.environ['FOLLOWER_PLACEMENT_CACHE']), 235322)
        self.assertEqual(page['markers'][322], dict(zone=-15, floor=0, percent=[29.3, 33]))
        point = project_marker(2813, page, 322, tables, {}, {})
        self.assertEqual(point['status'], 'FOLLOWER_TRASH_MARKER_PROJECTION_UNRESOLVED')
        self.assertIn('AreaTable', point['reason'])
        self.assertEqual(point['marker'], page['markers'][322])
        self.assertNotIn('wow_xyz', point)


@unittest.skipUnless('ALTAR_TRASH_DB' in os.environ, 'requires explicit Altar source/prepared fixture')
class AltarTrashAcceptance(unittest.TestCase):
    def test49_individual_observations_keep_native_data_and_per_type_density(self):
        path = Path(os.environ['ALTAR_TRASH_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                              'WHERE map=2993 AND guid BETWEEN 63993100 AND 63993999 ORDER BY guid').fetchall()
            self.assertEqual(len(rows), 49)
            self.assertEqual([r[0] for r in rows], list(range(63993100, 63993149)))
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            witness_path = Path(receipt['surface_proof']['path'])
            witness = json.loads(witness_path.read_text())
            oracle = json.loads(witness_path.with_name('altar-density-selection-reference.json').read_text())
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
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2993 AND id1=259445').fetchone(), (1,))
        density = receipt['density']
        self.assertEqual((density['qualified_count'], density['selected_count'], density['excluded_count']), (1191, 49, 1142))
        self.assertEqual(sum(r['shortfall'] for r in density['per_npc']), 108)
        self.assertEqual(receipt['marker_count'], 3012)
        self.assertEqual(len(receipt['source_gaps']), 1822)
        self.assertEqual(next(g for g in receipt['source_gaps'] if g['entry'] == 268358)['marker_count'], None)
        self.assertFalse(receipt['pack_groups_created'])
        self.assertEqual(receipt['preserved_group_tables']['baseline'], receipt['preserved_group_tables']['replayed'])
        self.assertEqual(receipt['changed_keys']['raid_encounter_creature'], [])
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2993'])
        self.assertEqual(receipt['route_queries'], [])
        self.assertEqual(receipt['sql_replay_count'], 3)


@unittest.skipUnless('MURDER_TRASH_DB' in os.environ, 'requires explicit Murder source/prepared fixture')
class MurderTrashAcceptance(unittest.TestCase):
    def test128_individual_observations_keep_own_entries_and_density(self):
        path = Path(os.environ['MURDER_TRASH_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                              'WHERE map=2813 AND guid BETWEEN 63813100 AND 63813999 ORDER BY guid').fetchall()
            self.assertEqual(len(rows), 128)
            self.assertEqual([r[0] for r in rows], list(range(63813100, 63813228)))
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            witness_path = Path(receipt['surface_proof']['path'])
            witness = json.loads(witness_path.read_text())
            oracle = json.loads(witness_path.with_name('murder-density-selection-reference.json').read_text())
            self.assertEqual(sorted([p['entry'], p['marker_index']] for p in receipt['placements']), oracle['selected_marker_keys'])
            expected_counts = {r['entry']: r['selected_count'] for r in oracle['per_npc'] if r['selected_count']}
            self.assertEqual(dict(Counter(r[1] for r in rows)), expected_counts)
            by_marker = {(p['entry'], p['marker_index']): p for p in witness['markers']}
            actual = {r[0]: r[1:] for r in rows}
            difficulties = {r['entry']: r['source_difficulty_id'] for r in receipt['source_difficulties']}
            for point in receipt['placements']:
                source = by_marker[(point['entry'], point['marker_index'])]
                self.assertEqual(source['status'], 'UNIQUE_BAKED_SURFACE')
                self.assertEqual(actual[point['guid']], (point['entry'], *source['wow_xy'], source['all_surface_heights'][0]))
            for entry in expected_counts:
                own = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier '
                                 'FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry '
                                 'WHERE t.entry=? AND d.DifficultyID=?', (entry, difficulties[entry])).fetchone()
                self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier '
                                            'FROM content_creature_template WHERE entry=? AND faction=16', (entry,)).fetchone(), own)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(),
                                 db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
            self.assertEqual(db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=2813').fetchall(), [(234648,)])
            self.assertNotIn(255050, [r['entry'] for r in receipt['donors']])
            self.assertEqual(difficulties[255050], 1)
            self.assertNotIn([255050], receipt['changed_keys']['content_creature_template'])
        density = receipt['density']
        self.assertEqual((density['qualified_count'], density['selected_count'], density['excluded_count']), (2433, 128, 2305))
        self.assertEqual(sum(r['shortfall'] for r in density['per_npc']), 92)
        self.assertEqual(receipt['marker_count'], 4896)
        self.assertEqual(len(receipt['source_gaps']), 2465)
        self.assertEqual({g['entry'] for g in receipt['source_gaps'] if g.get('marker_count', 0) is None}, {235520, 272246})
        self.assertEqual(sum(g['status'] == 'FOLLOWER_TRASH_MARKER_PROJECTION_UNRESOLVED' for g in receipt['source_gaps']), 11)
        self.assertFalse(receipt['pack_groups_created'])
        self.assertEqual(receipt['preserved_group_tables']['baseline'], receipt['preserved_group_tables']['replayed'])
        self.assertEqual(receipt['changed_keys']['raid_encounter_creature'], [])
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2813'])
        self.assertEqual(receipt['route_queries'], [])
        self.assertEqual(receipt['sql_replay_count'], 3)


if __name__ == '__main__':
    unittest.main()
