"""TC map difficulty scaling preserves native CT0 rows and their modifiers."""
from contextlib import closing
import copy
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import stage_source_difficulties, validate_native_room_anchors
from scripts.tests.test_followup_placements import copy_native_actor_fixture

POLICY = 'FOLLOWER_PLACEMENT_CONTENT_TUNING_FROM_NATIVE_MAP_DIFFICULTY'
CASES = ((207207, 2648, 5330, 2973), (207205, 2648, 5330, 2973),
         (211087, 2662, 5283, 2718), (211089, 2662, 5283, 2718),
         (213937, 2662, 5283, 2718), (226396, 2773, 5518, 2718),
         (234935, 2830, 5703, 2718))


@unittest.skipUnless('FOLLOWER_MAP_TUNING_DB' in os.environ, 'requires native source database')
class NativeMapTuning(unittest.TestCase):
    def open_source(self):
        path = Path(os.environ['FOLLOWER_MAP_TUNING_DB']).resolve(strict=True)
        return sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)

    def copy_actor(self, source, entry, map_id):
        target = copy_native_actor_fixture(source, entry)
        target.execute(source.execute("SELECT sql FROM sqlite_master WHERE name='map_difficulty'").fetchone()[0])
        rows = source.execute('SELECT * FROM map_difficulty WHERE MapID=?', (map_id,)).fetchall()
        target.executemany(f"INSERT INTO map_difficulty VALUES({','.join('?' for _ in rows[0])})", rows)
        target.execute('INSERT OR IGNORE INTO content_tuning_level VALUES(?,?,?,?,?)',
                       source.execute('SELECT * FROM content_tuning_level WHERE ID=2973').fetchone())
        return target

    def test_seven_native_zero_actors_select_map_tuning_without_rewriting_rows(self):
        with closing(self.open_source()) as source:
            for entry, map_id, row_id, tuning in CASES:
                with self.subTest(entry=entry), closing(self.copy_actor(source, entry, map_id)) as target:
                    native = target.execute('SELECT * FROM tdb_creature_template_difficulty ORDER BY Entry,DifficultyID').fetchall()
                    record = dict(entry=entry, map_id=map_id, content_tuning_policy=POLICY)
                    try:
                        proof = stage_source_difficulties(target, [record])[0]
                    except ValueError as error:
                        self.fail(f'Native map tuning is sourced, not missing: {error}')
                    self.assertEqual(proof['source_difficulty_id'], 0)
                    self.assertEqual(proof['source_row']['ContentTuningID'], 0)
                    self.assertEqual(proof['effective_content_tuning']['policy'], POLICY)
                    self.assertEqual(proof['effective_content_tuning']['map_difficulty_row']['ID'], row_id)
                    self.assertEqual(proof['effective_content_tuning']['content_tuning_id'], tuning)
                    self.assertEqual(target.execute('SELECT ContentTuningID FROM temp.tdb_difficulty').fetchone(), (0,))
                    self.assertEqual(target.execute('SELECT * FROM tdb_creature_template_difficulty ORDER BY Entry,DifficultyID').fetchall(), native)

    def test_swampface_room_uses_twenty_yards_without_moving_anchors(self):
        manifest = json.loads((Path(__file__).resolve().parents[2] /
                               'content/follower-placements/floodgate-swampface.json').read_text())
        with closing(self.open_source()) as source:
            proof = validate_native_room_anchors(source, manifest)[0]
            distances = [anchor['horizontal_distance_yards'] for anchor in proof['anchors']]
            self.assertAlmostEqual(distances[0], 13.340053445765932)
            self.assertAlmostEqual(distances[1], 19.021048674128014)
            narrower = copy.deepcopy(manifest)
            narrower['entries'][0]['source_floor_evidence']['maximum_horizontal_distance_yards'] = 12
            with self.assertRaisesRegex(ValueError, 'same-room native anchors do not agree'):
                validate_native_room_anchors(source, narrower)
            corroboration = manifest['entries'][0]['source_floor_evidence']['corroboration']
            self.assertEqual(source.execute('SELECT id,map,position_x,position_y,position_z FROM tdb_creature WHERE guid=?',
                                            (corroboration['guid'],)).fetchone(),
                             (231223, 2773, *corroboration['wow_xyz']))

    def test_missing_map_or_critter_never_acquires_map_tuning(self):
        with closing(self.open_source()) as source:
            for mutation in ('DELETE FROM map_difficulty',
                             'UPDATE map_difficulty SET ContentTuningID=0',
                             'UPDATE tdb_creature_template SET type=8'):
                with self.subTest(mutation=mutation), closing(self.copy_actor(source, 234935, 2830)) as target:
                    target.execute(mutation)
                    with self.assertRaisesRegex(ValueError, 'FOLLOWER_PLACEMENT_SCALING_SOURCE_MISSING_234935'):
                        stage_source_difficulties(target, [dict(entry=234935, map_id=2830, content_tuning_policy=POLICY)])


