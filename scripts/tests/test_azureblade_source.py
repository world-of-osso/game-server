"""Actual TDB/70448 Azureblade producer, isolated disk snapshot only."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from encounter_summons import import_azureblade

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'


class AzurebladeSourceTests(unittest.TestCase):
    def test_native_image_rows_local_binding_and_idempotence(self):
        fixtures = ROOT / 'target/encounter-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'azure.sqlite') as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                before_boss = dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=186739').fetchone()))
                before_population = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                before_bindings = [tuple(row) for row in db.execute('SELECT * FROM raid_encounter_creature ORDER BY map_id,creature_entry')]
                self.assertEqual(import_azureblade(db), [186739, 190187])
                expected = dict(before_boss, ScriptName='boss_azureblade')
                self.assertEqual(dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=186739').fetchone())), expected)
                difficulties = [tuple(row) for row in db.execute('SELECT DifficultyID,ContentTuningID,HealthModifier FROM content_creature_template_difficulty WHERE entry=190187 ORDER BY DifficultyID')]
                self.assertEqual(difficulties, [(1, 2156, 1.0), (23, 2157, 1.0)])
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template_difficulty WHERE entry=190187 AND DifficultyID=205').fetchone()[0], 0)
                self.assertEqual(tuple(db.execute('SELECT AIName,ScriptName FROM content_creature_template WHERE entry=190187').fetchone()), ('', ''))
                actual_models = [tuple(row) for row in db.execute('SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=190187 ORDER BY CreatureDisplayID')]
                source_models = [tuple(row) for row in db.execute('SELECT CreatureDisplayID,DisplayScale FROM tdb_creature_template_model WHERE CreatureID=190187 ORDER BY CreatureDisplayID')]
                self.assertEqual(actual_models, source_models)
                self.assertTrue(actual_models)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], before_population)
                self.assertEqual([tuple(row) for row in db.execute('SELECT * FROM raid_encounter_creature ORDER BY map_id,creature_entry')], before_bindings)
                import_azureblade(db)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template WHERE entry=190187').fetchone()[0], 1)
                db.execute("UPDATE tdb_creature_template SET ScriptName='conflicting_source_ai' WHERE entry=186739")
                with self.assertRaisesRegex(ValueError, 'source identity changed'):
                    import_azureblade(db)


if __name__ == '__main__':
    unittest.main()
