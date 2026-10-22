"""Actual Neltharus placement/route acceptance on an explicitly supplied DB copy.

FOLLOWER_PLACEMENT_DB must name a fresh canonical-patched copy. Route proof uses
FOLLOWER_PLACEMENT_ROUTE (navigation's follower_route executable) and the staged
FOLLOWER_PLACEMENT_NAV directory; neither synthetic geometry nor straight-line
reachability stands in for the server NavigationMap query.
"""
import math
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import json
import unittest
from contextlib import closing

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import write_placements


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB' in os.environ, 'requires an explicit prepared database copy')
class NeltharusPlacementAcceptance(unittest.TestCase):
    def test_all_four_primaries_keep_source_floor_templates_and_bindings(self):
        database = Path(os.environ['FOLLOWER_PLACEMENT_DB']).resolve(strict=True)
        expected_xy = {181861: (-211.1, -932.7), 189340: (-232.125, -674.9625),
                       189478: (-264.4580637024, -503.5), 189901: (-380.025, -772.8375)}
        with closing(sqlite3.connect(database.as_uri() + '?mode=ro', uri=True)) as db:
            self.assertEqual(db.execute('SELECT guid,id1 FROM content_creature WHERE guid BETWEEN 63519000 AND 63519999 ORDER BY guid').fetchall(),
                             [(63519000, 181861), (63519001, 189340), (63519002, 189478), (63519003, 189901)])
            rows = db.execute(
                'SELECT c.id1,c.guid,c.position_x,c.position_y,c.position_z,t.faction,t.unit_class,'
                's.unit_class,t.minlevel,t.maxlevel,t.ContentTuningID,t.HealthModifier,d.HealthModifier,'
                'r.display_id,m.CreatureDisplayID,c.Comment '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN tdb_creature_template s ON s.entry=c.id1 '
                'JOIN tdb_creature_template_difficulty d ON d.Entry=c.id1 AND d.DifficultyID=1 '
                'JOIN raid_encounter_creature r ON r.map_id=c.map AND r.creature_entry=c.id1 '
                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                'WHERE c.map=2519 AND c.id1 IN(181861,189340,189478,189901) ORDER BY c.id1'
            ).fetchall()
        self.assertEqual(len(rows), 4)
        self.assertEqual({row[0] for row in rows}, set(expected_xy))
        self.assertEqual([row[1] for row in rows], [63519000, 63519001, 63519002, 63519003])
        for row in rows:
            entry, guid, x, y, z, faction, unit_class, native_class, low, high, tuning, health, native_health, display, native_display, comment = row
            with self.subTest(entry=entry):
                self.assertAlmostEqual(x, expected_xy[entry][0], places=6)
                self.assertAlmostEqual(y, expected_xy[entry][1], places=6)
                self.assertEqual((faction, unit_class, low, high, tuning), (16, native_class, 70, 70, 2156))
                self.assertEqual(health, native_health)
                self.assertEqual(display, native_display)
                self.assertIn('FOLLOWER_PLACEMENT_WOWHEAD_UIMAP_COORDS', comment)
                self.assertIn(f'https://www.wowhead.com/npc={entry}', comment)
                if entry == 189478:
                    self.assertAlmostEqual(z, 125.19722023743995, places=6)

    def test_preparer_refuses_existing_same_key_with_changed_source_position(self):
        database = Path(os.environ['FOLLOWER_PLACEMENT_DB']).resolve(strict=True)
        packet = database.parent
        receipt = json.loads((packet / 'receipt.json').read_text())
        self.assertEqual(receipt['map_id'], 2519)
        manifest_path = Path(__file__).resolve().parents[2] / 'content/follower-placements/neltharus.json'
        manifest = json.loads(manifest_path.read_text())
        with closing(sqlite3.connect(database.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as target:
            for table in ('content_creature', 'raid_encounter_creature'):
                target.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
                where = 'map=2519 AND guid BETWEEN 63519000 AND 63519003' if table == 'content_creature' else 'map_id=2519'
                rows = source.execute(f'SELECT * FROM {table} WHERE {where}').fetchall()
                target.executemany(f"INSERT INTO {table} VALUES({','.join('?' for _ in rows[0])})", rows)
            target.execute('UPDATE content_creature SET position_x=position_x+1 WHERE guid=63519000')
            with self.assertRaisesRegex(ValueError, 'reserved placement GUID collision'):
                write_placements(target, manifest, receipt['placements'], [tuple(p['binding']) for p in receipt['placements']])

    def test_sql_reapplication_preserves_combat_owner_and_refuses_guid_collision(self):
        database = Path(os.environ['FOLLOWER_PLACEMENT_DB']).resolve(strict=True)
        sql = Path(os.environ['FOLLOWER_PLACEMENT_SQL']).read_text()
        with closing(sqlite3.connect(database.as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(':memory:')) as replay:
            for table in ('content_creature_template', 'content_creature_template_model', 'content_creature', 'raid_encounter_creature'):
                replay.execute(source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
            replay.executescript(sql)
            replay.execute("UPDATE content_creature_template SET ScriptName='separate_combat_owner' WHERE entry=189478")
            replay.commit()
            replay.executescript(sql)
            self.assertEqual(replay.execute('SELECT ScriptName FROM content_creature_template WHERE entry=189478').fetchone(), ('separate_combat_owner',))
            self.assertEqual(replay.execute('SELECT count(*) FROM content_creature').fetchone(), (4,))
            replay.execute('UPDATE content_creature SET id1=181861 WHERE guid=63519002')
            replay.commit()
            with self.assertRaises(sqlite3.IntegrityError):
                replay.executescript(sql)
            replay.rollback()
            self.assertEqual(replay.execute('SELECT id1 FROM content_creature WHERE guid=63519002').fetchone(), (181861,))

    def test_all_four_primary_bosses_have_normal_spawns_reachable_from_source_entrance(self):
        database = Path(os.environ["FOLLOWER_PLACEMENT_DB"]).resolve(strict=True)
        with closing(sqlite3.connect(database.as_uri() + "?mode=ro", uri=True)) as db:
            entrance = db.execute(
                "SELECT LocX,LocY,LocZ FROM tdb_world_safe_locs WHERE ID=100028 AND MapID=2519"
            ).fetchone()
            self.assertIsNotNone(entrance, "TDB1210 Neltharus entrance100028 missing")
            bosses = db.execute(
                "SELECT id1,position_x,position_y,position_z FROM content_creature "
                "WHERE map=2519 AND id1 IN(181861,189901,189478,189340) "
                "AND (','||replace(spawnDifficulties,' ','')||',') LIKE '%,1,%' ORDER BY id1,guid"
            ).fetchall()
        self.assertEqual(
            {row[0] for row in bosses}, {181861, 189901, 189478, 189340},
            "Neltharus missing one or more LittleWigs2777a9d/DB2 primary placements",
        )
        route = Path(os.environ["FOLLOWER_PLACEMENT_ROUTE"]).resolve(strict=True)
        nav = Path(os.environ["FOLLOWER_PLACEMENT_NAV"]).resolve(strict=True)
        start = [entrance[0], entrance[2], -entrance[1]]
        for entry, x, y, z in bosses:
            with self.subTest(entry=entry):
                self.assertTrue(all(math.isfinite(v) for v in (x, y, z)))
                end = [x, z, -y]
                result = subprocess.run(
                    [str(route), str(nav), *map(str, start), *map(str, end)],
                    capture_output=True, text=True, check=False, timeout=120,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                corners = [tuple(map(float, line.split(','))) for line in result.stdout.splitlines()]
                self.assertTrue(corners, "server navigation returned no corners")
                self.assertTrue(all(len(p) == 3 and all(math.isfinite(v) for v in p) for p in corners))
                # Same nominal nearest-polygon horizontal extent as NavigationMap.
                self.assertLessEqual(abs(corners[-1][0] - end[0]), 0.8)
                self.assertLessEqual(abs(corners[-1][2] - end[2]), 0.8)
                self.assertLessEqual(abs(corners[-1][1] - end[1]), 5.0)


if __name__ == "__main__":
    unittest.main()
