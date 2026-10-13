"""Real source-copy mirror-template admission; no canonical writes."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from encounter_summons import import_animated_gold_template, import_mirror_template, import_shadow_of_zul_template

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class MirrorTemplateTests(unittest.TestCase):
    def test_animated_gold_uses_actual_base_row_without_new_spawn_or_credit(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'gold.sqlite'
            with sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro',uri=True) as source, sqlite3.connect(copy) as db:
                source.backup(db)
                before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                source_models = db.execute('SELECT CreatureDisplayID,DisplayScale FROM tdb_creature_template_model WHERE CreatureID=135406').fetchall()
                db.execute('DELETE FROM content_creature_template WHERE entry=135406')
                db.execute('DELETE FROM content_creature_template_model WHERE CreatureID=135406')
                db.commit()
                import_animated_gold_template(db)
                self.assertEqual(db.execute('SELECT AIName,ScriptName,ContentTuningID,HealthModifier FROM content_creature_template WHERE entry=135406').fetchone(), ('','npc_animated_gold',0,0.45))
                self.assertEqual(db.execute('SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=135406').fetchall(),source_models)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0],before)
                self.assertEqual(db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=135406').fetchone()[0],0)
                import_animated_gold_template(db)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template WHERE entry=135406').fetchone()[0],1)

    def test_shadow_of_zul_uses_actual_base_row_without_new_spawn(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'zul.sqlite'
            with sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro',uri=True) as source, sqlite3.connect(copy) as db:
                source.backup(db)
                before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                source_models = db.execute('SELECT CreatureDisplayID,DisplayScale FROM tdb_creature_template_model WHERE CreatureID=137020').fetchall()
                db.execute('DELETE FROM content_creature_template WHERE entry=137020')
                db.execute('DELETE FROM content_creature_template_model WHERE CreatureID=137020')
                db.execute('DELETE FROM content_creature_template_addon WHERE entry=137020')
                db.commit()
                import_shadow_of_zul_template(db)
                # TDB AnimTier3 packs into bytes1 bits 24-31; auras carry Shadowform269058.
                self.assertEqual(db.execute('SELECT bytes1,bytes2,auras FROM content_creature_template_addon WHERE entry=137020').fetchone(), (3 << 24,1,'269058'))
                self.assertEqual(db.execute('SELECT name,ScriptName,faction FROM content_creature_template WHERE entry=137020').fetchone(), ('Shadow of Zul','npc_kings_rest_shadow_of_zul',16))
                self.assertEqual(db.execute('SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=137020').fetchall(),source_models)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0],before)
                import_shadow_of_zul_template(db)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template WHERE entry=137020').fetchone()[0],1)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template_addon WHERE entry=137020').fetchone()[0],1)

    def test_exact_source_template_models_and_unrelated_population_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'mirror.sqlite'
            with sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro',uri=True) as source, sqlite3.connect(copy) as db:
                source.backup(db)
                before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                unrelated = db.execute('SELECT * FROM content_creature_template WHERE entry=234648').fetchone()
                db.execute('DELETE FROM content_creature_template WHERE entry=255050')
                db.execute('DELETE FROM content_creature_template_model WHERE CreatureID=255050')
                db.execute('DELETE FROM content_creature_template_difficulty WHERE entry=255050')
                db.commit()
                import_mirror_template(db)
                self.assertEqual(db.execute('SELECT AIName,ScriptName FROM content_creature_template WHERE entry=255050').fetchone(), ('SmartAI',''))
                self.assertEqual(db.execute('SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=255050').fetchall(), [(138932,1.0)])
                self.assertEqual(db.execute('SELECT DifficultyID,ContentTuningID,HealthModifier FROM content_creature_template_difficulty WHERE entry=255050').fetchall(), [(1,3233,10.0)])
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0],before)
                self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=234648').fetchone(),unrelated)
                import_mirror_template(db)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature_template WHERE entry=255050').fetchone()[0],1)


if __name__ == '__main__':
    unittest.main()
