"""Actual TDB/70448 copy; local Lua binding never rewrites flags or population."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class AcademyBindingsTests(unittest.TestCase):
    def test_ancient_binding_preserves_source_immunity_stats_and_population(self):
        with tempfile.TemporaryDirectory() as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True) as source, sqlite3.connect(Path(directory)/'copy.sqlite') as db:
                source.backup(db)
                before = db.execute('SELECT * FROM content_creature_template WHERE entry=196482').fetchone()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                script = columns.index('ScriptName')
                population = db.execute('SELECT * FROM content_creature WHERE map=2526').fetchall()
                stats = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=196482').fetchall()
                models = db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=196482').fetchall()
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_ancient(db), [196482])
                    after = db.execute('SELECT * FROM content_creature_template WHERE entry=196482').fetchone()
                    expected = list(before)
                    expected[script] = 'boss_overgrown_ancient'
                    self.assertEqual(after, tuple(expected))
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE map=2526').fetchall(), population)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=196482').fetchall(), stats)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=196482').fetchall(), models)
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2526 AND creature_entry=196482').fetchone(), (2563,1,2512,5420,109194))
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=388796')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_ancient(db)
