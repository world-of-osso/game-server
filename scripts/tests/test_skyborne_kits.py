"""Forever kit source selection and atomic publication on owned SQLite fixtures."""

import copy
import csv
import tempfile
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import skyborne_data
import content_policy
from content_common import Report

from tests.fixture_env import env_path, requires_env
SOURCE = env_path("FOREVER_DB2_SOURCE")
REFERENCE = SCRIPTS / "reference/forever-1.60.1.70205"
DECODED = json.loads((REFERENCE / "starter-kit-rows.json").read_text())


@requires_env("FOREVER_DB2_SOURCE")
class LoadoutPublicationTest(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.executescript((SCRIPTS / "db2_schema.sql").read_text())
        self.db.execute("INSERT INTO character_loadout VALUES (1277,8,9,0,1,0)")
        # Shared Retail masks must stay intact, including Retail's race-95 bit.
        self.db.execute("INSERT INTO character_loadout VALUES (2200,1,9,0,1,1)")
        self.db.execute("INSERT INTO character_loadout_item VALUES (100,2200,25)")
        self.db.commit()

    def test_pinned_decoder_matches_concrete_authored_rows(self):
        rows = skyborne_data.load_rows(SOURCE)
        for table in DECODED:
            self.assertEqual(rows[table], DECODED[table])

    def test_decoded_authored_memberships_preserve_retail_and_are_idempotent(self):
        for _ in range(2):
            with self.db:
                skyborne_data.import_rows(self.db, DECODED)
        self.assertEqual(
            self.db.execute("SELECT * FROM character_loadout WHERE ID=2200").fetchone(),
            (2200, 1, 9, 0, 1, 1),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT * FROM character_loadout_item WHERE ID=100"
            ).fetchone(),
            (100, 2200, 25),
        )
        self.assertEqual(
            [
                r[0]
                for r in self.db.execute(
                    "SELECT ItemID FROM character_loadout_item WHERE CharacterLoadoutID=2375 ORDER BY ID"
                )
            ],
            [117, 6948, 271671, 271672, 271669, 280399, 876],
        )
        self.assertEqual(
            self.db.execute(
                "SELECT COUNT(*) FROM db2_row_source WHERE table_name='character_loadout_item'"
            ).fetchone(),
            (45,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT build,file_data_id,layout_hash,sha256 FROM db2_row_source WHERE table_name='character_loadout' AND row_id=2375"
            ).fetchone(),
            (
                "1.60.1.70205",
                1344281,
                "713CE8BB",
                "295c28c62daf2925025cb315872420bd5cc4e7d3595593ee616cafa176e47f23",
            ),
        )

    def test_collision_rolls_back_all_authored_rows_and_preserves_retail(self):
        self.db.execute("INSERT INTO character_loadout VALUES (2375,1,9,0,1,0)")
        self.db.commit()
        before = self.db.execute(
            "SELECT * FROM character_loadout ORDER BY ID"
        ).fetchall()
        with self.assertRaisesRegex(ValueError, "collision.*character_loadout.*2375"):
            with self.db:
                skyborne_data.import_rows(self.db, DECODED)
        self.assertEqual(
            self.db.execute("SELECT * FROM character_loadout ORDER BY ID").fetchall(),
            before,
        )
        self.assertEqual(
            self.db.execute("SELECT COUNT(*) FROM character_loadout_item").fetchone(),
            (1,),
        )

    def test_only_pinned_authored_selector_is_accepted(self):
        for field, value in [
            ("Purpose", 0),
            ("ModID", 0),
            ("Extra", 1),
            ("RaceMasks_1", 1),
        ]:
            rows = copy.deepcopy(DECODED)
            rows["character_loadout"][0][field] = value
            with (
                self.subTest(field=field),
                self.assertRaisesRegex(ValueError, "selector"),
            ):
                with self.db:
                    skyborne_data.import_rows(self.db, rows)


