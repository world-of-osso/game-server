"""Maintained DB2 publisher preserves native Curve type/flags and build identity."""

import csv
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2

SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])
REFERENCE = SCRIPTS / "reference/forever-1.60.1.70205"
FIXTURE_BUILD = "fixture-curve-publication"
# Actual70448 Curve.csv rows: Type0 is linear; retain flags independently.
NATIVE_CURVES = [(8, 0, 2), (33927, 0, 0)]


def write_csv_fixture(root):
    csv_dir = root / FIXTURE_BUILD
    csv_dir.mkdir()
    path = root / "scratch.db"
    with sqlite3.connect(path) as connection:
        connection.executescript((SCRIPTS / "db2_schema.sql").read_text())
        connection.execute("INSERT INTO map (ID, Directory) VALUES (0, 'Azeroth')")
        retail = json.loads((REFERENCE / "retail-kit-runtime-rows.json").read_text())
        for table, records in retail.items():
            for row in records:
                columns = ",".join(row)
                placeholders = ",".join("?" for _ in row)
                connection.execute(
                    f"INSERT INTO {table} ({columns}) VALUES ({placeholders})",
                    tuple(row.values()),
                )
        for table in db2.TABLES:
            name = db2.table_name(table)
            columns = [row[1] for row in connection.execute(f"PRAGMA table_info({name})")]
            with (csv_dir / f"{table}.csv").open("w", newline="") as handle:
                writer = csv.writer(handle)
                writer.writerow(columns)
                writer.writerows(connection.execute(f"SELECT * FROM {name}"))
    with (csv_dir / "Curve.csv").open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["ID", "Type", "Flags"])
        writer.writerows(NATIVE_CURVES)
    return path


class CurvePublicationTest(unittest.TestCase):
    def test_process_import_publishes_native_types_flags_and_build_on_regeneration(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = write_csv_fixture(root)
            for generation in range(2):
                result = subprocess.run(
                    [
                        sys.executable,
                        str(SCRIPTS / "db2.py"),
                        "import",
                        "--source-dir",
                        str(SOURCE),
                        "--build",
                        FIXTURE_BUILD,
                        "--csv-root",
                        str(root),
                        "--db",
                        str(path),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with sqlite3.connect(path) as connection:
                    published = connection.execute(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name='curve'"
                    ).fetchone()
                    self.assertIsNotNone(published, "successful DB2 import must publish Curve types")
                    self.assertEqual(
                        connection.execute("SELECT ID, Type, Flags FROM curve ORDER BY ID").fetchall(),
                        NATIVE_CURVES,
                    )
                    self.assertEqual(
                        connection.execute("SELECT build FROM db2_build WHERE table_name='curve'").fetchone(),
                        (FIXTURE_BUILD,),
                    )
                    self.assertEqual(
                        connection.execute("SELECT Directory FROM map WHERE ID=0").fetchone(),
                        ("Azeroth",),
                    )
                    self.assertEqual(
                        connection.execute("SELECT WdtFileDataID FROM map WHERE ID=2991").fetchone(),
                        (7198644,),
                    )
                    if generation == 0:
                        connection.execute("UPDATE curve SET Type=1 WHERE ID=33927")


if __name__ == "__main__":
    unittest.main()
