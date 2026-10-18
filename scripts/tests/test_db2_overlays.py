"""DB2 refresh preserves already-approved Zephras derived factions, offline."""

import contextlib
import csv
import hashlib
import io
import json
import os
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2
import zephras_content

SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])


class Db2OverlayTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.path = self.root / "world.db"
        self.connection = sqlite3.connect(self.path)
        self.addCleanup(self.connection.close)
        self.connection.executescript(db2.SCHEMA_SQL.read_text())
        self.connection.execute(zephras_content.DDL[0])
        csv_dir = self.root / db2.DEFAULT_BUILD
        csv_dir.mkdir()
        for table in db2.TABLES:
            name = db2.table_name(table)
            columns = [r[1] for r in self.connection.execute(f"PRAGMA table_info({name})")]
            with (csv_dir / f"{table}.csv").open("w", newline="") as handle:
                csv.writer(handle).writerow(columns)
        # Exact source identities/reactions from existing zephras_row_source receipts.
        writer = zephras_content.OverlayWriter(
            self.connection, {"provenance": "forever-zephras-20261004"}
        )
        for npc, reactions in [(250868, [-1, -1]), (249363, [1, 1])]:
            source = {
                "id": npc,
                "react": reactions,
                "source": f"https://www.wowhead.com/forever/npc={npc}",
            }
            writer.insert(
                "faction_template", zephras_content.faction_row(reactions), ["ID"], source
            )
        self.connection.commit()

    def import_csvs(self):
        with contextlib.redirect_stdout(io.StringIO()):
            db2.import_tables(db2.DEFAULT_BUILD, self.root, self.path, SOURCE)

    def test_refresh_restores_derived_reactions_and_records_provenance_twice(self):
        before = self.connection.execute(
            "SELECT * FROM faction_template ORDER BY ID"
        ).fetchall()
        for _ in range(2):
            self.import_csvs()
            after = self.connection.execute(
                "SELECT * FROM faction_template WHERE ID>=90000000 ORDER BY ID"
            ).fetchall()
            self.assertEqual(after, before)
            for row_key, scrape, source in self.connection.execute(
                "SELECT row_key,scrape,source_json FROM zephras_row_source"
            ):
                row_id = json.loads(row_key)["ID"]
                provenance = self.connection.execute(
                    "SELECT build,file_data_id,layout_hash,sha256 FROM db2_row_source "
                    "WHERE table_name='faction_template' AND row_id=?", (row_id,)
                ).fetchone()
                self.assertEqual(
                    provenance,
                    (scrape, 0, "derived", hashlib.sha256(source.encode()).hexdigest()),
                )
            self.assertEqual(
                self.connection.execute(
                    "SELECT build FROM db2_build WHERE table_name='faction_template'"
                ).fetchone(),
                (db2.DEFAULT_BUILD,),
            )

    def test_refresh_preserves_build_receipts_owned_by_other_importers(self):
        tables = ("achievement", "achievement_category", "criteria", "criteria_tree")
        receipts = [(table, db2.DEFAULT_BUILD) for table in tables]
        self.connection.executemany("INSERT INTO db2_build VALUES (?,?)", receipts)
        self.connection.commit()
        for _ in range(2):
            self.import_csvs()
            self.assertEqual(
                self.connection.execute(
                    "SELECT table_name,build FROM db2_build WHERE table_name IN (?,?,?,?) "
                    "ORDER BY table_name", tables
                ).fetchall(),
                receipts,
            )
            self.assertEqual(
                self.connection.execute("SELECT COUNT(*) FROM db2_build").fetchone(),
                (len(db2.TABLES) + len(tables),),
            )

    def test_invalid_reconstruction_key_rolls_back_entire_refresh(self):
        self.connection.execute(
            "UPDATE zephras_row_source SET row_key=? WHERE row_key=?",
            ('{"ID": 90000004}', '{"ID": 90000000}'),
        )
        self.connection.commit()
        before = self.connection.execute("SELECT * FROM faction_template ORDER BY ID").fetchall()
        with self.assertRaisesRegex(ValueError, "faction.*key"):
            self.import_csvs()
        self.assertEqual(
            self.connection.execute("SELECT * FROM faction_template ORDER BY ID").fetchall(),
            before,
        )
        self.assertEqual(self.connection.execute("SELECT * FROM db2_build").fetchall(), [])


if __name__ == "__main__":
    unittest.main()
