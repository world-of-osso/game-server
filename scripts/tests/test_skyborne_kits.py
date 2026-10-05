"""Forever kit source selection and atomic publication on owned SQLite fixtures."""

import copy
import json
from pathlib import Path
import sqlite3
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import skyborne_data

SOURCE = SCRIPTS / "reference/forever-1.60.1.70205"
DECODED = json.loads((SOURCE / "starter-kit-rows.json").read_text())


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


if __name__ == "__main__":
    unittest.main()
