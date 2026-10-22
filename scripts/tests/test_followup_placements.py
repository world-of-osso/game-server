"""Sourced primaries for Halls of Infusion and the absent Academy Vexamus."""
from contextlib import closing
import os
import math
import json
from pathlib import Path
import sqlite3
import unittest
import copy
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import derive_binding, prepare_templates, read_binding_tables, select_surface_query, validate_native_room_anchors


def copy_native_actor_fixture(source, entry):
    target = sqlite3.connect(':memory:')
    tables = {
        'tdb_creature_template': f'entry={entry}',
        'tdb_creature_template_difficulty': f'Entry={entry}',
        'tdb_creature_template_model': f'CreatureID={entry}',
        'content_tuning_level': 'ID=2718',
        'difficulty': '1',
        'creature_template': '0',
        'content_creature_template': '0',
        'content_creature_template_model': '0',
    }
    for table, where in tables.items():
        target.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
        rows = source.execute(f'SELECT * FROM {table} WHERE {where}').fetchall()
        if rows:
            target.executemany(f"INSERT INTO {table} VALUES({','.join('?' for _ in rows[0])})", rows)
    return target


@unittest.skipUnless('FOLLOWUP_PLACEMENT_DB' in os.environ, 'requires an explicit prepared database copy')
class FollowupPlacementAcceptance(unittest.TestCase):
    def assert_source_primaries(self, map_id, entries, source_difficulty=1):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        marks = ','.join('?' for _ in entries)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute(
                'SELECT c.id1,t.faction,t.unit_class,s.unit_class,t.HealthModifier,d.HealthModifier,'
                't.ContentTuningID,d.ContentTuningID,r.display_id,m.CreatureDisplayID,n.CreatureDisplayID,s.faction '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN tdb_creature_template s ON s.entry=c.id1 '
                'JOIN tdb_creature_template_difficulty d ON d.Entry=c.id1 AND d.DifficultyID=? '
                'JOIN raid_encounter_creature r ON r.creature_entry=c.id1 AND r.map_id=c.map '
                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                'JOIN tdb_creature_template_model n ON n.CreatureID=c.id1 AND n.Idx=m.Idx '
                f'WHERE c.map=? AND c.id1 IN({marks}) ORDER BY c.id1,c.guid', (source_difficulty, map_id, *entries)
            ).fetchall()
        self.assertEqual([r[0] for r in rows], list(entries), f'map{map_id} missing sourced primary placements')
        for entry, faction, actual_class, native_class, health, native_health, tuning, native_tuning, display, runtime_display, native_display, native_faction in rows:
            with self.subTest(entry=entry):
                self.assertEqual(faction, native_faction if native_faction > 0 else 16)
                self.assertEqual(actual_class, native_class)
                self.assertEqual(health, native_health)
                self.assertEqual(tuning, native_tuning)
                self.assertEqual(runtime_display, native_display)
                self.assertEqual(display, {189719: 106801, 189722: 103584, 189727: 107064,
                                           189729: 106934, 194181: 107525, 186151: 107680,
                                           186338: 105722, 186615: 107145, 186616: 105823,
                                           190485: 108318, 208743: 115888, 208747: 117773, 210149: 114171,
                                           213179: 119793, 210267: 117177, 214661: 118000, 234933: 123825}[entry])

    def test_avanoxx_keeps_own_native_zero_difficulty_budget_and_identity(self):
        self.assert_source_primaries(2660, (213179,), source_difficulty=0)
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual({p['entry']: p['source_difficulty_id'] for p in receipt['source_difficulties']}, {213179: 0})
        self.assertEqual(receipt['donors'], [])
        self.assertEqual(receipt['navigation_gaps'], [])
        placement = receipt['placements'][0]
        self.assertEqual(placement['page']['assignment_resolution']['chosen_assignment_id'], 68441)
        self.assertEqual(len(placement['assignments']), 67)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT id1,map,position_x,position_y,position_z FROM content_creature WHERE guid=63660000').fetchone(),
                             (213179, 2660, -103.24485721595764, 424.63702790355273, 137.09528))
            self.assertEqual(db.execute('SELECT DifficultyID FROM tdb_creature_template_difficulty WHERE Entry=213179').fetchall(), [(0,)])

    def test_cinderbrew_missing_primaries_keep_native_zero_rows_and_goldie_faction(self):
        self.assert_source_primaries(2661, (210267, 214661), source_difficulty=0)
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual({p['entry']: p['source_difficulty_id'] for p in receipt['source_difficulties']}, {210267: 0, 214661: 0})
        self.assertEqual(receipt['donors'], [])
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=214661').fetchone(), (14,))
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                                        'WHERE map=2661 AND guid BETWEEN 63661000 AND 63661999 ORDER BY guid').fetchall(), [
                (63661000, 210267, 2760.08212038, -4935.52571057, 100.20407886059111),
                (63661001, 214661, 2649.0533460799998, -5013.14830145, 110.600006),
            ])
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2661_214661'])
        self.assertEqual(receipt['placements'][1]['source_floor_evidence']['wmo_group_id'], 69970)

    def test_taahbat_keeps_native_identity_existing_awazj_and_soul_scribe_scaling_gap(self):
        self.assert_source_primaries(2830, (234933,))
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual([g['name'] for g in receipt['source_gaps']], ['FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_234935'])
        self.assertEqual(receipt['donors'], [])
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT id1,map,position_x,position_y,position_z FROM content_creature WHERE guid=30006707').fetchone(),
                             (237514, 2830, 878.045, -614.084, 999.058))
            self.assertEqual(db.execute('SELECT id1,map,position_x,position_y,position_z FROM content_creature WHERE guid=63830000').fetchone(),
                             (234933, 2830, 860.9113228374999, -671.9940244898503, 998.2693509718046))
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2830_234933'])
        evidence = receipt['placements'][0]['source_floor_evidence']
        self.assertEqual(evidence['maximum_horizontal_distance_yards'], 12)
        self.assertEqual(evidence['corroboration']['entry'], 237514)

    def test_darkflame_missing_primaries_keep_native_one_and_zero_source_rows(self):
        self.assert_source_primaries(2651, (208743,))
        self.assert_source_primaries(2651, (208747, 210149), source_difficulty=0)
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual({p['entry']: p['source_difficulty_id'] for p in receipt['source_difficulties']},
                         {208743: 1, 208747: 0, 210149: 0})
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT Entry FROM tdb_creature_template_difficulty '
                                        'WHERE Entry IN(208747,210149) AND DifficultyID=1').fetchall(), [])
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2651 AND id1=212777').fetchone(), (0,))
            self.assertEqual(db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                                        'WHERE map=2651 AND guid BETWEEN 63651000 AND 63651999 ORDER BY guid').fetchall(), [
                (63651000, 208743, 406.77750715, -155.2714289, 56.68354978731017),
                (63651001, 208747, 188.47888114999995, -547.817170475, 59.8),
                (63651002, 210149, 541.8676539, 30.722493174999954, 86.600006),
            ])
        placements = {p['entry']: p for p in receipt['placements']}
        self.assertTrue(all(p['marker']['floor'] is None for p in placements.values()))
        self.assertEqual(placements[208743]['source_floor_evidence']['maximum_horizontal_distance_yards'], 8)
        self.assertEqual(placements[208747]['source_floor_evidence']['policy'], 'FOLLOWER_PLACEMENT_Z_FROM_SOURCE_FLOOR_UPWARD_CONSENSUS')
        self.assertEqual(placements[210149]['status'], 'UNIQUE_REACHABLE_SURFACE')
        self.assertEqual(set(receipt['navigation_gaps']), {'FOLLOWER_ROUTE_INCOMPLETE_2651_208743',
                                                         'FOLLOWER_ROUTE_INCOMPLETE_2651_208747'})

    def test_native_difficulty_chain_imports_source_zero_without_synthetic_normal_rows(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source:
            for tuning in (2718, 0):
                with self.subTest(tuning=tuning), closing(copy_native_actor_fixture(source, 210149)) as target:
                    target.execute('UPDATE tdb_creature_template_difficulty SET ContentTuningID=? WHERE Entry=210149', (tuning,))
                    if tuning == 0:
                        with self.assertRaisesRegex(ValueError, 'FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_210149'):
                            prepare_templates(target, [dict(entry=210149)], Path('unused'))
                        continue
                    donors, proof = prepare_templates(target, [dict(entry=210149)], Path('unused'))
                    self.assertEqual(donors, [])
                    self.assertEqual([(p['entry'], p['source_difficulty_id'], p['policy']) for p in proof],
                                     [(210149, 0, 'FOLLOWER_PLACEMENT_SOURCE_DIFFICULTY_CHAIN')])
                    self.assertEqual(target.execute('SELECT faction,unit_class,HealthModifier,ContentTuningID '
                                                    'FROM content_creature_template WHERE entry=210149').fetchone(),
                                     (16, 1, 25.0, 2718))
                    self.assertEqual(target.execute('SELECT DifficultyID FROM tdb_creature_template_difficulty WHERE Entry=210149').fetchall(), [(0,)])

    def test_ruby_erkhart_keeps_own_native_faction_model_and_budget(self):
        self.assert_source_primaries(2521, (190485,))
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        self.assertEqual(receipt['donors'], [])
        self.assertEqual([g['name'] for g in receipt['source_gaps']], ['FOLLOWER_PLACEMENT_NO_INDIVIDUAL_SOURCE_190484'])
        self.assertEqual(receipt['source_findings'][0]['entry'], 190485)
        self.assertGreater(receipt['source_findings'][0]['distance_yards'], 5)
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2521_190485'])
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT id1,map,position_x,position_y,position_z FROM content_creature '
                                        'WHERE guid=63521000').fetchone(),
                             (190485, 2521, 1773.8755744402401, 18.689396737716038, 313.12299999999993))
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2521 AND id1=190484').fetchone(), (0,))

    def test_nokhud_four_source_supported_primaries_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2516, (186151, 186338, 186615, 186616))

    def test_nokhud_three_supported_primaries_keep_exact_journal_poses_and_missing_cases_visible(self):
        self.assert_source_primaries(2516, (186151, 186338, 186616))
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            poses = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                               'WHERE map=2516 AND guid BETWEEN 63516000 AND 63516999 ORDER BY guid').fetchall()
            self.assertEqual(poses, [
                (63516000, 186151, -536.0531378044291, 4163.211154128592, 381.202345311977),
                (63516001, 186338, -2095.4968717149695, 4335.885373101182, 536.0469244723145),
                (63516004, 186616, -1167.5642312417303, 1356.6360801056499, 390.72347992372613),
            ])
        self.assertEqual({g['entry'] for g in receipt['source_gaps']}, {186339, 186615})
        self.assertEqual({g['name'] for g in receipt['source_gaps']},
                         {'FOLLOWER_PLACEMENT_NO_INDIVIDUAL_SOURCE_186339', 'FOLLOWER_PLACEMENT_NO_BAKED_SUPPORT_2516_186615'})
        self.assertEqual(set(receipt['navigation_gaps']), {'FOLLOWER_ROUTE_INCOMPLETE_2516_186151',
                         'FOLLOWER_ROUTE_INCOMPLETE_2516_186338', 'FOLLOWER_ROUTE_INCOMPLETE_2516_186616'})

    def test_halls_four_primary_bosses_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727, 189729))

    def test_halls_three_source_supported_primaries_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727))
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            poses = db.execute('SELECT guid,id1,position_x,position_y,position_z FROM content_creature '
                               'WHERE map=2527 AND guid BETWEEN 63527000 AND 63527999 ORDER BY guid').fetchall()
            self.assertEqual(poses, [
                (63527000, 189719, -2.162094289251968, -210.6418370698127, -35.49920222451337),
                (63527001, 189722, -353.12273242499987, -155.72340692500006, -67.90659723069882),
                (63527002, 189727, -315.9738499750001, 161.897570825, -100.40462215402596),
            ])
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2527 AND id1=189729').fetchone(), (0,))
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2527_189722',
                                                       'FOLLOWER_ROUTE_INCOMPLETE_2527_189727'])
        self.assertEqual([g['name'] for g in receipt['source_gaps']], ['FOLLOWER_PLACEMENT_FLOOR_UNRESOLVED_2527_189729'])

    def test_packet_sql_preserves_later_combat_owner_and_refuses_changed_pose(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        sql = (path.parent / 'delta.sql').read_text()
        placement = receipt['placements'][0]
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as replay:
            for table in ('content_creature_template', 'content_creature_template_model', 'content_creature', 'raid_encounter_creature'):
                replay.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
            replay.executescript(sql)
            replay.execute("UPDATE content_creature_template SET ScriptName='separate_combat_owner' WHERE entry=?", (placement['entry'],))
            replay.commit()
            replay.executescript(sql)
            self.assertEqual(replay.execute('SELECT ScriptName FROM content_creature_template WHERE entry=?',
                                            (placement['entry'],)).fetchone(), ('separate_combat_owner',))
            replay.execute('UPDATE content_creature SET position_x=position_x+1 WHERE guid=?', (placement['guid'],))
            replay.commit()
            with self.assertRaises(sqlite3.IntegrityError):
                replay.executescript(sql)
            replay.rollback()
            self.assertEqual(replay.execute('SELECT position_x FROM content_creature WHERE guid=?',
                                            (placement['guid'],)).fetchone(), (placement['wow_xyz'][0] + 1,))

    @unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ, 'requires authenticated journal CSVs')
    def test_lua_primary_identity_keeps_native_and_journal_displays_separate(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        root = Path(__file__).resolve().parents[2]
        inventory_path = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/df-bosses-2-2026-10-21/follower-source-inventory.json')
        inventory = json.loads(inventory_path.read_text())
        tables, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source:
            for manifest_name, entry, expected, native_display in [
                ('halls-of-infusion', 189722, (2527, 189722, 2616, 1, 2507, 5448, 103584), 110696),
                ('vexamus', 194181, (2526, 194181, 2562, 0, 2509, 5411, 107525), 109099),
            ]:
                manifest = json.loads((root / f'content/follower-placements/{manifest_name}.json').read_text())
                record = next(r for r in manifest['entries'] if r['entry'] == entry)
                identity = record['journal_actor_source']
                binding = derive_binding(source, inventory, tables, manifest['map_id'], entry, identity)
                self.assertEqual(binding, expected)
                self.assertEqual(source.execute('SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID=?',
                                                (entry,)).fetchall(), [(native_display,)])
                duplicated = copy.deepcopy(tables)
                actor = next(r for r in tables['JournalEncounterCreature'] if int(r['ID']) == expected[5])
                duplicated['JournalEncounterCreature'].append(dict(actor, ID='999999'))
                with self.assertRaisesRegex(ValueError, 'unique exact-name'):
                    derive_binding(source, inventory, duplicated, manifest['map_id'], entry, identity)
                with self.assertRaisesRegex(ValueError, 'evidence changed'):
                    derive_binding(source, inventory, tables, manifest['map_id'], entry, dict(identity, sha256='0' * 64))

    def test_taahbat_native_room_uses_only_its_twelve_yard_limit_and_separate_corroboration(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/ecodome-taahbat.json'
        manifest = json.loads(manifest_path.read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source:
            proofs = validate_native_room_anchors(source, manifest)
            self.assertEqual([p['entry'] for p in proofs], [234933])
            self.assertEqual({a['entry'] for a in proofs[0]['anchors']}, {210931, 234960})
            self.assertGreater(proofs[0]['anchors'][1]['horizontal_distance_yards'], 10.9)
            narrowed = copy.deepcopy(manifest)
            narrowed['entries'][0]['source_floor_evidence']['maximum_horizontal_distance_yards'] = 8
            with self.assertRaisesRegex(ValueError, 'do not agree'):
                validate_native_room_anchors(source, narrowed)
            corroboration = manifest['entries'][0]['source_floor_evidence']['corroboration']
            self.assertEqual(source.execute('SELECT id,map,position_x,position_y,position_z FROM tdb_creature '
                                            'WHERE guid=?', (corroboration['guid'],)).fetchone(),
                             (237514, 2830, *corroboration['wow_xyz']))
            self.assertLess(abs(corroboration['wow_xyz'][2] - proofs[0]['reference_floor_height']), 1)

    def test_blazikon_native_room_uses_only_its_explicit_eight_yard_limit(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/darkflame-cleft.json'
        manifest = json.loads(manifest_path.read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source:
            proofs = validate_native_room_anchors(source, manifest)
            self.assertEqual([p['entry'] for p in proofs], [208743])
            record = manifest['entries'][0]
            query = dict(height=56.68354978731017, exit_code=1, corners=[])
            self.assertEqual(select_surface_query(record, dict(floor=None), [query]), query)
            distances = [a['horizontal_distance_yards'] for a in proofs[0]['anchors']]
            self.assertLess(distances[0], 3)
            self.assertGreater(distances[1], 7)
            self.assertLess(distances[1], 8)
            narrowed = copy.deepcopy(manifest)
            narrowed['entries'][0]['source_floor_evidence']['maximum_horizontal_distance_yards'] = 5
            with self.assertRaisesRegex(ValueError, 'do not agree'):
                validate_native_room_anchors(source, narrowed)

    def test_native_room_policy_refuses_single_moved_or_ambiguous_anchors(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/vexamus.json'
        manifest = json.loads(manifest_path.read_text())
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as target:
            target.execute('CREATE TABLE tdb_creature(guid INTEGER,id INTEGER,map INTEGER,position_x REAL,position_y REAL,position_z REAL)')
            rows = source.execute('SELECT guid,id,map,position_x,position_y,position_z FROM tdb_creature '
                                  'WHERE guid IN(9001421,9001226)').fetchall()
            self.assertEqual(len(rows), 2)
            target.executemany('INSERT INTO tdb_creature VALUES(?,?,?,?,?,?)', rows)
            proof = validate_native_room_anchors(target, manifest)
            self.assertEqual(len(proof[0]['anchors']), 2)
            for anchor in proof[0]['anchors']:
                self.assertLess(anchor['horizontal_distance_yards'], 5)
                self.assertLess(anchor['reference_vertical_difference_yards'], 1)
            single = copy.deepcopy(manifest)
            single['entries'][0]['source_floor_evidence']['anchors'].pop()
            with self.assertRaisesRegex(ValueError, 'two distinct'):
                validate_native_room_anchors(target, single)
            target.execute('UPDATE tdb_creature SET position_z=807 WHERE guid=9001421')
            with self.assertRaisesRegex(ValueError, 'differs from recorded source'):
                validate_native_room_anchors(target, manifest)
            moved = copy.deepcopy(manifest)
            moved['entries'][0]['source_floor_evidence']['anchors'][0]['wow_xyz'][2] = 807
            with self.assertRaisesRegex(ValueError, 'do not agree'):
                validate_native_room_anchors(target, moved)
        record = manifest['entries'][0]
        selected = {'height': 904.0191540738892, 'exit_code': 1, 'corners': []}
        rejected = {'height': 807.1348686963983, 'exit_code': 1, 'corners': []}
        self.assertEqual(select_surface_query(record, {'floor': 0}, [selected, rejected]), selected)
        with self.assertRaisesRegex(ValueError, 'exactly one baked candidate'):
            select_surface_query(record, {'floor': 0}, [selected, rejected, dict(selected, height=904.1)])

    def test_academy_vexamus_has_its_own_sourced_primary_spawn(self):
        self.assert_source_primaries(2526, (194181,))

    def test_vexamus_keeps_the_native_encounter_room_floor_and_separate_route_gap(self):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        placement = next(p for p in receipt['placements'] if p['entry'] == 194181)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            pose = db.execute('SELECT position_x,position_y,position_z FROM content_creature '
                              'WHERE guid=63526000 AND id1=194181 AND map=2526').fetchone()
            self.assertIsNotNone(pose)
            self.assertEqual(pose, (1815.2512440319297, -2783.9755457624997, 904.0191540738892))
            evidence = placement['source_floor_evidence']
            self.assertEqual(evidence['policy'], 'FOLLOWER_PLACEMENT_Z_FROM_SAME_ROOM_NATIVE_ANCHORS')
            self.assertGreaterEqual(len(evidence['anchors']), 2)
            self.assertEqual({a['guid'] for a in evidence['anchors']}, {9001421, 9001226})
            for anchor in evidence['anchors']:
                native = db.execute('SELECT id,map,position_x,position_y,position_z FROM tdb_creature '
                                    'WHERE guid=?', (anchor['guid'],)).fetchone()
                self.assertEqual(native, (anchor['entry'], 2526, *anchor['wow_xyz']))
                self.assertLessEqual(math.hypot(pose[0] - native[2], pose[1] - native[3]), 5)
                self.assertLessEqual(abs(pose[2] - native[4]), 1)
                self.assertGreater(abs(807.1348686963983 - native[4]), 96)
        self.assertEqual(placement['navigation_gap'], 'FOLLOWER_ROUTE_INCOMPLETE_2526_194181')
        self.assertEqual(placement['status'], 'SOURCE_FLOOR_ROUTE_GAP')


if __name__ == '__main__':
    unittest.main()
