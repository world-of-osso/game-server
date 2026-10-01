"""Behavioral tests for read-only modern candidate item projection."""

from pathlib import Path
import sqlite3
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import economy_crafting


class CraftingItemsTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.executescript((SCRIPTS / "crafting_schema.sql").read_text())
        columns = ",".join(f"Reagent_{i} INTEGER DEFAULT 0" for i in range(8))
        self.db.executescript(f"""
            CREATE TABLE spell_effect(SpellID INTEGER,Effect INTEGER,
                DifficultyID INTEGER,EffectMiscValue_0 INTEGER,EffectItemType INTEGER);
            CREATE TABLE spell_reagents(SpellID INTEGER,{columns});
            CREATE TABLE item_sparse(ID INTEGER,ModifiedCraftingReagentItemID INTEGER);
            INSERT INTO spell_effect VALUES (1200,288,0,500,9999),(1201,288,1,501,0),
                (1202,24,0,502,2302),(1203,288,0,503,0),(1204,288,0,504,0),
                (1205,288,0,505,0),(1206,288,0,506,0);
            INSERT INTO crafting_data VALUES
                (500,0,0,900,0,0,0,0,0,0,0,0,0,0,0,0),
                (501,0,0,901,0,0,0,0,0,0,0,0,0,0,0,0),
                (502,0,0,902,0,0,0,0,0,0,0,0,0,0,0,0),
                (503,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0),
                (504,1,0,904,0,0,0,0,0,0,0,0,0,0,0,0),
                (505,0,0,905,0,0,0,0,0,0,0,0,0,0,0,77);
            INSERT INTO crafting_data_item_quality VALUES (1,903,503);
            INSERT INTO spell_reagents(SpellID,Reagent_0,Reagent_1) VALUES
                (1200,2318,2320),(1201,998,0),(1202,997,0);
            INSERT INTO modified_crafting_spell_slot VALUES (15388,1200,0,421,0,140,80);
            INSERT INTO modified_crafting_reagent_slot VALUES ('Basic',421,0,0,0,0,0,0);
            INSERT INTO modified_crafting_category VALUES (50,'Basic','',0,0,0);
            INSERT INTO mcr_slot_x_mcr_category VALUES (1,50,0,421);
            INSERT INTO modified_crafting_reagent_item VALUES
                (700,'',50,0,0,0,0),(701,'',51,0,0,0,0);
            INSERT INTO item_sparse VALUES (777,700),(778,700),(779,701);
        """)

    def test_actual_ids_without_inferred_outputs_or_unrelated_alternatives(self):
        for factory in (None, sqlite3.Row):
            with self.subTest(factory=factory):
                self.db.row_factory = factory
                changes = self.db.total_changes
                self.assertEqual(
                    economy_crafting.crafting_item_ids(self.db),
                    {900, 2318, 2320, 777, 778},
                )
                self.assertEqual(self.db.total_changes, changes)
                self.assertIs(self.db.row_factory, factory)
        self.db.row_factory = None
        self.assertEqual(
            self.db.execute(
                "SELECT Effect,EffectItemType FROM spell_effect WHERE SpellID=1200"
            ).fetchone(),
            (288, 9999),
        )

    def test_every_missing_reference_table_has_explicit_import_error(self):
        for table in economy_crafting.REFERENCE_TABLES:
            with self.subTest(table=table):
                self.db.execute("SAVEPOINT missing_table")
                self.db.execute(f"DROP TABLE {table}")
                with self.assertRaisesRegex(ValueError, "crafting_data.py.*" + table):
                    economy_crafting.crafting_item_ids(self.db)
                self.db.execute("ROLLBACK TO missing_table")
                self.db.execute("RELEASE missing_table")


if __name__ == "__main__":
    unittest.main()
