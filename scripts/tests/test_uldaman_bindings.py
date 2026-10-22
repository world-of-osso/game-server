"""Actual TDB/70448 copy; local Lua bindings cannot rewrite source actors."""
import sqlite3
from contextlib import closing
import tempfile
import unittest
from pathlib import Path

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class UldamanBindingsTests(unittest.TestCase):
    def test_lost_dwarves_source_rows_and_shared_credit_without_new_stats_or_spawns(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory)/'copy.sqlite')) as db:
                source.backup(db)
                population = db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall()
                stats = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN(184580,184581,184582)').fetchall()
                before = db.execute('SELECT * FROM content_creature_template WHERE entry IN(184580,184581,184582) ORDER BY entry').fetchall()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                script = columns.index('ScriptName')
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_lost_dwarves(db), [184580,184581,184582])
                    expected = [list(row) for row in before]
                    for row in expected:
                        row[script] = 'boss_lost_dwarves'
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry IN(184580,184581,184582) ORDER BY entry').fetchall(), [tuple(row) for row in expected])
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall(), population)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN(184580,184581,184582)').fetchall(), stats)
                    self.assertEqual(db.execute('SELECT creature_entry,encounter_id,bit,journal_creature_id FROM raid_encounter_creature WHERE map_id=2451 AND creature_entry IN(184580,184581,184582) ORDER BY creature_entry').fetchall(), [(184580,2555,0,5349),(184581,2555,0,5348),(184582,2555,0,5347)])
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=369791 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_lost_dwarves(db)

    def test_bromach_projects_only_native_totem_rows_and_primary_binding(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory)/'copy.sqlite')) as db:
                source.backup(db)
                parent = db.execute('SELECT * FROM content_creature_template WHERE entry=184018').fetchone()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                population = db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall()
                stats = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=184018').fetchall()
                other_bindings = db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry<>184018 ORDER BY map_id,creature_entry').fetchall()
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_bromach(db), [184018,186696])
                    expected = list(parent)
                    expected[columns.index('ScriptName')] = 'boss_bromach'
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=184018').fetchone(), tuple(expected))
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall(), population)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=184018').fetchall(), stats)
                    self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry<>184018 ORDER BY map_id,creature_entry').fetchall(), other_bindings)
                    self.assertEqual(db.execute('SELECT name,AIName,ScriptName,faction,unit_flags FROM content_creature_template WHERE entry=186696').fetchone(), ('Quaking Totem','','',16,0))
                    self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID,HealthModifier FROM content_creature_template_difficulty WHERE entry=186696 ORDER BY DifficultyID').fetchall(), [(1,2555,0.5),(2,2156,1.0),(23,2157,1.8)])
                    self.assertEqual(db.execute('SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=186696').fetchall(), [(65077,1.35)])
                    self.assertEqual(db.execute('SELECT auras FROM content_creature_template_addon WHERE entry=186696').fetchone(), ('388314',))
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2451 AND creature_entry=184018').fetchone(), (2556,1,2487,5357,107143))
                db.execute('UPDATE spell_effect SET EffectMiscValue_0=0 WHERE SpellID=382302 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_bromach(db)

    def test_talondras_binding_retains_runtime_model_and_native_source_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory)/'copy.sqlite')) as db:
                source.backup(db)
                before = db.execute('SELECT * FROM content_creature_template WHERE entry=184124').fetchone()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                models = db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=184124').fetchall()
                stats = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=184124').fetchall()
                population = db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall()
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_talondras(db), [184124])
                    expected = list(before)
                    expected[columns.index('ScriptName')] = 'boss_sentinel_talondras'
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=184124').fetchone(), tuple(expected))
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=184124').fetchall(), models)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry=184124').fetchall(), stats)
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall(), population)
                    self.assertEqual(db.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=184124').fetchall(), [(109735,)])
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2451 AND creature_entry=184124').fetchone(), (2557,2,2484,5356,106790))
                db.execute('UPDATE spell_effect SET ImplicitTarget_1=0 WHERE SpellID=372701 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_talondras(db)

    def test_emberon_binding_preserves_actor_native_channel_and_population(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory)/'copy.sqlite')) as db:
                source.backup(db)
                before = db.execute('SELECT * FROM content_creature_template WHERE entry=184422').fetchone()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                preserved_queries = [
                    'SELECT * FROM content_creature WHERE map=2451',
                    'SELECT * FROM content_creature_template_difficulty WHERE entry=184422',
                    'SELECT * FROM content_creature_template_model WHERE CreatureID=184422',
                    'SELECT * FROM content_creature_template_addon WHERE entry=184422',
                    'SELECT * FROM spell_misc WHERE SpellID=368990',
                    'SELECT * FROM spell_effect WHERE SpellID IN(368990,369025,395681,369061,369110)',
                    'SELECT * FROM raid_encounter_creature WHERE creature_entry<>184422 ORDER BY map_id,creature_entry',
                ]
                preserved = [db.execute(query).fetchall() for query in preserved_queries]
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_emberon(db), [184422])
                    expected = list(before)
                    expected[columns.index('ScriptName')] = 'boss_emberon'
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=184422').fetchone(), tuple(expected))
                    self.assertEqual([db.execute(query).fetchall() for query in preserved_queries], preserved)
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2451 AND creature_entry=184422').fetchone(), (2558,3,2476,5404,107816))
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=368990 AND EffectIndex=1')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_emberon(db)
