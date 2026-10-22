"""Real six-actor Brackenhide primary census, using an explicitly supplied DB copy."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import unittest

ENTRIES = (186116, 186120, 186121, 186122, 186124, 186125)


@unittest.skipUnless('BRACKENHIDE_PLACEMENT_DB' in os.environ, 'requires an explicit prepared database copy')
class BrackenhidePlacementAcceptance(unittest.TestCase):
    def test_six_boss_actors_have_native_stats_models_and_primary_bindings(self):
        path = Path(os.environ['BRACKENHIDE_PLACEMENT_DB']).resolve(strict=True)
        with closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
            rows = db.execute(
                'SELECT c.id1,t.faction,t.unit_class,s.unit_class,t.HealthModifier,d.HealthModifier,'
                't.ContentTuningID,d.ContentTuningID,r.display_id,m.CreatureDisplayID '
                'FROM content_creature c JOIN content_creature_template t ON t.entry=c.id1 '
                'JOIN tdb_creature_template s ON s.entry=c.id1 '
                'JOIN tdb_creature_template_difficulty d ON d.Entry=c.id1 AND d.DifficultyID=1 '
                'JOIN raid_encounter_creature r ON r.creature_entry=c.id1 AND r.map_id=c.map '
                'JOIN content_creature_template_model m ON m.CreatureID=c.id1 '
                'WHERE c.map=2520 AND c.id1 IN(186116,186120,186121,186122,186124,186125) ORDER BY c.id1,c.guid'
            ).fetchall()
            self.assertEqual([r[0] for r in rows], list(ENTRIES), 'six Brackenhide boss actors lack prepared placements')
            for entry, faction, actual_class, native_class, health, native_health, tuning, native_tuning, display, native_display in rows:
                with self.subTest(entry=entry):
                    self.assertEqual(faction, 16)
                    self.assertEqual(actual_class, native_class)
                    self.assertEqual(health, native_health)
                    self.assertEqual(tuning, native_tuning)
                    self.assertEqual(display, native_display)
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE guid BETWEEN 63520000 AND 63520999').fetchone(), (6,))


if __name__ == '__main__':
    unittest.main()
