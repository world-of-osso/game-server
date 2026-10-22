"""Muro journal placement with native Nekraxx kept at its authored source pose."""
from contextlib import closing
import copy
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import follower_placements


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ, 'requires explicit native database')
class NativeDuoSource(unittest.TestCase):
    def open_source(self):
        path = Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve(strict=True)
        return sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)

    def read_manifest(self):
        return json.loads((Path(__file__).resolve().parents[2] /
                           'content/follower-placements/maisara-muro-nekraxx.json').read_text())

    def test_native_binding_source_preserves_pose_and_rejects_drift(self):
        manifest = self.read_manifest()
        with closing(self.open_source()) as source:
            proof = follower_placements.read_native_binding_entries(source, manifest)[0]
            self.assertEqual(proof['native_spawn']['VerifiedBuild'], 66431)
            self.assertEqual(proof['runtime_spawn']['guid'], 31000176)
            for mutation in ('UPDATE content_creature SET position_x=position_x+1',
                             "UPDATE content_creature SET spawnDifficulties='1'",
                             'UPDATE tdb_creature SET VerifiedBuild=70448'):
                with self.subTest(mutation=mutation), closing(sqlite3.connect(':memory:')) as changed:
                    for table, guid in [('tdb_creature', 11000176), ('content_creature', 31000176)]:
                        changed.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
                        row = source.execute(f'SELECT * FROM {table} WHERE guid=?', (guid,)).fetchone()
                        changed.execute(f"INSERT INTO {table} VALUES({','.join('?' for _ in row)})", row)
                    changed.execute(mutation)
                    with self.assertRaisesRegex(ValueError, 'exact authored pose/mask/build'):
                        follower_placements.read_native_binding_entries(changed, manifest)

    def test_both_own_markers_and_precise_journal_are_recorded_without_relocating_nekraxx(self):
        from follower_placement_sources import read_db2, read_surface_polygons
        manifest = self.read_manifest()
        db2 = Path(os.environ['FOLLOWER_PLACEMENT_DB2'])
        tables, _ = read_db2(db2)
        binding, _ = follower_placements.read_binding_tables(db2)
        surfaces = Path(os.environ['MIDNIGHT_DUO_SURFACES'])
        cache = Path('/home/osso/.cache/wowhead-cache')
        placement = follower_placements.derive_placement(manifest['entries'][0], manifest, tables | binding,
                    cache, read_surface_polygons(surfaces), Path('/must-not-run-route'), Path('unused'), None)
        with closing(self.open_source()) as source:
            native = follower_placements.read_native_binding_entries(source, manifest)
        proof = follower_placements.validate_sourced_duo(manifest, [placement], native, cache)[0]
        self.assertEqual(proof['journal_xy'], [-916.5888856148282, 164.45449613761912])
        self.assertEqual([p['markers'][0]['percent'] for p in proof['individual_pages']], [[52.2, 68.6], [52.2, 68.6]])
        self.assertEqual(proof['journal_marker']['percent'], [52.2145987, 68.6129987])
        self.assertEqual(proof['native_corroboration']['runtime_spawn']['position_x'], -919.337)
        changed = copy.deepcopy(manifest)
        changed['native_binding_only_entries'][0]['cache_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'individual marker source changed'):
            follower_placements.validate_sourced_duo(changed, [placement], native, cache)

    def test_missing_muro_and_native_nekraxx_binding_are_present(self):
        with closing(self.open_source()) as db:
            for label, sql in [
                ('Muro placement', 'SELECT count(*) FROM content_creature WHERE map=2874 AND id1=247570'),
                ('Nekraxx native binding', 'SELECT count(*) FROM raid_encounter_creature WHERE map_id=2874 AND creature_entry=247572')]:
                with self.subTest(label=label):
                    self.assertEqual(db.execute(sql).fetchone()[0], 1, f'missing {label}')


@unittest.skipUnless('MIDNIGHT_PLACEMENT_DB' in os.environ, 'requires explicit prepared packet')
class NativeDuoAcceptance(unittest.TestCase):
    def test_muro_is_new_but_nekraxx_only_gains_a_binding(self):
        path = Path(os.environ['MIDNIGHT_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                                        'WHERE map=2874 AND id1 IN(247570,247572) ORDER BY id1').fetchall(), [
                (63874000, 247570, -916.5888856148282, 164.45449613761912, 99.89445016939732),
                (31000176, 247572, -919.337, 165.427, 99.8452)])
            self.assertEqual(db.execute('SELECT spawnDifficulties,VerifiedBuild FROM content_creature WHERE guid=31000176').fetchone(),
                             ('1,2,8,23,205', 66431))
            self.assertEqual(db.execute('SELECT creature_entry,journal_encounter_id,journal_creature_id,display_id '
                                        'FROM raid_encounter_creature WHERE map_id=2874 AND creature_entry IN(247570,247572) '
                                        'ORDER BY creature_entry').fetchall(),
                             [(247570, 2810, 6029, 130699), (247572, 2810, 6030, 130705)])
            self.assertEqual(db.execute('SELECT entry,faction,unit_class,ContentTuningID,HealthModifier '
                                        'FROM content_creature_template WHERE entry IN(247570,247572) ORDER BY entry').fetchall(),
                             [(247570, 16, 4, 5071, 27.0), (247572, 16, 4, 5071, 27.0)])
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2874'])
        self.assertEqual(receipt['navigation_gaps'], [])
        self.assertEqual(receipt['donors'], [])
        self.assertEqual(receipt['changed_keys']['content_creature'], [[63874000]])
        self.assertEqual(receipt['changed_keys']['content_creature_template'], [[247570]])
        self.assertEqual(receipt['changed_keys']['content_creature_template_model'], [[247570, 0]])
        self.assertEqual(receipt['changed_keys']['raid_encounter_creature'], [[2874, 247570], [2874, 247572]])
        native = receipt['native_binding_only'][0]
        self.assertEqual(native['native_spawn']['guid'], 11000176)
        self.assertEqual(native['native_spawn']['VerifiedBuild'], 66431)
        self.assertEqual(native['runtime_spawn']['guid'], 31000176)
        relationship = receipt['source_relationships'][0]
        self.assertEqual(relationship['policy'], 'FOLLOWER_PLACEMENT_DUO_INDIVIDUALLY_SOURCED_SAME_MARKER')
        self.assertEqual([p['entry'] for p in relationship['individual_pages']], [247570, 247572])
        self.assertEqual(relationship['individual_pages'][0]['markers'], relationship['individual_pages'][1]['markers'])
        self.assertEqual(relationship['native_corroboration']['native_spawn']['guid'], 11000176)
        self.assertEqual(receipt['sql_replay_count'], 3)


if __name__ == '__main__':
    unittest.main()
