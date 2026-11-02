"""Follower equipment spell roots include Journal items, WCL gear and modifiers."""
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
from follower_summon_roots import gear_spell_roots


class FollowerSummonRootsTests(unittest.TestCase):
    def test_journal_wcl_set_enchant_and_gem_spells(self):
        with tempfile.TemporaryDirectory() as directory, sqlite3.connect(':memory:') as db:
            path = Path(directory)
            (path / 'JournalInstance.csv').write_text('ID,MapID\n1,34\n2,99\n')
            (path / 'JournalEncounter.csv').write_text('ID,JournalInstanceID\n10,1\n11,2\n')
            (path / 'JournalEncounterItem.csv').write_text(
                'JournalEncounterID,ItemID,DisplaySeasonID,WorldStateExpressionID\n'
                '10,101,0,0\n10,102,1,0\n11,103,0,0\n')
            wcl = path / 'wcl.json'
            wcl.write_text(json.dumps({'profiles': {'spec': {'gear': [
                {'id': 104, 'gems': [{'id': '105'}], 'permanentEnchant': '201',
                 'temporaryEnchant': '202', 'bonusIDs': ['601']}]}}}))
            db.executescript('''
                CREATE TABLE follower_lfg_dungeons(map_id,difficulty_id);
                INSERT INTO follower_lfg_dungeons VALUES (34,205);
                CREATE TABLE item_sparse(ID,ItemSet,Gem_properties);
                INSERT INTO item_sparse VALUES (101,301,0),(104,0,0),(105,0,401);
                CREATE TABLE item_x_item_effect(ItemID,ItemEffectID);
                INSERT INTO item_x_item_effect VALUES (101,1),(104,2),(102,3),(103,4);
                CREATE TABLE item_effect(ID,SpellID);
                INSERT INTO item_effect VALUES (1,501),(2,502),(3,503),(4,504),(5,509);
                CREATE TABLE item_bonus(ParentItemBonusListID,Type,Value_0);
                INSERT INTO item_bonus VALUES (601,23,5),(602,23,3);
                CREATE TABLE item_set_spell(ItemSetID,SpellID);
                INSERT INTO item_set_spell VALUES (301,505);
                CREATE TABLE gem_properties(ID,Enchant_ID);
                INSERT INTO gem_properties VALUES (401,203);
                CREATE TABLE spell_item_enchantment(ID,Effect_0,EffectArg_0,Effect_1,EffectArg_1,Effect_2,EffectArg_2);
                INSERT INTO spell_item_enchantment VALUES
                  (201,3,506,0,0,0,0),(202,0,0,3,507,0,0),(203,0,0,0,0,3,508);
            ''')
            self.assertEqual(gear_spell_roots(db, path, wcl), {501, 502, 505, 506, 507, 508, 509})


if __name__ == '__main__':
    unittest.main()
