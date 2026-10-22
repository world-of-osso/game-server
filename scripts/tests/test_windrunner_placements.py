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
from follower_placements import derive_placement, read_binding_tables

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


if __name__ == '__main__':
    unittest.main()