@requires_env("FOREVER_DB2_SOURCE")
class IsolatedItemPublicationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = skyborne_data.load_rows(SOURCE)
        cls.retail = json.loads((REFERENCE / "retail-common-items.json").read_text())

    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.executescript((SCRIPTS / "db2_schema.sql").read_text())
        for table in ("item", "item_sparse"):
            for row in self.retail[table]:
                self.db.execute(
                    f"INSERT INTO {table} ({','.join(row)}) VALUES ({','.join('?' for _ in row)})",
                    tuple(row.values()),
                )
        content_policy.create_map_table(self.db, "item_map", {})
        content_policy.build_items(
            self.db, {}, {}, {r["ID"] for r in self.retail["item"]}, Report()
        )
        self.db.commit()

    def snapshot_retail(self):
        return {
            table: self.db.execute(f"SELECT * FROM {table} ORDER BY ID").fetchall()
            for table in ("item", "item_sparse", "content_item")
        }

    def test_shared_ids_have_actual_forever_definitions_without_retail_replacement(
        self,
    ):
        before = self.snapshot_retail()
        with self.db:
            skyborne_data.import_rows(self.db, self.rows)
            skyborne_data.project_items(self.db)
        self.assertEqual(self.snapshot_retail(), before)
        self.assertEqual(
            self.db.execute(
                "SELECT ClassID,SubclassID,InventoryType,Stackable,VendorStackCount FROM skyborne_content_item WHERE ID=2947"
            ).fetchone(),
            (2, 16, 25, 1, 1),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT ClassID,SubclassID,Stackable FROM skyborne_content_item WHERE ID=2512"
            ).fetchone(),
            (6, 2, 200),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT ClassID,SubclassID FROM skyborne_content_item WHERE ID=2101"
            ).fetchone(),
            (11, 2),
        )
        self.assertEqual(
            self.db.execute("SELECT COUNT(*) FROM skyborne_content_item").fetchone(),
            (30,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT Name,InventoryType,RequiredLevel,AllowableClass FROM skyborne_content_item WHERE ID=271655"
            ).fetchone(),
            ("Thendal Apprentice's Shirt", 4, 0, -1),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT file_data_id,layout_hash,sha256 FROM db2_row_source WHERE table_name='skyborne_item_sparse' AND row_id=271655"
            ).fetchone(),
            (
                1572924,
                "6FCC3191",
                "15fde0ce086da0e642011b1f99b49977b6dfa56e72cf4edc941401dddf139f6d",
            ),
        )

    def test_food_drink_effects_decode_declared_signed_and_unsigned_widths(self):
        rows = {row["ID"]: row for row in self.rows["item_effect"]}
        self.assertEqual(
            (rows[98985]["SpellCategoryID"], rows[98985]["Charges"]), (11, -1)
        )
        self.assertEqual(
            (rows[97218]["SpellCategoryID"], rows[97218]["Charges"]), (59, -1)
        )
        self.assertEqual(rows[98264]["SpellCategoryID"], 0)

    def test_missing_required_definition_rolls_back_loadouts_and_isolated_tables(self):
        rows = copy.deepcopy(self.rows)
        rows["item_sparse"] = [
            row for row in rows["item_sparse"] if row["ID"] != 271655
        ]
        before = self.snapshot_retail()
        with self.assertRaisesRegex(ValueError, "missing.*item_sparse.*271655"):
            with self.db:
                skyborne_data.import_rows(self.db, rows)
        self.assertEqual(self.snapshot_retail(), before)
        self.assertEqual(
            self.db.execute("SELECT COUNT(*) FROM character_loadout").fetchone(), (0,)
        )

    def test_missing_required_table_is_explicit_not_retail_substitution(self):
        rows = copy.deepcopy(self.rows)
        del rows["item_sparse"]
        with self.assertRaisesRegex(ValueError, "missing.*item_sparse"):
            skyborne_data.import_rows(self.db, rows)
        self.assertEqual(
            self.db.execute("SELECT COUNT(*) FROM character_loadout").fetchone(), (0,)
        )
        skyborne_data.import_rows(self.db, self.rows)
        self.db.execute("DROP TABLE skyborne_item")
        with self.assertRaisesRegex(ValueError, "missing.*skyborne_item"):
            skyborne_data.project_items(self.db)

    def test_missing_appearance_or_effect_parent_rolls_back(self):
        for table, required in [("item_appearance", 130040), ("item_effect", 98985)]:
            rows = copy.deepcopy(self.rows)
            rows[table] = [row for row in rows[table] if row["ID"] != required]
            with (
                self.subTest(table=table),
                self.assertRaisesRegex(ValueError, f"missing.*{table}.*{required}"),
            ):
                skyborne_data.import_rows(self.db, rows)
            self.assertEqual(
                self.db.execute("SELECT COUNT(*) FROM character_loadout").fetchone(),
                (0,),
            )

    def test_conflicting_isolated_row_rolls_back_without_changing_retail(self):
        with self.db:
            skyborne_data.import_rows(self.db, self.rows)
            skyborne_data.project_items(self.db)
        self.db.execute("UPDATE skyborne_item SET ClassID=15 WHERE ID=2947")
        self.db.commit()
        before = self.snapshot_retail()
        with self.assertRaisesRegex(ValueError, "collision.*skyborne_item.*2947"):
            with self.db:
                skyborne_data.import_rows(self.db, self.rows)
        self.assertEqual(self.snapshot_retail(), before)
        self.assertEqual(
            self.db.execute(
                "SELECT ClassID FROM skyborne_item WHERE ID=2947"
            ).fetchone(),
            (15,),
        )

    def test_additive_kit_refresh_reapplies_forever_without_erasing_retail(self):
        import add_player_create_info
        import db2

        self.db.executescript((SCRIPTS / "world_schema.sql").read_text())
        self.db.executescript((SCRIPTS / "tdb_schema.sql").read_text())
        self.db.execute("CREATE TABLE wowhead_creature_drop (item INTEGER)")
        retail = json.loads((REFERENCE / "retail-kit-runtime-rows.json").read_text())
        with self.db:
            skyborne_data.import_rows(self.db, self.rows)
            skyborne_data.project_items(self.db)
        expected_retail = self.snapshot_retail()
        with tempfile.TemporaryDirectory() as directory:
            csv_dir = Path(directory) / db2.DEFAULT_BUILD
            csv_dir.mkdir()
            for db2_name in add_player_create_info.DB2_TABLES:
                table = db2.table_name(db2_name)
                columns = [
                    row[1] for row in self.db.execute(f"PRAGMA table_info({table})")
                ]
                with (csv_dir / f"{db2_name}.csv").open("w", newline="") as handle:
                    writer = csv.writer(handle)
                    writer.writerow(columns)
                    writer.writerows(
                        [row[column] for column in columns] for row in retail[table]
                    )
            with self.db:
                add_player_create_info.add_player_create_info(self.db, csv_dir, SOURCE)
        self.assertEqual(self.snapshot_retail(), expected_retail)
        self.assertEqual(
            self.db.execute(
                "SELECT COUNT(*) FROM character_loadout_item WHERE CharacterLoadoutID BETWEEN 2373 AND 2378"
            ).fetchone(),
            (45,),
        )
        self.assertEqual(
            self.db.execute(
                "SELECT ModID,RaceMasks_0,RaceMasks_1 FROM character_loadout WHERE ID=1486"
            ).fetchone(),
            (75, 0, 1),
        )
        self.assertEqual(
            self.db.execute("SELECT COUNT(*) FROM skyborne_content_item").fetchone(),
            (30,),
        )

    def test_standalone_projection_collision_rolls_back_entire_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.db"
            db = sqlite3.connect(path)
            db.executescript((SCRIPTS / "db2_schema.sql").read_text())
            db.execute("INSERT INTO faction_template (ID,Faction) VALUES (1,1)")
            db.execute("CREATE VIEW skyborne_content_item AS SELECT ID FROM item")
            db.commit()
            db.close()
            result = subprocess.run(
                [
                    sys.executable, str(SCRIPTS / "skyborne_data.py"),
                    "--source-dir", str(SOURCE), "--db", str(path),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("view", result.stderr.lower())
            db = sqlite3.connect(path)
            try:
                self.assertEqual(
                    db.execute("SELECT ID,Faction FROM faction_template").fetchall(),
                    [(1, 1)],
                )
                self.assertEqual(
                    db.execute("SELECT COUNT(*) FROM character_loadout").fetchone(),
                    (0,),
                )
                self.assertEqual(
                    db.execute(
                        "SELECT name FROM sqlite_master WHERE name='skyborne_item'"
                    ).fetchall(),
                    [],
                )
            finally:
                db.close()

    def test_content_regeneration_reprojects_isolated_definitions_and_preserves_retail(
        self,
    ):
        with self.db:
            skyborne_data.import_rows(self.db, self.rows)
            skyborne_data.project_items(self.db)
        before = self.snapshot_retail()
        expected = self.db.execute(
            "SELECT * FROM skyborne_content_item ORDER BY ID"
        ).fetchall()
        self.db.execute("DELETE FROM skyborne_content_item")
        for _ in range(2):
            self.db.execute("DROP TABLE temp.item_source")
            content_policy.build_items(
                self.db, {}, {}, {r["ID"] for r in self.retail["item"]}, Report()
            )
            self.assertEqual(
                self.db.execute(
                    "SELECT * FROM skyborne_content_item ORDER BY ID"
                ).fetchall(),
                expected,
            )
            self.assertEqual(self.snapshot_retail(), before)


if __name__ == "__main__":
    unittest.main()
