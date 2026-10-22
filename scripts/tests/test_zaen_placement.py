"""Zaen exact identity and assigned-floor proof; all published Murder rows survive."""
from contextlib import closing
import copy
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from follower_placement_sources import read_db2, read_surface_polygons
from follower_placements import derive_placement


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ, 'requires authenticated source and fixed geometry')
class ZaenFloorSource(unittest.TestCase):
    def test_only_low_baked_floor_is_supported_by_assigned_group_upward_triangle(self):
        manifest = json.loads((ROOT/'content/follower-placements/murder-zaen.json').read_text())
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('JournalEncounter',))
        polygons = read_surface_polygons(Path(manifest['surface_proof']['path']).with_suffix('.csv'))
        record = manifest['entries'][0]
        def place(record):
            return derive_placement(record, manifest, tables, Path('/unused-cache'), polygons, Path('/route-must-not-run'), Path('/unused-nav'), None)
        point = place(record)
        self.assertEqual(point['all_surface_heights'], [30.38151415540137, 65.06133, 71.861336])
        self.assertEqual(point['wow_xyz'], [8924.3223042, -4738.448827475, 30.38151415540137])
        self.assertEqual(point['source_floor_evidence']['triangles'][0]['height'], 29.989988803863525)
        changed = copy.deepcopy(record)
        del changed['source_floor_evidence']
        with self.assertRaisesRegex(ValueError, 'requires one'):
            place(changed)
        changed = copy.deepcopy(record)
        changed['source_floor_evidence']['source_files'][0]['sha256'] = '0'*64
        with self.assertRaisesRegex(ValueError, 'source WMO floor evidence file changed'):
            place(changed)


@unittest.skipUnless('ZAEN_PLACEMENT_DB' in os.environ, 'requires explicit source/prepared database')
class ZaenPlacementAcceptance(unittest.TestCase):
    def test_own_native_model_modifiers_pose_binding_and_published_population(self):
        path = Path(os.environ['ZAEN_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri()+'?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,position_x,position_y,position_z FROM content_creature WHERE map=2813 AND id1=234649').fetchall(),
                             [(63813001, 8924.3223042, -4738.448827475, 30.38151415540137)])
            self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=234649 AND faction=16').fetchone(),
                             db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=234649 AND d.DifficultyID=0').fetchone())
            self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=234649').fetchall(),
                             db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=234649').fetchall())
            self.assertEqual(db.execute('SELECT journal_encounter_id,display_id FROM raid_encounter_creature WHERE map_id=2813 AND creature_entry=234649').fetchone(), (2680,124592))
            self.assertEqual(db.execute('SELECT guid,id1 FROM content_creature WHERE map=2813 AND guid NOT BETWEEN 63813000 AND 63813999 ORDER BY guid').fetchall(), [(31000647,234648),(31000648,234660)])
            old_path = Path(json.loads((ROOT/'content/follower-placements/murder-zaen.json').read_text())['preserved_placement_sources'][0]['receipt'])
            old = json.loads(old_path.read_text())
            with closing(sqlite3.connect((old_path.parent/'replayed.sqlite').as_uri()+'?mode=ro',uri=True)) as previous:
                for point in old['placements']:
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE guid=?',(point['guid'],)).fetchone(),
                                     previous.execute('SELECT * FROM content_creature WHERE guid=?',(point['guid'],)).fetchone())
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE id1 IN(259446,238887)').fetchone(), (2,))
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2805 AND guid BETWEEN 63805100 AND 63805999').fetchone(), (129,))
        receipt = json.loads((path.parent/'receipt.json').read_text())
        self.assertEqual(receipt['sql_replay_count'], 3)
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2813'])
        self.assertEqual(receipt['navigation_gaps'], [])
        self.assertFalse(receipt['placements'][0]['preview_identity']['display_mismatch'])
        self.assertEqual(receipt['placements'][0]['all_surface_heights'], [30.38151415540137,65.06133,71.861336])


if __name__ == '__main__':
    unittest.main()