@unittest.skipUnless('FOLLOWER_MAP_PLACEMENT_DB' in os.environ, 'requires explicit placement database')
class RegionalPlacementAcceptance(unittest.TestCase):
    def test_anub_keeps_native_zero_tuning_and_explicit_terrain_project_selection(self):
        path = Path(os.environ['FOLLOWER_MAP_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            actual = db.execute('SELECT c.guid,t.faction,t.unit_class,t.ContentTuningID,t.HealthModifier,'
                                'c.position_x,c.position_y,c.position_z,m.CreatureDisplayID,r.display_id '
                                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                                'JOIN raid_encounter_creature r ON r.map_id=c.map AND r.creature_entry=c.id1 '
                                'WHERE c.map=2662 AND c.id1=211089').fetchall()
            self.assertEqual(actual, [(63662001, 16, 4, 0, 35.0, 1896.8120528718662, 987.4777805024102,
                                       124.99189880457607, 114315, 114315)], 'missing sourced Anub primary')
            self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID FROM tdb_creature_template_difficulty WHERE Entry=211089').fetchall(), [(0, 0)])
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        evidence = receipt['placements'][0]['source_floor_evidence']
        self.assertEqual(evidence['policy'], 'FOLLOWER_PLACEMENT_Z_FROM_AUTHORED_TERRAIN_TRIANGLE')
        self.assertEqual(evidence['selection_scope'], 'PROJECT_SELECTION_NOT_TC_SPAWN_Z')
        self.assertEqual(receipt['source_difficulties'][0]['effective_content_tuning']['map_difficulty_row']['ID'], 5283)
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2662_211089'])
        self.assertEqual(receipt['donors'][0]['donor_entry'], 188252)
        self.assertEqual({gap['entry'] for gap in receipt['source_gaps']}, {211087, 213937})

    def test_swampface_keeps_native_model_and_zero_ct_with_twenty_yard_room_provenance(self):
        path = Path(os.environ['FOLLOWER_MAP_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute('SELECT c.guid,t.faction,t.unit_class,t.ContentTuningID,t.HealthModifier,'
                              'c.position_x,c.position_y,c.position_z,m.CreatureDisplayID,r.display_id '
                              'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                              'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                              'JOIN raid_encounter_creature r ON r.map_id=c.map AND r.creature_entry=c.id1 '
                              'WHERE c.map=2773 AND c.id1=226396').fetchall()
            self.assertEqual(rows, [(63773000, 16, 4, 0, 35.0, 1593.7705868895773, -2745.6774048999923,
                                     335.20258, 124283, 115575)], 'missing native Swampface primary/model/pose')
            self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID FROM tdb_creature_template_difficulty WHERE Entry=226396').fetchall(), [(0, 0)])
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        tuning = receipt['source_difficulties'][0]['effective_content_tuning']
        self.assertEqual((tuning['map_difficulty_row']['ID'], tuning['content_tuning_id']), (5518, 2718))
        evidence = receipt['placements'][0]['source_floor_evidence']
        self.assertEqual([anchor['guid'] for anchor in evidence['anchors']], [10006487, 10006494])
        self.assertEqual(evidence['corroboration']['guid'], 10006489)
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2773_226396'])
        self.assertEqual(receipt['donors'][0]['donor_entry'], 226398)

    def test_gorren_keeps_native_zero_tuning_and_uses_sourced_map_and_floor(self):
        path = Path(os.environ['FOLLOWER_MAP_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute(
                'SELECT c.guid,t.faction,t.unit_class,t.ContentTuningID,t.HealthModifier,t.DamageModifier,'
                't.ArmorModifier,t.ManaModifier,c.position_x,c.position_y,c.position_z,r.display_id '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN raid_encounter_creature r ON r.creature_entry=c.id1 AND r.map_id=c.map '
                'WHERE c.map=2648 AND c.id1=207205').fetchall()
            self.assertEqual(len(rows), 1, 'missing sourced Stormguard Gorren primary')
            native = db.execute('SELECT HealthModifier,DamageModifier,ArmorModifier,ManaModifier '
                                'FROM tdb_creature_template_difficulty WHERE Entry=207205 AND DifficultyID=0').fetchone()
            self.assertEqual(rows[0][:4], (63648001, 16, 2, 0))
            self.assertEqual(rows[0][4:8], native)
            self.assertEqual(rows[0][8:], (2805.147910485, -1643.4466778225, 142.126491995114, 114403))
            self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID FROM tdb_creature_template_difficulty WHERE Entry=207205').fetchall(), [(0, 0)])
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        scaling = receipt['source_difficulties'][0]
        self.assertEqual(scaling['source_row']['ContentTuningID'], 0)
        self.assertEqual(scaling['effective_content_tuning']['map_difficulty_row']['ID'], 5330)
        self.assertEqual(scaling['effective_content_tuning']['content_tuning_id'], 2973)
        self.assertEqual(receipt['placements'][0]['source_floor_evidence']['wmo_group_id'], 69610)
        self.assertEqual(receipt['navigation_gaps'], ['FOLLOWER_ROUTE_INCOMPLETE_2648_207205'])
        self.assertEqual(receipt['donors'][0]['donor_entry'], 209230)


if __name__ == '__main__':
    unittest.main()
