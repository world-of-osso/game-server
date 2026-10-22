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
from follower_placement_sources import read_db2, read_surface_polygons
from follower_trash import read_trash_markers
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


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ and 'FOLLOWER_PLACEMENT_CACHE' in os.environ,
                     'requires fixed marker and covering-nav source witnesses')
class TrashMarkerProjection(unittest.TestCase):
    def test_missing_own_mapper_remains_an_explicit_unknown_count_gap(self):
        gap = dict(entry=255000, status='FOLLOWER_TRASH_OWN_MAPPER_VARIABLE_MISSING', marker_count=None)
        manifest = dict(map_id=2923, entries=[], source_gaps=[gap])
        pages, placements, gaps = read_trash_markers(manifest, {}, Path('/unused-cache'), [])
        self.assertEqual(pages, [])
        self.assertEqual(placements, [])
        self.assertEqual(gaps, [gap])
        self.assertIsNone(gaps[0]['marker_count'])

    def test_every_grouped_and_singleton_source_point_keeps_its_exact_candidates(self):
        root = Path(__file__).resolve().parents[2]
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        for name, filename, count in [('magisters', 'magisters-all-trash-covering-surfaces.csv', 1344),
                                      ('blinding', 'blinding-trash-covering-surfaces.csv', 4541)]:
            with self.subTest(map=name):
                manifest = json.loads((root / f'content/follower-placements/{name}-trash.json').read_text())
                witness = Path(manifest['surface_proof']['path'])
                proof = json.loads(witness.read_text())
                pages, placements, gaps = read_trash_markers(manifest, tables, Path(os.environ['FOLLOWER_PLACEMENT_CACHE']),
                                                            read_surface_polygons(witness.parent / filename))
                self.assertEqual(sum(len(page['markers']) for page in pages), count)
                actual = {(point['entry'], point['marker_index']): point for point in placements + gaps if 'marker_index' in point}
                self.assertEqual(len(actual), count)
                for expected in proof['markers']:
                    key = (expected['entry'], expected['marker_index'])
                    if key not in actual:
                        self.fail(f'missing source marker {key}')
                    point = actual[key]
                    self.assertEqual((point['wow_xy'], point['all_surface_heights']),
                                     (expected['wow_xy'], expected['all_surface_heights']), f'{name} marker {key}')
                    if name == 'blinding':
                        self.assertEqual(point['observed_wowhead_floor'], 0)
                        self.assertIsNone(point['db2_group_floor_identity'])


