"""Pinned Forever DB2 decoding and scratch-only Skyborne publication."""

import csv
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
import warnings
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2_casc

SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])
REFERENCE = SCRIPTS / "reference/forever-1.60.1.70205"


class SkyborneDecodeTest(unittest.TestCase):
    def test_zephras_row_decodes_strings_arrays_and_selected_id(self):
        data = (SOURCE / "1349477.db2").read_bytes()
        header, rows = db2_casc.decode(
            data,
            text_columns=frozenset(range(1, 7)),
            record_ids={2991},
            inline_arrays={6: 2},
        )
        self.assertEqual(header["layout_hash"], 0xD43AFAC3)
        self.assertEqual(set(rows), {2991})
        self.assertEqual(len(rows[2991]), 30)
        self.assertEqual(rows[2991][1:3], ["2991", "Zephras Isle"])
        self.assertEqual(rows[2991][23], 7198644)

    def test_missing_selected_row_fails_instead_of_importing_partial_data(self):
        data = (SOURCE / "1349477.db2").read_bytes()
        with self.assertRaisesRegex(ValueError, "missing.*999999"):
            db2_casc.decode(data, record_ids={2991, 999999}, inline_arrays={6: 2})

    def test_full_fixed_table_reports_and_skips_zero_filled_sections(self):
        data = (SOURCE / "1349477.db2").read_bytes()
        with warnings.catch_warnings(record=True) as reported:
            warnings.simplefilter("always")
            header, rows = db2_casc.decode(
                data,
                text_columns=frozenset(range(1, 7)),
                inline_arrays={6: 2},
            )
        self.assertEqual(
            [(warning.category, str(warning.message)) for warning in reported],
            [
                (
                    UserWarning,
                    "Skipping zero-filled DB2 section key 0DA4670DB36FB2A9, 2 records",
                ),
                (
                    UserWarning,
                    "Skipping zero-filled DB2 section key D2C028CB7AC14D0B, 5 records",
                ),
                (
                    UserWarning,
                    "Skipping zero-filled DB2 section key DA0E7785727A0A65, 1 records",
                ),
            ],
        )
        self.assertEqual(header["layout_hash"], 0xD43AFAC3)
        self.assertEqual(header["record_count"], 79)
        self.assertEqual(len(rows), 71)
        # Exact IDs from the pinned file's clear section; eight encrypted ID
        # entries are zero-filled, not additional Map 0 records/placeholders.
        self.assertEqual(
            set(rows),
            {
                0,
                1,
                13,
                25,
                29,
                30,
                33,
                34,
                35,
                36,
                37,
                42,
                43,
                44,
                47,
                48,
                70,
                90,
                109,
                129,
                169,
                189,
                209,
                229,
                230,
                249,
                269,
                289,
                309,
                329,
                349,
                369,
                389,
                409,
                429,
                449,
                450,
                469,
                489,
                509,
                529,
                531,
                533,
                2720,
                2784,
                2789,
                2791,
                2804,
                2806,
                2807,
                2817,
                2832,
                2853,
                2856,
                2868,
                2875,
                2902,
                2921,
                2959,
                2980,
                2991,
                2995,
                2996,
                2997,
                2998,
                2999,
                3005,
                3021,
                3065,
                3104,
                3109,
            },
        )
        self.assertTrue(all(len(row) == 30 for row in rows.values()))
        self.assertEqual(rows[0][1], "Azeroth")
        self.assertEqual(
            rows[2991][1:9], ["2991", "Zephras Isle", "", "", "", "", 0, 0]
        )
        self.assertEqual(rows[2991][23], 7198644)


