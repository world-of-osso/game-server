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
from follower_placements import select_surface_query, validate_native_room_anchors


@unittest.skipUnless('FOLLOWUP_PLACEMENT_DB' in os.environ, 'requires an explicit prepared database copy')
class FollowupPlacementAcceptance(unittest.TestCase):
    def assert_source_primaries(self, map_id, entries):
        path = Path(os.environ['FOLLOWUP_PLACEMENT_DB']).resolve(strict=True)
        marks = ','.join('?' for _ in entries)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute(
                'SELECT c.id1,t.faction,t.unit_class,s.unit_class,t.HealthModifier,d.HealthModifier,'
                't.ContentTuningID,d.ContentTuningID,r.display_id,m.CreatureDisplayID '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN tdb_creature_template s ON s.entry=c.id1 '
                'JOIN tdb_creature_template_difficulty d ON d.Entry=c.id1 AND d.DifficultyID=1 '
                'JOIN raid_encounter_creature r ON r.creature_entry=c.id1 AND r.map_id=c.map '
                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                f'WHERE c.map=? AND c.id1 IN({marks}) ORDER BY c.id1,c.guid', (map_id, *entries)
            ).fetchall()
        self.assertEqual([r[0] for r in rows], list(entries), f'map{map_id} missing sourced primary placements')
        for entry, faction, actual_class, native_class, health, native_health, tuning, native_tuning, display, native_display in rows:
            with self.subTest(entry=entry):
                self.assertEqual(faction, 16)
                self.assertEqual(actual_class, native_class)
                self.assertEqual(health, native_health)
                self.assertEqual(tuning, native_tuning)
                self.assertEqual(display, native_display)

    def test_halls_four_primary_bosses_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727, 189729))

    def test_halls_three_source_supported_primaries_keep_own_native_identity_and_budget(self):
        self.assert_source_primaries(2527, (189719, 189722, 189727))

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
                self.assertLess(anchor['vertical_difference_yards'], 1)
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
        selected = {'height': 904.0077893316047, 'exit_code': 1, 'corners': []}
        rejected = {'height': 807.1927971673523, 'exit_code': 1, 'corners': []}
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
            self.assertEqual(pose, (1815.58629572, -2783.3011699999997, 904.0077893316047))
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
                self.assertGreater(abs(807.1927971673523 - native[4]), 96)
        self.assertEqual(placement['navigation_gap'], 'FOLLOWER_ROUTE_INCOMPLETE_2526_194181')
        self.assertEqual(placement['status'], 'SOURCE_FLOOR_ROUTE_GAP')


if __name__ == '__main__':
    unittest.main()