@unittest.skipUnless('FOLLOWER_TRASH_DB' in os.environ, 'requires explicit private/prepared DB fixture')
class MagistersTrashAcceptance(unittest.TestCase):
    def test_qualified_independent_markers_keep_own_native_templates(self):
        path = Path(os.environ['FOLLOWER_TRASH_DB']).resolve()
        expected = [(63811100, 232369, 11443.16612, -5037.9, 91.92210097074138),
                    (63811101, 234089, 11562.92, -4962.4, 164.19668309117662),
                    (63811102, 234089, 11462.670297, -5024.6875, 91.8625331085667)]
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            actual_spawns = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=2811 AND guid BETWEEN 63811100 AND 63811999 ORDER BY guid').fetchall()
            self.assertEqual(len(actual_spawns), 161)
            self.assertEqual(actual_spawns[:3], expected)
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            proof_path = Path(receipt['surface_proof']['path'])
            self.assertEqual(hashlib.sha256(proof_path.read_bytes()).hexdigest(), receipt['surface_proof']['sha256'])
            proof = json.loads(proof_path.read_text())
            qualified = [point for point in proof['markers'] if point['status'] == 'UNIQUE_BAKED_SURFACE']
            self.assertEqual(actual_spawns, [(63811100 + index, point['entry'], *point['wow_xy'], point['all_surface_heights'][0])
                                             for index, point in enumerate(qualified)])
            self.assertEqual(len({row[1] for row in actual_spawns}), 9)
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=232106').fetchone(), (16,))
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2811 AND id1=232106').fetchone(), (1,))
            for entry in {row[1] for row in actual_spawns}:
                native = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone()
                actual = db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=? AND faction=16', (entry,)).fetchone()
                self.assertEqual(actual, native)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(), db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
            self.assertEqual(db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=2811 AND creature_entry IN(232369,234089)').fetchall(), [])
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual(receipt['marker_count'], 1344)
        self.assertEqual(len(receipt['placements']), 161)
        self.assertEqual(sum(g['status'] == 'FOLLOWER_TRASH_FLOOR_UNRESOLVED' for g in receipt['source_gaps']), 1183)
        self.assertEqual(sum(g['status'] == 'FOLLOWER_TRASH_INDIVIDUAL_MARKERS_MISSING' for g in receipt['source_gaps']), 10)
        self.assertFalse(receipt['pack_groups_created'])
        groups = receipt['preserved_group_tables']
        self.assertEqual(groups['baseline'], groups['prepared'])
        self.assertEqual(groups['baseline'], groups['replayed'])
        update = receipt['guarded_faction_updates'][0]
        self.assertEqual((update['entry'], update['before']['faction'], update['after']['faction']), (232106, 0, 16))
        self.assertEqual({k: v for k, v in update['before'].items() if k != 'faction'},
                         {k: v for k, v in update['after'].items() if k != 'faction'})
        self.assertEqual(receipt['sql_replay_count'], 3)


@unittest.skipUnless('BLINDING_TRASH_DB' in os.environ, 'requires explicit Blinding prepared/source fixture')
class BlindingTrashAcceptance(unittest.TestCase):
    def test_all803_single_frame_markers_keep_absent_db2_floor_and_own_templates(self):
        path = Path(os.environ['BLINDING_TRASH_DB']).resolve()
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            actual = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=2859 AND guid BETWEEN 63859100 AND 63859999 ORDER BY guid').fetchall()
            self.assertEqual(len(actual), 803)
            self.assertEqual(len({row[1] for row in actual}), 16)
            receipt = json.loads((path.parent / 'receipt.json').read_text())
            proof_path = Path(receipt['surface_proof']['path'])
            self.assertEqual(hashlib.sha256(proof_path.read_bytes()).hexdigest(), receipt['surface_proof']['sha256'])
            proof = json.loads(proof_path.read_text())
            qualified = [point for point in proof['markers'] if point['status'] == 'UNIQUE_BAKED_SURFACE']
            self.assertEqual(actual, [(63859100 + index, point['entry'], *point['wow_xy'], point['all_surface_heights'][0])
                                      for index, point in enumerate(qualified)])
            for entry in {row[1] for row in actual}:
                native = db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone()
                self.assertEqual(db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=? AND faction=16', (entry,)).fetchone(), native)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(), db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
                self.assertEqual(db.execute('SELECT count(*) FROM raid_encounter_creature WHERE map_id=2859 AND creature_entry=?', (entry,)).fetchone(), (0,))
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2859 AND id1=244887').fetchone(), (1,))
        self.assertEqual(receipt['marker_count'], 4541)
        self.assertEqual(len(receipt['source_gaps']), 3738)
        self.assertEqual(receipt['guarded_faction_updates'], [])
        self.assertEqual(receipt['entrance_gaps'], ['FOLLOWER_ENTRANCE_UNSOURCED_2859'])
        self.assertFalse(receipt['pack_groups_created'])
        self.assertEqual(receipt['preserved_group_tables']['baseline'], receipt['preserved_group_tables']['replayed'])
        for point in receipt['placements']:
            self.assertEqual(point['projection_policy'], 'FOLLOWER_PLACEMENT_SINGLE_ASSIGNMENT_UIMAP_FRAME')
            self.assertEqual(point['observed_wowhead_floor'], 0)
            self.assertIsNone(point['db2_group_floor_identity'])
            self.assertEqual([r['ID'] for r in point['assignments']], ['69155'])
        self.assertEqual(receipt['sql_replay_count'], 3)


if __name__ == '__main__':
    unittest.main()
