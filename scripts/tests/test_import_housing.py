"""Housing imports authenticate receipts and preserve authored catalog facts."""
import csv
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from import_housing import import_catalog

BUILD = "12.1.0.69933"
KEY = "dcfc90fffd79ba00406ae46f5f657592"


def write_evidence(root):
    (root / "db2").mkdir()
    (root / "build-info-receipt.txt").write_text(
        "Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n"
        f"1|{KEY}|{BUILD}|wow\n", encoding="utf-8"
    )
    tables = {
        "HouseDecor": (5149691, "6A051268", [
            {"ID": "80", "Name_lang": "Ornate Stonework Fireplace", "ItemID": "235994",
             "ModelFileDataID": "6033663", "ThumbnailFileDataID": "7415905",
             "StartingQuantity": "1", "Field_12_0_0_63534_015": "0"},
            {"ID": "81", "Name_lang": "Tusked Fireplace", "ItemID": "245398",
             "ModelFileDataID": "6033664", "ThumbnailFileDataID": "7415913",
             "StartingQuantity": "1", "Field_12_0_0_63534_015": "0"},
        ]),
        "NeighborhoodMap": (5550624, "4973BA38", [
            {"ID": "1", "MapID": "2735", "EntryPosition_0": "3807.760009765625"},
        ]),
        "NeighborhoodPlot": (5550625, "03FC4E53", [
            {"ID": "359", "NeighborhoodMapID": "1", "Cost": "10000000"},
        ]),
    }
    reports, checks = [], []
    for name, (fdid, layout, rows) in tables.items():
        raw = f"local fixture bytes for {name}".encode()
        (root / "db2" / f"{fdid}.db2").write_bytes(raw)
        with (root / f"{name}.csv").open("w", newline="", encoding="utf-8") as output:
            writer = csv.DictWriter(output, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
        reports.append({"table": name, "fdid": fdid, "housing": True,
                        "status": "exported", "layout": layout,
                        "columns": list(rows[0]), "exported_records": len(rows),
                        "encrypted_records_dropped": 20 if name == "HouseDecor" else 0,
                        "sections": []})
        checks.append({"table": name, "layout_hash": layout,
                       "csv_rows": len(rows), "row_count_and_unique_ids": "pass",
                       "csv_sha256": hashlib.sha256((root / f"{name}.csv").read_bytes()).hexdigest(),
                       "db2_sha256": hashlib.sha256(raw).hexdigest()})
    reports.append({"table": "NeighborhoodInitiativeTask", "housing": True,
                    "fdid": 6702595, "status": "absent from local CASC"})
    (root / "table-export-report.json").write_text(json.dumps(reports))
    (root / "audit-proof-ledger.json").write_text(json.dumps({"table_checks": checks}))


class HousingImportTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.evidence = self.root / "evidence"
        self.evidence.mkdir()
        write_evidence(self.evidence)
        self.db = self.root / "world.db"
        with sqlite3.connect(self.db) as db:
            db.execute("CREATE TABLE unrelated (name TEXT)")
            db.execute("INSERT INTO unrelated VALUES ('keep existing world')")

    def import_catalog(self, **kwargs):
        return import_catalog(self.evidence, self.db, kwargs.get("build", BUILD), kwargs.get("build_key", KEY))

    def test_authored_catalog_is_build_qualified_idempotent_and_not_owned(self):
        self.import_catalog()
        self.import_catalog()
        with sqlite3.connect(self.db) as db:
            rows = db.execute("SELECT build_key, record_id, fields_json FROM housing_catalog_record WHERE table_name='HouseDecor' ORDER BY record_id").fetchall()
            self.assertEqual(len(rows), 2)
            self.assertEqual(rows[0][:2], (KEY, 80))
            decor = json.loads(rows[0][2])
            self.assertEqual(decor["Name_lang"], "Ornate Stonework Fireplace")
            self.assertEqual(decor["StartingQuantity"], "1")
            self.assertEqual(decor["Field_12_0_0_63534_015"], "0")
            self.assertNotIn("owned", decor)
            plot = json.loads(db.execute("SELECT fields_json FROM housing_catalog_record WHERE table_name='NeighborhoodPlot'").fetchone()[0])
            self.assertEqual(plot["Cost"], "10000000")
            self.assertNotIn("currency", plot)
            self.assertEqual(db.execute("SELECT name FROM unrelated").fetchone()[0], "keep existing world")
            gaps = db.execute("SELECT table_name, status, encrypted_records_dropped FROM housing_catalog_source WHERE build_key=? ORDER BY table_name", (KEY,)).fetchall()
            self.assertIn(("HouseDecor", "exported", 20), gaps)
            self.assertIn(("NeighborhoodInitiativeTask", "absent from local CASC", 0), gaps)

    def test_wrong_build_or_key_leaves_database_unchanged(self):
        before = self.db.read_bytes()
        for values in [{"build": "12.1.0.70338"}, {"build_key": "f" * 32}]:
            with self.assertRaisesRegex(ValueError, "build"):
                self.import_catalog(**values)
            self.assertEqual(self.db.read_bytes(), before)

    def test_tampered_csv_or_db2_does_not_replace_previous_catalog(self):
        self.import_catalog()
        before = self.db.read_bytes()
        for path in [self.evidence / "HouseDecor.csv", self.evidence / "db2/5149691.db2"]:
            original = path.read_bytes()
            path.write_bytes(original + b"tampered")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                self.import_catalog()
            self.assertEqual(self.db.read_bytes(), before)
            path.write_bytes(original)

    def test_cross_table_failure_rolls_back_whole_catalog(self):
        self.import_catalog()
        with sqlite3.connect(self.db) as db:
            db.execute("CREATE TRIGGER reject_plot BEFORE INSERT ON housing_catalog_record WHEN NEW.table_name='NeighborhoodPlot' BEGIN SELECT RAISE(ABORT, 'reject fixture plot'); END")
        before = self.db.read_bytes()
        with self.assertRaisesRegex(sqlite3.IntegrityError, "reject fixture plot"):
            self.import_catalog()
        self.assertEqual(self.db.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
