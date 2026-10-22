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
from follower_placements import derive_binding, read_binding_tables, select_surface_query, validate_native_room_anchors


@unittest.skipUnless('FOLLOWUP_PLACEMENT_DB' in os.environ, 'requires an explicit prepared database copy')
class FollowupPlacementAcceptance(unittest.TestCase):
    def assert_source_primaries(self, map_id, entries):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        marks = ','.join('?' for _ in entries)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute(
                'SELECT c.id1,t.faction,t.unit_class,s.unit_class,t.HealthModifier,d.HealthModifier,'
                't.ContentTuningID,d.ContentTuningID,r.display_id,m.CreatureDisplayID,n.CreatureDisplayID '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN tdb_creature_template s ON s.entry=c.id1 '
                'JOIN tdb_creature_template_difficulty d ON d.Entry=c.id1 AND d.DifficultyID=1 '
                'JOIN raid_encounter_creature r ON r.creature_entry=c.id1 AND r.map_id=c.map '
                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                'JOIN tdb_creature_template_model n ON n.CreatureID=c.id1 AND n.Idx=m.Idx '
                f'WHERE c.map=? AND c.id1 IN({marks}) ORDER BY c.id1,c.guid', (map_id, *entries)
            ).fetchall()
        self.assertEqual([r[0] for r in rows], list(entries), f'map{map_id} missing sourced primary placements')
        for entry, faction, actual_class, native_class, health, native_health, tuning, native_tuning, display, runtime_display, native_display in rows:
            with self.subTest(entry=entry):
                self.assertEqual(faction, 16)
                self.assertEqual(actual_class, native_class)
                self.assertEqual(health, native_health)
                self.assertEqual(tuning, native_tuning)
                self.assertEqual(runtime_display, native_display)
                self.assertEqual(display, {189719: 106801, 189722: 103584, 189727: 107064,
                                           189729: 106934, 194181: 107525}[entry])

    def test_halls_four_primary_bosses_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727, 189729))

    def test_halls_three_source_supported_primaries_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727))

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
