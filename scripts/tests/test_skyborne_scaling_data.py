"""Behavioral publication proof using nine exact local Forever CSV snapshots."""

from contextlib import closing
import csv
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
FIXTURE = SCRIPTS / "tests/fixtures/forever-scaling-70205"
CLI = SCRIPTS / "skyborne_scaling_data.py"


class ScalingPublicationTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.catalog = self.root / "catalog"
        shutil.copytree(FIXTURE, self.catalog)
        self.db = self.root / "world.db"
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.executescript((SCRIPTS / "db2_schema.sql").read_text())
            connection.execute(
                "INSERT INTO item_armor_total (ID,ItemLevel,Cloth) VALUES (1,1,999)"
            )
            connection.execute(
                "INSERT INTO rand_prop_points (ID,EpicF_0) VALUES (1,1234)"
            )
            connection.execute(
                "CREATE TABLE retail_bytes (id INTEGER PRIMARY KEY, payload BLOB)"
            )
            connection.execute(
                "INSERT INTO retail_bytes VALUES (1,?)", (b"\x00\xffretail",)
            )
            connection.execute(
                "CREATE TABLE db2_row_source (table_name TEXT NOT NULL,row_id INTEGER NOT NULL,build TEXT NOT NULL,file_data_id INTEGER NOT NULL,layout_hash TEXT NOT NULL,sha256 TEXT NOT NULL,PRIMARY KEY(table_name,row_id))"
            )
            connection.execute(
                "INSERT INTO db2_row_source VALUES ('item_armor_total',1,'12.1.0.69933',1283022,'RETAIL','retail-digest')"
            )
            connection.execute(
                "CREATE TABLE skyborne_item_sparse (ID INTEGER PRIMARY KEY,ItemLevel INTEGER)"
            )
            connection.executemany(
                "INSERT INTO skyborne_item_sparse VALUES (?,?)", [(2947, 3), (117, 5)]
            )

    def invoke(self, *extra):
        return subprocess.run(
            [
                sys.executable,
                str(CLI),
                "--catalog",
                str(self.catalog),
                "--db",
                str(self.db),
                *extra,
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def snapshot(self):
        with closing(sqlite3.connect(self.db)) as connection:
            return list(connection.iterdump())

    def update_manifest(self, edit):
        path = self.catalog / "manifest.json"
        manifest = json.loads(path.read_text())
        edit(manifest)
        path.write_text(json.dumps(manifest))

    def rewrite_csv(self, name, edit):
        path = self.catalog / f"{name}.csv"
        with path.open(newline="") as stream:
            rows = list(csv.reader(stream))
        edit(rows)
        with path.open("w", newline="") as stream:
            csv.writer(stream).writerows(rows)

        def update(manifest):
            output = manifest["outputs"][path.name]
            output.update(
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                columns=rows[0],
                rows=len(rows) - 1,
                row_ids=[int(row[0]) for row in rows[1:]],
            )

        self.update_manifest(update)

    def test_source_floats_provenance_and_retail_preservation_are_deterministic(self):
        before = self.snapshot()
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        published = self.snapshot()
        with closing(sqlite3.connect(self.db)) as connection:
            self.assertEqual(
                connection.execute(
                    "SELECT Cloth,Plate FROM skyborne_item_armor_total WHERE ItemLevel=1"
                ).fetchone(),
                (18.5200005, 271.579987),
            )
            self.assertEqual(
                connection.execute(
                    "SELECT Quality_1 FROM skyborne_item_damage_one_hand WHERE ItemLevel=2"
                ).fetchone(),
                (1.01635063,),
            )
            self.assertEqual(
                connection.execute(
                    "SELECT Clothmodifier FROM skyborne_armor_location WHERE ID=1"
                ).fetchone(),
                (0.129999995,),
            )
            self.assertEqual(
                connection.execute(
                    "SELECT GoodF_0,SuperiorF_4,EpicF_2 FROM skyborne_rand_prop_points WHERE ID=1"
                ).fetchone(),
                (1.0, 1.0, 1.0),
            )
            manifest = json.loads((FIXTURE / "manifest.json").read_text())
            for table, name in [
                ("item_armor_quality", "ItemArmorQuality"),
                ("item_armor_total", "ItemArmorTotal"),
                ("item_armor_shield", "ItemArmorShield"),
                ("armor_location", "ArmorLocation"),
                ("item_damage_one_hand", "ItemDamageOneHand"),
                ("item_damage_one_hand_caster", "ItemDamageOneHandCaster"),
                ("item_damage_two_hand", "ItemDamageTwoHand"),
                ("item_damage_two_hand_caster", "ItemDamageTwoHandCaster"),
                ("rand_prop_points", "RandPropPoints"),
            ]:
                pin = manifest["sources"][name]
                output = manifest["outputs"][f"{name}.csv"]
                source_rows = connection.execute(
                    "SELECT row_id,build,file_data_id,layout_hash,sha256 FROM db2_row_source WHERE table_name=? ORDER BY row_id",
                    (f"skyborne_{table}",),
                ).fetchall()
                self.assertEqual(
                    source_rows,
                    [
                        (
                            row_id,
                            "1.60.1.70205",
                            pin["fdid"],
                            pin["layout"],
                            pin["raw_sha256"],
                        )
                        for row_id in output["row_ids"]
                    ],
                )
                self.assertEqual(
                    [
                        row[1]
                        for row in connection.execute(
                            f"PRAGMA table_info(skyborne_{table})"
                        )
                    ],
                    output["columns"],
                )
            self.assertEqual(
                connection.execute("SELECT payload FROM retail_bytes").fetchone(),
                (b"\x00\xffretail",),
            )
        # Every original schema declaration and SQL row remains byte-for-byte in dump.
        self.assertTrue(set(before).issubset(set(published)))
        again = self.invoke()
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertEqual(self.snapshot(), published)

    def test_missing_or_tampered_last_source_leaves_database_file_unchanged(self):
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        for kind in ("missing", "tampered"):
            with self.subTest(kind=kind):
                path = self.catalog / "RandPropPoints.csv"
                original = (FIXTURE / "RandPropPoints.csv").read_bytes()
                before = self.db.read_bytes()
                if kind == "missing":
                    path.unlink()
                else:
                    path.write_bytes(original + b"tampered")
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("RandPropPoints", result.stderr)
                self.assertEqual(self.db.read_bytes(), before)
                path.write_bytes(original)

    def test_bad_build_and_source_pins_fail_before_publication(self):
        original = (self.catalog / "manifest.json").read_bytes()
        for key in (
            "build",
            "fdid",
            "layout",
            "raw_sha256",
            "schema_sha256",
            "table_hash",
            "explicit_build",
        ):
            with self.subTest(key=key):

                def edit(manifest):
                    if key == "build":
                        manifest[key] = "12.1.0.69933"
                    else:
                        manifest["sources"]["RandPropPoints"][key] = "wrong"

                self.update_manifest(edit)
                before = self.db.read_bytes()
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.db.read_bytes(), before)
                (self.catalog / "manifest.json").write_bytes(original)

    def test_missing_used_kit_level_fails_even_with_consistent_manifest(self):
        self.update_manifest(
            lambda manifest: manifest.update(authored_item_levels=[1, 2])
        )
        self.rewrite_csv(
            "ItemDamageTwoHand",
            lambda rows: rows.__setitem__(
                slice(1, None), [row for row in rows[1:] if row[0] != "5"]
            ),
        )
        before = self.db.read_bytes()
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("5", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_missing_base_level_and_requested_level_fail(self):
        self.rewrite_csv(
            "ItemArmorQuality",
            lambda rows: rows.__setitem__(
                slice(1, None), [row for row in rows[1:] if row[0] != "2"]
            ),
        )
        before = self.db.read_bytes()
        result = self.invoke("--required-level", "101")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("2", result.stderr)
        self.assertIn("101", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_unknown_placeholder_column_and_nonfinite_value_rejected(self):
        for kind in ("column", "number"):
            with self.subTest(kind=kind):
                shutil.copyfile(
                    FIXTURE / "ArmorLocation.csv", self.catalog / "ArmorLocation.csv"
                )

                def edit(rows):
                    if kind == "column":
                        rows[0].append("Modifier")
                        for row in rows[1:]:
                            row.append("0")
                    else:
                        rows[1][1] = "NaN"

                self.rewrite_csv("ArmorLocation", edit)
                before = self.db.read_bytes()
                result = self.invoke()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("ArmorLocation", result.stderr)
                self.assertEqual(self.db.read_bytes(), before)

    def test_publication_failure_rolls_back_earlier_tables_and_provenance(self):
        with closing(sqlite3.connect(self.db)) as connection, connection:
            connection.execute(
                "CREATE VIEW skyborne_rand_prop_points AS SELECT 1 AS ID"
            )
        before = self.snapshot()
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.snapshot(), before)

    def test_absent_destination_is_not_created(self):
        self.db.unlink()
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.db.exists())


if __name__ == "__main__":
    unittest.main()