class SkyborneImportTest(unittest.TestCase):
    def test_corrupt_source_fails_before_database_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            shutil.copytree(SOURCE, source)
            path = source / "1349477.db2"
            path.write_bytes(path.read_bytes() + b"corrupt")
            db = root / "scratch.db"
            with closing(sqlite3.connect(db)) as conn, conn:
                conn.executescript((SCRIPTS / "db2_schema.sql").read_text())
                conn.execute("INSERT INTO faction_template (ID,Faction) VALUES (1,1)")
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "skyborne_data.py"),
                    "--db",
                    str(db),
                    "--source-dir",
                    str(source),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("SHA-256", result.stderr)
            with closing(sqlite3.connect(db)) as conn:
                self.assertEqual(
                    conn.execute("SELECT ID,Faction FROM faction_template").fetchall(),
                    [(1, 1)],
                )
                self.assertEqual(
                    conn.execute("SELECT COUNT(*) FROM map").fetchone(), (0,)
                )

    def test_retail_regeneration_reapplies_skyborne_overlay(self):
        import db2

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            csv_dir = root / db2.DEFAULT_BUILD
            csv_dir.mkdir()
            conn = sqlite3.connect(root / "scratch.db")
            conn.executescript((SCRIPTS / "db2_schema.sql").read_text())
            conn.execute("INSERT INTO map (ID, Directory) VALUES (0, 'Azeroth')")
            retail = json.loads((REFERENCE / "retail-kit-runtime-rows.json").read_text())
            for table, records in retail.items():
                for row in records:
                    conn.execute(
                        f"INSERT INTO {table} ({','.join(row)}) VALUES ({','.join('?' for _ in row)})",
                        tuple(row.values()),
                    )
            expected_retail = {
                table: conn.execute(f"SELECT * FROM {table} ORDER BY ID").fetchall()
                for table in retail
            }
            conn.commit()
            for table in db2.TABLES:
                name = db2.table_name(table)
                columns = [row[1] for row in conn.execute(f"PRAGMA table_info({name})")]
                with (csv_dir / f"{table}.csv").open("w", newline="") as handle:
                    writer = csv.writer(handle)
                    writer.writerow(columns)
                    writer.writerows(conn.execute(f"SELECT * FROM {name}"))
            conn.close()
            for _ in range(2):
                result = subprocess.run(
                    [
                        sys.executable,
                        str(SCRIPTS / "db2.py"),
                        "import",
                        "--source-dir",
                        str(SOURCE),
                        "--build",
                        db2.DEFAULT_BUILD,
                        "--csv-root",
                        str(root),
                        "--db",
                        str(root / "scratch.db"),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with closing(sqlite3.connect(root / "scratch.db")) as check:
                    self.assertEqual(
                        check.execute(
                            "SELECT WdtFileDataID FROM map WHERE ID=2991"
                        ).fetchone(),
                        (7198644,),
                    )
                    self.assertEqual(
                        check.execute(
                            "SELECT COUNT(*) FROM faction_template"
                        ).fetchone(),
                        (2,),
                    )
                    self.assertEqual(
                        check.execute(
                            "SELECT Directory FROM map WHERE ID=0"
                        ).fetchone(),
                        ("Azeroth",),
                    )
                    for table, records in retail.items():
                        ids = [row["ID"] for row in records]
                        self.assertEqual(
                            check.execute(
                                f"SELECT * FROM {table} WHERE ID IN ({','.join('?' for _ in ids)}) ORDER BY ID",
                                ids,
                            ).fetchall(),
                            expected_retail[table],
                        )
                    self.assertEqual(
                        check.execute(
                            "SELECT COUNT(*) FROM skyborne_item_sparse"
                        ).fetchone(),
                        (30,),
                    )
                    self.assertEqual(
                        check.execute(
                            "SELECT ClassID,SubclassID FROM skyborne_item WHERE ID=2947"
                        ).fetchone(),
                        (2, 16),
                    )
                    self.assertEqual(
                        check.execute(
                            "SELECT ClassID,SubclassID FROM item WHERE ID=2947"
                        ).fetchone(),
                        (15, 0),
                    )

    def test_import_preserves_retail_rows_and_records_exact_source(self):
        import skyborne_data

        expected_sources = sorted(
            (
                f"skyborne_{table}"
                if table in skyborne_data.ISOLATED_TABLES
                else table,
                row["ID"],
                skyborne_data.BUILD,
            )
            for table, records in skyborne_data.load_rows(SOURCE).items()
            for row in records
        )
        expected_sources = sorted(
            expected_sources
            + [
                ("skyborne_content_item", row["ID"], skyborne_data.BUILD)
                for row in skyborne_data.load_rows(SOURCE)["item_sparse"]
            ]
        )
        with tempfile.TemporaryDirectory() as directory:
            db = Path(directory) / "scratch.db"
            conn = sqlite3.connect(db)
            conn.executescript((SCRIPTS / "db2_schema.sql").read_text())
            conn.execute("INSERT INTO faction_template (ID, Faction) VALUES (1, 1)")
            conn.execute("INSERT INTO map (ID, Directory) VALUES (0, 'Azeroth')")
            conn.commit()
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "skyborne_data.py"),
                    "--db",
                    str(db),
                    "--source-dir",
                    str(SOURCE),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(
                conn.execute(
                    "SELECT ID,Faction,Flags,FactionGroup,FriendGroup,EnemyGroup "
                    "FROM faction_template WHERE ID IN (3628,3629) ORDER BY ID"
                ).fetchall(),
                [(3628, 2794, 72, 5, 4, 10), (3629, 2795, 72, 3, 2, 12)],
            )
            self.assertEqual(
                conn.execute(
                    "SELECT Directory,MapName_lang,WdtFileDataID FROM map WHERE ID=2991"
                ).fetchone(),
                ("2991", "Zephras Isle", 7198644),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT Faction FROM faction_template WHERE ID=1"
                ).fetchone(),
                (1,),
            )
            self.assertEqual(
                conn.execute("SELECT Directory FROM map WHERE ID=0").fetchone(),
                ("Azeroth",),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT table_name,row_id,build FROM db2_row_source ORDER BY table_name,row_id"
                ).fetchall(),
                expected_sources,
            )
            conn.close()
