"""Single-Z preview identities added without replacing published map populations."""
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from follower_placement_sources import read_db2, read_surface_polygons
from follower_placements import derive_binding, derive_placement, read_binding_tables, write_placements

CASES = {
    'altar-coil': (2993, 259446, 63993001, 1249.6636067480806, 1949.646240579902, -46.48520722396362, 147556, 45.0),
    'voidscar-tazrah': (2923, 238887, 63923001, 4575.1869993, -454.1339994779998, -154.6508419821415, 124085, 31.5),
}


def manifest(label):
    return json.loads((ROOT / f'content/follower-placements/{label}.json').read_text())


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit current/native source fixtures')
class PreviewFollowupSource(unittest.TestCase):
    def test_add_boss_preserves_receipt_pinned_boss_and_trash_but_refuses_trash_drift(self):
        tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('MapDifficulty', 'MapChallengeMode'))
        bindings, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        tables.update(bindings)
        source_path = Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve(strict=True)
        with closing(sqlite3.connect(source_path.as_uri() + '?mode=ro', uri=True)) as source:
            for label, (map_id, entry, guid, x, y, z, preview, _) in CASES.items():
                with self.subTest(label=label), closing(sqlite3.connect(':memory:')) as target:
                    m = manifest(label)
                    record = m['entries'][0]
                    surfaces = read_surface_polygons(Path(m['surface_proof']['path']).with_suffix('.csv'))
                    point = derive_placement(record, m, tables, Path('/unused-cache'), surfaces, Path('/route-must-not-run'), Path('/unused-nav'), None)
                    binding = derive_binding(source, {}, tables, map_id, entry, record['journal_actor_source'])
                    self.assertEqual(point['wow_xyz'], [x, y, z])
                    self.assertEqual(binding[-1], preview)
                    for table, where in (('content_creature', f'guid BETWEEN {guid-1} AND {guid+998}'),
                                         ('raid_encounter_creature', f'map_id={map_id}')):
                        target.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
                        rows = source.execute(f'SELECT * FROM {table} WHERE {where}').fetchall()
                        target.executemany(f"INSERT INTO {table} VALUES({','.join('?' for _ in rows[0])})", rows)
                    before = target.execute('SELECT * FROM content_creature ORDER BY guid').fetchall()
                    write_placements(target, m, [point], [binding])
                    self.assertEqual(target.execute('SELECT * FROM content_creature WHERE guid<>? ORDER BY guid', (guid,)).fetchall(), before)
                    self.assertEqual(target.execute('SELECT id1,position_x,position_y,position_z FROM content_creature WHERE guid=?', (guid,)).fetchone(), (entry,x,y,z))
                    trash = json.loads(Path(m['preserved_placement_sources'][1]['receipt']).read_text())['placements'][0]['guid']
                    target.execute('UPDATE content_creature SET position_x=position_x+1 WHERE guid=?', (trash,))
                    with self.assertRaisesRegex(ValueError, 'preserved placement'):
                        write_placements(target, m, [point], [binding])


@unittest.skipUnless('PREVIEW_FOLLOWUP_DB' in os.environ, 'requires explicit source/prepared copy')
class PreviewFollowupAcceptance(unittest.TestCase):
    def test_native_pose_preview_and_all_published_population_survive(self):
        label = os.environ['PREVIEW_FOLLOWUP_LABEL']
        map_id, entry, guid, x, y, z, preview, health = CASES[label]
        path = Path(os.environ['PREVIEW_FOLLOWUP_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,position_x,position_y,position_z FROM content_creature WHERE map=? AND id1=?', (map_id,entry)).fetchall(), [(guid,x,y,z)])
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2805 AND guid BETWEEN 63805100 AND 63805999').fetchone(), (129,))
            self.assertEqual(db.execute('SELECT HealthModifier,faction,ContentTuningID FROM content_creature_template WHERE entry=?', (entry,)).fetchone(), (health,16,0))
            self.assertEqual(db.execute('SELECT t.unit_class,d.ContentTuningID,d.HealthModifier,d.DamageModifier,d.ArmorModifier,d.ManaModifier FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry WHERE t.entry=? AND d.DifficultyID=0', (entry,)).fetchone(),
                             db.execute('SELECT unit_class,ContentTuningID,HealthModifier,DamageModifier,ArmorModifier,ManaModifier FROM content_creature_template WHERE entry=?', (entry,)).fetchone())
            self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall(),
                             db.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx', (entry,)).fetchall())
            self.assertEqual(db.execute('SELECT display_id FROM raid_encounter_creature WHERE map_id=? AND creature_entry=?', (map_id,entry)).fetchone(), (preview,))
            for preserved in manifest(label)['preserved_placement_sources']:
                old_path = Path(preserved['receipt'])
                old = json.loads(old_path.read_text())
                with closing(sqlite3.connect((old_path.parent/'replayed.sqlite').as_uri()+'?mode=ro', uri=True)) as previous:
                    for point in old['placements']:
                        self.assertEqual(db.execute('SELECT * FROM content_creature WHERE guid=?',(point['guid'],)).fetchone(),
                                         previous.execute('SELECT * FROM content_creature WHERE guid=?',(point['guid'],)).fetchone())
        receipt = json.loads((path.parent/'receipt.json').read_text())
        self.assertEqual(receipt['sql_replay_count'],3)
        self.assertEqual(receipt['entrance_gaps'],[f'FOLLOWER_ENTRANCE_UNSOURCED_{map_id}'])
        self.assertEqual(receipt['navigation_gaps'],[])
        self.assertEqual(receipt['placements'][0]['all_surface_heights'],[z])
        self.assertTrue(receipt['placements'][0]['preview_identity']['display_mismatch'])
        self.assertEqual(len(receipt['changed_keys']['content_creature']),1)


if __name__ == '__main__':
    unittest.main()
