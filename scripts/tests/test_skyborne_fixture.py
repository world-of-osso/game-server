"""The portable native fixture preserves the actual importer/regeneration outputs."""

from contextlib import closing
from pathlib import Path
import os
import sqlite3
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import skyborne_kit_fixture

from tests.fixture_env import env_path, requires_env
SOURCE = env_path("FOREVER_DB2_SOURCE")


@requires_env("FOREVER_DB2_SOURCE")
class PortableFixtureTest(unittest.TestCase):
    def test_native_fixture_matches_current_source_definitions_and_retail_rows(self):
        path = SCRIPTS.parent / "crates/server/src/skyborne_kit_fixture.rs"
        # Deserialize the generated fixture's raw string, then observe its SQLite data.
        sql = path.read_text().split('r#"', 1)[1].rsplit('"#;', 1)[0]
        with (
            closing(sqlite3.connect(":memory:")) as portable,
            closing(skyborne_kit_fixture.fixture_database(SOURCE)) as current,
        ):
            portable.executescript(sql)
            tables = {
                row[0]
                for row in current.execute(
                    "SELECT name FROM sqlite_master WHERE type='table'"
                )
            }
            self.assertEqual(
                tables,
                {
                    row[0]
                    for row in portable.execute(
                        "SELECT name FROM sqlite_master WHERE type='table'"
                    )
                },
            )
            for table in tables:
                columns = [
                    row[1] for row in current.execute(f"PRAGMA table_info({table})")
                ]
                order = ",".join(f'"{column}"' for column in columns)
                query = f'SELECT * FROM "{table}" ORDER BY {order}'
                self.assertEqual(
                    portable.execute(query).fetchall(),
                    current.execute(query).fetchall(),
                    table,
                )
            self.assertEqual(
                portable.execute("SELECT ID,ArmorTypeMask FROM chr_classes ORDER BY ID").fetchall(),
                [(1, 113), (3, 41), (4, 37), (7, 2665), (8, 35), (11, 2341)],
            )
            self.assertEqual(
                portable.execute(
                    "SELECT DISTINCT e.SpellID,e.EquippedItemSubclass "
                    "FROM spell_equipped_items e JOIN spell_effect f ON f.SpellID=e.SpellID "
                    "WHERE f.Effect=60 AND e.EquippedItemClass=2"
                ).fetchall(),
                [],
            )
            self.assertEqual(
                portable.execute(
                    "SELECT ClassID,SubclassID,Stackable,VendorStackCount FROM skyborne_content_item WHERE ID=2947"
                ).fetchone(),
                (2, 16, 1, 1),
            )
            self.assertEqual(
                portable.execute(
                    "SELECT ClassID,SubclassID,Stackable,VendorStackCount FROM content_item WHERE ID=2947"
                ).fetchone(),
                (15, 0, 200, 200),
            )
            self.assertEqual(
                portable.execute(
                    "SELECT COUNT(*) FROM character_loadout_item WHERE CharacterLoadoutID BETWEEN 2373 AND 2378"
                ).fetchone(),
                (45,),
            )


if __name__ == "__main__":
    unittest.main()
