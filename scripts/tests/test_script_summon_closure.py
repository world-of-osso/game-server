"""Behavioral coverage for declared script children and native source expansion."""
import sqlite3
import sys
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import retail_content


class ScriptSummonClosureTests(unittest.TestCase):
    def test_declared_child_trigger_and_enchant_reach_native_summons(self):
        with sqlite3.connect(':memory:') as db:
            db.executescript('''
                CREATE TABLE skill_line(ID,DisplayName_lang,CategoryID);
                CREATE TABLE chr_classes(ID,Name_lang);
                CREATE TABLE skill_line_ability(Spell,SkillLine,AcquireMethod,ClassMask);
                CREATE TABLE chr_specialization(ID,ClassID,OrderIndex,MasterySpellID_0,MasterySpellID_1);
                CREATE TABLE specialization_spells(SpellID,SpecID);
                CREATE TABLE trait_definition(ID,SpellID);
                CREATE TABLE trait_node_entry(ID,TraitDefinitionID);
                CREATE TABLE trait_node_x_trait_node_entry(TraitNodeEntryID,TraitNodeID);
                CREATE TABLE trait_node(ID,TraitTreeID);
                CREATE TABLE skill_line_x_trait_tree(TraitTreeID,SkillLineID);
                CREATE TABLE spell_effect(SpellID,DifficultyID,Effect,EffectTriggerSpell,EffectMiscValue_0);
                CREATE TABLE spell_item_enchantment(ID,Effect_0,EffectArg_0,Effect_1,EffectArg_1,Effect_2,EffectArg_2);
                CREATE TABLE tdb_creature_template(entry);
                INSERT INTO spell_effect VALUES
                    (434587,0,28,0,9001),
                    (1243659,0,6,9000001,0),
                    (9000001,0,28,0,9002),
                    (334294,0,54,0,5400),
                    (319778,0,28,0,9003),
                    (9000002,0,28,0,9004);
                INSERT INTO spell_item_enchantment VALUES(5400,3,319778,0,0,0,0);
            ''')
            reached = {row[0] for row in db.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertEqual(reached, {9001, 9002, 9003})
            with patch.object(retail_content, 'gear_spell_roots', return_value={9000002}):
                gear_query = retail_content.follower_class_summon_entries(db)
            self.assertEqual({row[0] for row in db.execute(gear_query)}, {9001, 9002, 9003, 9004})


if __name__ == '__main__':
    unittest.main()
