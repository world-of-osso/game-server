"""Additive Retail projection and rebuild behavior on isolated SQLite fixtures."""

import csv
import importlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import content_policy
from content_common import Report
from test_import_azerothcore import write_db2_rows, write_header_only_csvs


class CraftingItemTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.path = self.root / "world.db"
        self.db = sqlite3.connect(self.path)
        self.addCleanup(self.db.close)
        for schema in (
            "world_schema.sql",
            "db2_schema.sql",
            "tdb_schema.sql",
            "crafting_schema.sql",
        ):
            self.db.executescript((SCRIPTS / schema).read_text())
        self.csv_root = self.root / "csv"
        write_header_only_csvs(self.csv_root)
        self.source_rows(
            "ItemSparse",
            "item_sparse",
            [
                {
                    "ID": 2302,
                    "Display_lang": "Handstitched Leather Boots",
                    "Stackable": 1,
                },
                {
                    "ID": 244612,
                    "Display_lang": "Row Walker's Deflectors",
                    "ExpansionID": 11,
                    "OverallQualityID": 4,
                    "ItemLevel": 197,
                    "RequiredLevel": 80,
                    "BuyPrice": 2506951,
                    "SellPrice": 501390,
                    "VendorStackCount": 1,
                    "Stackable": 1,
                    "InventoryType": 9,
                    "Bonding": 1,
                    "ItemSet": 1966,
                    "AllowableClass": -1,
                    "AllowableRaces_0": -1,
                    "AllowableRaces_1": -1,
                    "Flags_1": 24576,
                    "Flags_3": 16777216,
                    "StatModifier_bonusStat_0": 73,
                    "StatPercentEditor_0": 5259,
                    "StatModifier_bonusStat_1": 7,
                    "StatPercentEditor_1": 7889,
                    "StatModifier_bonusStat_2": 49,
                    "StatPercentEditor_2": 3000,
                    "StatModifier_bonusStat_3": 40,
                    "StatPercentEditor_3": 4000,
                    **{f"StatModifier_bonusStat_{i}": -1 for i in range(4, 10)},
                },
                {"ID": 2318, "Display_lang": "Light Leather", "Stackable": 1000},
                {"ID": 2320, "Display_lang": "Coarse Thread", "Stackable": 1000},
            ],
        )
        self.source_rows(
            "Item",
            "item",
            [
                {"ID": 2302, "ClassID": 4, "SubclassID": 1},
                {"ID": 244612, "ClassID": 4, "SubclassID": 2},
                {"ID": 2318, "ClassID": 7, "SubclassID": 6},
                {"ID": 2320, "ClassID": 7, "SubclassID": 0},
            ],
        )
        self.db.execute(
            "INSERT INTO spell_effect(ID,SpellID,Effect,DifficultyID,EffectMiscValue_0) VALUES (1,1237486,288,0,3978)"
        )
        self.db.execute(
            "INSERT INTO crafting_data VALUES (3978,0,0,244612,0,0,0,0,0,0,0,0,0,0,0,0)"
        )
        self.db.execute(
            "INSERT INTO spell_reagents(ID,SpellID,Reagent_0,ReagentCount_0,Reagent_1,ReagentCount_1) VALUES (1,1237486,2318,2,2320,1)"
        )
        content_policy.create_map_table(self.db, "item_map", {2302: 2302})
        content_policy.build_items(self.db, {2302: 2302}, {}, set(), Report())
        # Persisted original must survive even if the current source changed.
        self.db.execute(
            "UPDATE content_item SET Name='Existing saved boots', BuyPrice=17, SourceEntry=2302 WHERE ID=2302"
        )
        self.db.commit()

    def source_rows(self, db2_name, table, rows):
        write_db2_rows(self.csv_root, db2_name, rows)
        with (self.csv_root / f"{db2_name}.csv").open(newline="") as handle:
            reader = csv.reader(handle)
            header = next(reader)
            self.db.executemany(
                f"INSERT INTO {table} ({','.join(header)}) VALUES ({','.join('?' for _ in header)})",
                reader,
            )

    def module(self):
        return importlib.import_module("crafting_items")

    def rows(self):
        return self.db.execute("SELECT * FROM content_item ORDER BY ID").fetchall()

    def test_existing_2302_preserved_and_new_244612_uses_exact_retail_fields(self):
        before = self.rows()
        result = self.module().project_crafting_items(self.db)
        self.assertEqual(
            self.db.execute("SELECT * FROM content_item WHERE ID=2302").fetchall(),
            before,
        )
        row = self.db.execute("SELECT * FROM content_item WHERE ID=244612")
        names = [column[0] for column in row.description]
        values = row.fetchone()
        expected = self.db.execute(
            f"SELECT {content_policy.ITEM_COLUMNS}, NULL AS SourceEntry FROM item_sparse s JOIN item i ON i.ID=s.ID WHERE s.ID=244612"
        ).fetchone()
        self.assertEqual(values, expected)
        fields = dict(zip(names, values))
        self.assertEqual(
            (
                fields["Name"],
                fields["ClassID"],
                fields["SubclassID"],
                fields["Quality"],
                fields["ItemLevel"],
                fields["BuyPrice"],
                fields["SellPrice"],
                fields["SourceEntry"],
            ),
            ("Row Walker's Deflectors", 4, 2, 4, 197, 2506951, 501390, None),
        )
        self.assertEqual(result["added"], [2318, 2320, 244612])
        self.assertEqual(result["missing_item"], [])
        self.assertEqual(result["missing_item_sparse"], [])

    def test_absent_item_and_sparse_are_reported_not_inserted(self):
        self.db.execute("DELETE FROM item WHERE ID=2318")
        self.db.execute("DELETE FROM item_sparse WHERE ID=2320")
        self.db.execute("UPDATE spell_reagents SET Reagent_2=999999,ReagentCount_2=1")
        result = self.module().project_crafting_items(self.db)
        self.assertEqual(result["missing_item"], [2318, 999999])
        self.assertEqual(result["missing_item_sparse"], [2320, 999999])
        self.assertEqual(result["added"], [244612])
        self.assertEqual([row[0] for row in self.rows()], [2302, 244612])

    def test_idempotent_update_keeps_existing_rows_and_sourceentry(self):
        mod = self.module()
        mod.project_crafting_items(self.db)
        before = self.rows()
        result = mod.project_crafting_items(self.db)
        self.assertEqual(result["added"], [])
        self.assertEqual(result["existing"], [2318, 2320, 244612])
        self.assertEqual(self.rows(), before)

    def test_invalid_projection_rolls_back_earlier_additions(self):
        # SQLite checks the projected value, not a synthetic substitute.
        self.db.execute(
            "ALTER TABLE content_item ADD COLUMN Valid INTEGER CHECK (typeof(ItemLevel)='integer')"
        )
        self.db.execute("UPDATE item_sparse SET ItemLevel='invalid' WHERE ID=244612")
        self.db.commit()
        before = self.rows()
        with self.assertRaisesRegex(sqlite3.IntegrityError, "CHECK constraint"):
            self.module().project_crafting_items(self.db)
        self.assertEqual(self.rows(), before)
        self.assertFalse(self.db.in_transaction)

    def test_helper_does_not_commit_or_change_caller_row_factory(self):
        self.db.row_factory = sqlite3.Row
        self.db.execute("BEGIN")
        self.db.execute("UPDATE content_item SET SellPrice=99 WHERE ID=2302")
        self.module().project_crafting_items(self.db)
        self.assertTrue(self.db.in_transaction)
        self.assertIs(self.db.row_factory, sqlite3.Row)
        self.db.rollback()
        self.db.row_factory = None
        self.assertEqual([row[0] for row in self.rows()], [2302])
        self.assertNotEqual(
            self.db.execute(
                "SELECT SellPrice FROM content_item WHERE ID=2302"
            ).fetchone()[0],
            99,
        )

    def test_missing_modern_tables_is_clear_error_without_writes(self):
        self.db.execute("DROP TABLE crafting_data_item_quality")
        before = self.rows()
        with self.assertRaisesRegex(
            ValueError, "Import crafting references.*crafting_data_item_quality"
        ):
            self.module().project_crafting_items(self.db)
        self.assertEqual(self.rows(), before)

    def test_cli_requires_explicit_paths_and_reports_atomic_addition(self):
        import subprocess

        command = [sys.executable, str(SCRIPTS / "crafting_items.py")]
        missing = subprocess.run(command, capture_output=True, text=True)
        self.assertNotEqual(missing.returncode, 0)
        report = self.root / "report.json"
        result = subprocess.run(
            command + ["--db", str(self.path), "--report", str(report)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(report.read_text())
        self.assertEqual(payload["db"], str(self.path.resolve()))
        self.assertEqual(payload["added"], [2318, 2320, 244612])
        self.assertEqual(
            self.db.execute(
                "SELECT SourceEntry FROM content_item WHERE ID=244612"
            ).fetchone(),
            (None,),
        )
        absent = self.root / "absent.db"
        result = subprocess.run(
            command + ["--db", str(absent), "--report", str(report)],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(absent.exists())

    def test_full_rebuild_keeps_outputs_fixed_reagents_and_category_alternatives(self):
        self.db.execute(
            "INSERT INTO modified_crafting_spell_slot VALUES (15388,1237486,0,421,0,140,80)"
        )
        self.db.execute(
            "INSERT INTO modified_crafting_reagent_slot VALUES ('Material',421,0,0,1,2,0,0)"
        )
        self.db.execute(
            "INSERT INTO modified_crafting_category VALUES (10,'Leather','Material',0,0,0)"
        )
        self.db.execute(
            "INSERT INTO modified_crafting_reagent_item VALUES (20,'Material',10,0,0,0,0)"
        )
        self.db.execute("INSERT INTO mcr_slot_x_mcr_category VALUES (1,10,0,421)")
        self.source_rows(
            "ItemSparse",
            "item_sparse",
            [
                {
                    "ID": 244603,
                    "Display_lang": "Quality material",
                    "ModifiedCraftingReagentItemID": 20,
                }
            ],
        )
        self.source_rows(
            "Item", "item", [{"ID": 244603, "ClassID": 7, "SubclassID": 6}]
        )
        self.db.execute(
            "INSERT INTO item_template(entry,name) VALUES (2302,'Handstitched Leather Boots')"
        )
        self.db.executemany(
            "INSERT INTO map(ID,InstanceType) VALUES (?,0)",
            [(map_id,) for map_id in content_policy.retail_content.RETAIL_MAPS],
        )
        self.db.execute("DROP TABLE temp.item_map")
        self.db.execute("DROP TABLE temp.item_source")
        overrides = self.root / "overrides.csv"
        overrides.write_text(
            "kind,source_id,retail_id,source_name,retail_name,reason\n"
        )
        import os
        from unittest.mock import patch

        with patch.dict(os.environ, {"WOWHEAD_DIR": str(self.root / "no-scrapes")}):
            content_policy.apply_policy(self.db, {}, overrides, Report())
        self.assertEqual(
            [row[0] for row in self.rows()], [2302, 2318, 2320, 244603, 244612]
        )
        self.assertEqual(
            self.db.execute(
                "SELECT SourceEntry FROM content_item WHERE ID=2302"
            ).fetchone(),
            (2302,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT SourceEntry FROM content_item WHERE ID=244612"
            ).fetchone(),
            (None,),
        )


if __name__ == "__main__":
    unittest.main()
