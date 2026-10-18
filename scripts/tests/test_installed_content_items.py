"""Installed Retail Item/ItemSparse, not older content references, own definitions."""

from contextlib import closing
from pathlib import Path
import os
import sqlite3
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import content_policy
import retail_content
import skyborne_data
from content_common import Report

SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])

# Concrete battleground gear absent from the old reference-filtered projection.
ITEM_IDS = (
    270629, 270631, 270633, 270635, 270637, 270639, 270641, 270643,
    270805, 270807, 270809, 270811, 270813, 270815, 270817, 270819,
    270831, 270846, 270849,
)


def copy_source_table(source, destination, table, item_ids=None):
    ddl = source.execute(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name=?", (table,)
    ).fetchone()[0]
    destination.execute(ddl)
    query = f'SELECT * FROM "{table}"'
    parameters = ()
    if item_ids is not None:
        query += f" WHERE ID IN ({','.join('?' for _ in item_ids)})"
        parameters = item_ids
    rows = source.execute(query, parameters).fetchall()
    if rows:
        placeholders = ",".join("?" for _ in rows[0])
        destination.executemany(f'INSERT INTO "{table}" VALUES ({placeholders})', rows)


class InstalledContentItemsTest(unittest.TestCase):
    def test_item_only_refresh_preserves_isolated_overlay_and_provenance(self):
        import skyborne_kit_fixture

        with closing(skyborne_kit_fixture.fixture_database(SOURCE)) as db:
            # Keep authentic extra provenance after the overlay rows, as in world.db.
            with closing(sqlite3.connect(f"file:{ROOT / 'data/world.db'}?mode=ro", uri=True)) as source:
                existing = set(db.execute("SELECT table_name,row_id FROM db2_row_source"))
                extra = next(
                    row for row in source.execute("SELECT * FROM db2_row_source")
                    if row[:2] not in existing
                )
                db.execute("INSERT INTO db2_row_source VALUES (?,?,?,?,?,?)", extra)
            before = db.execute("SELECT rowid,* FROM db2_row_source ORDER BY rowid").fetchall()
            overlay = db.execute("SELECT * FROM skyborne_content_item ORDER BY ID").fetchall()
            content_policy.refresh_items(db)
            self.assertEqual(db.execute("SELECT rowid,* FROM db2_row_source ORDER BY rowid").fetchall(), before)
            self.assertEqual(db.execute("SELECT * FROM skyborne_content_item ORDER BY ID").fetchall(), overlay)

    def test_every_installed_definition_is_projected_without_tdb_membership(self):
        with (
            closing(sqlite3.connect(f"file:{ROOT / 'data/world.db'}?mode=ro", uri=True)) as source,
            closing(sqlite3.connect(":memory:")) as db,
        ):
            for table in ("item", "item_sparse"):
                copy_source_table(source, db, table, ITEM_IDS)
            for table in ("item_modified_appearance", "item_appearance"):
                copy_source_table(source, db, table)
            db.executescript((ROOT / "scripts/tdb_schema.sql").read_text())
            for table in (
                "toy", "character_loadout", "character_loadout_item",
                "wowhead_creature_drop", "db2_row_source",
            ):
                ddl = source.execute(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name=?", (table,)
                ).fetchone()[0]
                db.execute(ddl)
            # Empty TDB/toy/drop references must not exclude installed definitions.
            placeholders = ",".join("?" for _ in ITEM_IDS)
            expected = source.execute(
                f"SELECT {content_policy.ITEM_COLUMNS}, NULL AS SourceEntry "
                "FROM item_sparse s JOIN item i ON i.ID=s.ID "
                f"WHERE s.ID IN ({placeholders}) ORDER BY s.ID", ITEM_IDS,
            ).fetchall()
            self.assertEqual(len(expected), len(ITEM_IDS))
            self.assertEqual(retail_content.retail_item_ids(db), set(ITEM_IDS))
            content_policy.create_map_table(db, "item_map", {})
            # Empty authored overlay, with its required isolated schemas.
            for table in skyborne_data.ISOLATED_TABLES:
                ddl = source.execute(
                    "SELECT sql FROM sqlite_master WHERE name=?", (f"skyborne_{table}",)
                ).fetchone()[0]
                db.execute(ddl)
            for _ in range(2):
                content_policy.build_items(db, {}, {}, retail_content.retail_item_ids(db), Report())
                self.assertEqual(db.execute("SELECT * FROM content_item ORDER BY ID").fetchall(), expected)
                db.execute("DROP TABLE temp.item_source")
            # Publication refreshes source values without losing reviewed SourceEntry.
            db.execute("UPDATE content_item SET SourceEntry=ID, Name='stale' WHERE ID=270635")
            refreshed = [
                (*row[:-1], 270635 if row[0] == 270635 else None) for row in expected
            ]
            for _ in range(2):
                content_policy.refresh_items(db)
                actual = db.execute("SELECT * FROM content_item ORDER BY ID").fetchall()
                self.assertEqual(actual, refreshed)


if __name__ == "__main__":
    unittest.main()
