"""Pinned previous-packet ownership before any density cleanup can delete rows."""
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import make_sql
from follower_trash_ownership import load_owned_spawns

D = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-placements-2026-10-22')
TABLES = ('content_creature_template', 'content_creature_template_model', 'content_creature', 'raid_encounter_creature')


def make_reference(directory):
    reference_path = Path(os.environ['FOLLOWER_OWNERSHIP_REFERENCE']).resolve(strict=True)
    original = json.loads(reference_path.read_text())
    placements = original['placements'][:2]
    replay = sqlite3.connect(directory / 'replayed.sqlite')
    empty = sqlite3.connect(':memory:')
    with closing(sqlite3.connect((reference_path.parent / 'replayed.sqlite').as_uri() + '?mode=ro', uri=True)) as source:
        for table in TABLES:
            schema = source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0]
            replay.execute(schema)
            empty.execute(schema)
        rows = source.execute('SELECT * FROM content_creature WHERE guid IN(63859100,63859101) ORDER BY guid').fetchall()
        replay.executemany(f"INSERT INTO content_creature VALUES({','.join('?' for _ in rows[0])})", rows)
    replay.commit()
    sql, _ = make_sql(empty, replay)
    (directory / 'delta.sql').write_text(sql)
    receipt = dict(map_id=2859, policy='FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS', guid_range=[63859100, 63859999],
                   placements=placements, sql_sha256=hashlib.sha256((directory / 'delta.sql').read_bytes()).hexdigest(),
                   replayed_sha256=hashlib.sha256((directory / 'replayed.sqlite').read_bytes()).hexdigest())
    (directory / 'receipt.json').write_text(json.dumps(receipt))
    evidence = dict(receipt=str(directory / 'receipt.json'), receipt_sha256=hashlib.sha256((directory / 'receipt.json').read_bytes()).hexdigest())
    baseline = sqlite3.connect(':memory:')
    replay.backup(baseline)
    replay.close()
    empty.close()
    return baseline, placements, evidence


@unittest.skipUnless('FOLLOWER_OWNERSHIP_REFERENCE' in os.environ, 'requires explicit published ownership reference')
class TrashOwnership(unittest.TestCase):
    def test_exact_owned_namespace_and_source_footprint_are_proved(self):
        with tempfile.TemporaryDirectory(prefix='ownership-', dir=D) as temp:
            baseline, points, evidence = make_reference(Path(temp))
            with closing(baseline):
                owned, proof = load_owned_spawns(baseline, 2859, points, evidence)
                self.assertEqual({row['guid'] for row in owned.values()}, {63859100, 63859101})
                self.assertEqual(proof['owned_spawn_count'], 2)
                self.assertEqual(proof['guid_range'], [63859100, 63859999])

    def test_reference_drift_foreign_map_unowned_row_or_changed_source_refuse(self):
        for mutation in ('receipt_hash', 'pose', 'unowned', 'source_pose', 'source_missing', 'map'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory(prefix='ownership-', dir=D) as temp:
                baseline, points, evidence = make_reference(Path(temp))
                with closing(baseline):
                    map_id = 2859
                    if mutation == 'receipt_hash':
                        evidence['receipt_sha256'] = '0' * 64
                    elif mutation == 'pose':
                        baseline.execute('UPDATE content_creature SET position_x=position_x+1 WHERE guid=63859100')
                    elif mutation == 'unowned':
                        baseline.execute('INSERT INTO content_creature(guid,id1,map) VALUES(63859999,244887,2859)')
                    elif mutation == 'source_pose':
                        points = copy.deepcopy(points)
                        points[0]['wow_xyz'][0] += 1
                    elif mutation == 'source_missing':
                        points = points[:1]
                    else:
                        map_id = 2923
                    with self.assertRaises(ValueError):
                        load_owned_spawns(baseline, map_id, points, evidence)


if __name__ == '__main__':
    unittest.main()
