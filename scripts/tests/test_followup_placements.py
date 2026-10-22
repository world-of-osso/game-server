"""Sourced primaries for Halls of Infusion and the absent Academy Vexamus."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import unittest


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

    def test_academy_vexamus_has_its_own_sourced_primary_spawn(self):
        self.assert_source_primaries(2526, (194181,))


if __name__ == '__main__':
    unittest.main()
