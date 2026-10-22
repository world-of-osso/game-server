"""Exact Midnight identities and source placements without invented admission routes."""
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import derive_binding, derive_placement, read_binding_tables
from follower_placement_sources import read_db2

ROOT = Path(__file__).resolve().parents[2]
CASES = ((2811, 231864, 2660, 63811000, 11432.141841355158, -5034.01362900625, 91.79095517557616),
         (2859, 244887, 2770, 63859000, 1239.978471470763, 1739.5917083368204, 72.49153223814916),
         (2923, 239008, 2792, 63923000, 4800.443040567881, -452.21876251249995, -62.17750391486241),
         (2993, 259445, 2878, 63993000, 1537.57051953125, 2117.8805041961014, -78.38069412571085))


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit native/database fixtures')
class MidnightSourceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.path = Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve(strict=True)
        cls.db2 = Path(os.environ['FOLLOWER_PLACEMENT_DB2'])
        cls.tables, _ = read_db2(cls.db2)
        binding, _ = read_binding_tables(cls.db2)
        cls.tables.update(binding)

    def test_four_exact_unique_single_surface_actors_are_present(self):
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as db:
            for map_id, entry, _, _, _, _, _ in CASES:
                with self.subTest(map_id=map_id, entry=entry):
                    count = db.execute('SELECT count(*) FROM content_creature WHERE map=? AND id1=?', (map_id, entry)).fetchone()[0]
                    self.assertEqual(count, 1, 'missing exact-identity Midnight single-surface primary')

    def test_exact_native_identity_does_not_require_empty_lua_registrations(self):
        identity = dict(policy='FOLLOWER_BINDING_EXACT_NATIVE_NAME_DISPLAY_TO_JOURNAL_ACTOR',
                        journal_encounter_id=2660, journal_actor_id=5982)
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as source:
            result = derive_binding(source, {}, self.tables, 2811, 231864, identity)
        self.assertEqual(result[:3], (2811, 231864, 3073))
        self.assertEqual(result[4:], (2660, 5982, 131317))
        changed = json.loads(json.dumps(self.tables))
        next(a for a in changed['JournalEncounterCreature'] if a['ID'] == '5982')['Name_lang'] = 'Not Gemellus'
        with closing(sqlite3.connect(self.path.as_uri() + '?mode=ro', uri=True)) as source:
            with self.assertRaisesRegex(ValueError, 'exact native name and display'):
                derive_binding(source, {}, changed, 2811, 231864, identity)

    def test_one_source_surface_without_entrance_does_not_query_a_route(self):
        manifest = dict(map_id=2811, placement_policy='FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP')
        record = dict(entry=231864, journal_encounter_id=2660)
        journal = next(r for r in self.tables['JournalEncounter'] if r['ID'] == '2660')
        from follower_placement_sources import read_journal_marker, transform_marker
        _, _, assignments = read_journal_marker(self.tables, 2811, 231864, 2660)
        x, y = transform_marker(dict(percent=[float(journal['Map_0']) * 100, float(journal['Map_1']) * 100]), assignments)
        z = 91.79095517557616
        polygon = [[(x - 1, z, -y - 1), (x + 1, z, -y - 1), (x + 1, z, -y + 1), (x - 1, z, -y + 1)]]
        result = derive_placement(record, manifest, self.tables, Path('unused-cache'), polygon,
                                  Path('/this-route-program-must-not-be-run'), Path('unused-nav'), None)
        self.assertEqual(result['wow_xyz'], [x, y, z])
        self.assertEqual(result['status'], 'SOURCE_PLACEMENT_ENTRANCE_UNSOURCED')
        self.assertEqual(result['entrance_gap'], 'FOLLOWER_ENTRANCE_UNSOURCED_2811')
        self.assertEqual(result['surface_queries'][0]['status'], 'NOT_QUERIED_ENTRANCE_UNSOURCED')
        self.assertIsNone(result['surface_queries'][0]['exit_code'])
        self.assertNotIn('navigation_gap', result)


@unittest.skipUnless('MIDNIGHT_PLACEMENT_DB' in os.environ, 'requires prepared Midnight packet')
class MidnightPlacementAcceptance(unittest.TestCase):
    def test_single_actor_packet_preserves_native_zero_ct_and_explicit_entrance_gap(self):
        path = Path(os.environ['MIDNIGHT_PLACEMENT_DB']).resolve(strict=True)
        receipt = json.loads((path.parent / 'receipt.json').read_text())
        map_id = receipt['map_id']
        case = next(c for c in CASES if c[0] == map_id)
        _, entry, journal_id, guid, x, y, z = case
        self.assertIsNone(receipt['entrance'])
        self.assertEqual(receipt['entrance_gaps'], [f'FOLLOWER_ENTRANCE_UNSOURCED_{map_id}'])
        self.assertEqual(receipt['navigation_gaps'], [])
        self.assertEqual(receipt['placements'][0]['status'], 'SOURCE_PLACEMENT_ENTRANCE_UNSOURCED')
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            actual = db.execute('SELECT c.guid,t.name,t.faction,t.unit_class,t.ContentTuningID,t.HealthModifier,'
                                't.DamageModifier,t.ArmorModifier,t.ManaModifier,c.position_x,c.position_y,c.position_z '
                                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                                'WHERE c.map=? AND c.id1=?', (map_id, entry)).fetchall()
            native = db.execute('SELECT t.name,t.unit_class,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier '
                                'FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry '
                                'WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone()
            self.assertEqual(len(actual), 1)
            self.assertEqual(actual[0][:6], (guid, native[0], 16, native[1], 0, native[2]))
            self.assertEqual(actual[0][6:9], native[3:])
            self.assertEqual(actual[0][9:], (x, y, z))
            self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID FROM tdb_creature_template_difficulty WHERE Entry=?', (entry,)).fetchall(), [(0, 0)])
            self.assertEqual(db.execute('SELECT journal_encounter_id FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?', (map_id, entry)).fetchone(), (journal_id,))
            self.assertEqual(db.execute('SELECT m.CreatureDisplayID,n.CreatureDisplayID FROM content_creature_template_model m '
                                        'JOIN tdb_creature_template_model n ON n.CreatureID=m.CreatureID AND n.Idx=m.Idx '
                                        'WHERE m.CreatureID=?', (entry,)).fetchall(),
                             [(receipt['placements'][0]['binding'][-1], receipt['placements'][0]['binding'][-1])])
        proof = receipt['source_difficulties'][0]
        self.assertEqual(proof['source_difficulty_id'], 0)
        self.assertEqual(proof['source_row']['ContentTuningID'], 0)
        self.assertEqual(proof['effective_content_tuning']['map_difficulty_row']['MapID'], map_id)
        self.assertEqual(receipt['sql_replay_count'], 3)
        if entry in (231864, 239008, 259445):
            proof = receipt['donors'][0]['hostility_source']
            self.assertEqual(proof['policy'], 'FOLLOWER_BOSS_HOSTILE_FROM_JOURNAL_ENCOUNTER')
            self.assertEqual(proof['binding'][0:2], [map_id, entry])
            self.assertEqual(int(proof['rows']['JournalEncounter']['row']['ID']), journal_id)
            self.assertEqual(proof['rows']['MapDifficulty']['row']['DifficultyID'], '205')
            self.assertEqual(int(proof['rows']['MapDifficulty']['row']['MapID']), map_id)
            self.assertEqual(len(receipt['placements'][0]['all_surface_heights']), 1)


if __name__ == '__main__':
    unittest.main()
