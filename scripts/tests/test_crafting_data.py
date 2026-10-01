"""Bounded crafting import: real WDC5 fixtures, CSVs and staging-only integration."""

import csv
from contextlib import closing
import hashlib
import importlib
import io
import json
from pathlib import Path
import sqlite3
import struct
import subprocess
import sys
import tempfile
import unittest
from dataclasses import replace
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SOURCE = SCRIPTS.parent / "data/reference/crafting/12.1.0.69933"


def wdc5(layout, values, *, noninline=False, parent=None, encrypted=False):
    fields = values[1:] if noninline else values
    size = len(fields) * 4
    offset = 204 + 40 + len(fields) * (4 + 24)
    ids = struct.pack("<I", values[0]) if noninline else b""
    relations = (
        struct.pack("<5I", 1, values[0], values[0], parent, 0)
        if parent is not None
        else b""
    )
    header = bytearray(204)
    header[:4] = b"WDC5"
    struct.pack_into(
        "<9I", header, 136, 1, len(fields), size, 0, 0, layout, values[0], values[0], 0
    )
    struct.pack_into("<HH", header, 172, 4 if noninline else 0, 0)
    struct.pack_into("<7I", header, 176, len(fields), 0, 0, len(fields) * 24, 0, 0, 1)
    section = struct.pack(
        "<Q8I",
        int(encrypted),
        offset,
        1,
        0,
        offset + size,
        len(ids),
        len(relations),
        0,
        0,
    )
    infos = b"".join(
        struct.pack("<HHIIIII", i * 32, 32, 0, 0, 0, 0, 0) for i in range(len(fields))
    )
    return (
        bytes(header)
        + section
        + b"\0" * (4 * len(fields))
        + infos
        + struct.pack(f"<{len(fields)}I", *fields)
        + ids
        + relations
    )


class FixtureImportTests(unittest.TestCase):
    def setUp(self):
        self.mod = importlib.import_module("crafting_data")
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.db = self.root / "staging.db"
        self.source = self.root / "source"
        self.source.mkdir()
        floats = [
            struct.unpack("<I", struct.pack("<f", f))[0]
            for f in [1.5, 2.5, 3.5, 4.5, 5.5, 6.5]
        ]
        (self.source / "4545611.db2").write_bytes(
            wdc5(0xC21C854F, [1, 0, 2, 190645, 3, 30, *floats, 4, 5, 6, 7])
        )
        (self.source / "4545612.db2").write_bytes(
            wdc5(0xDE11A051, [1, 190646], noninline=True, parent=1)
        )
        (self.source / "3390956.db2").write_bytes(
            wdc5(
                0x00739B08,
                [15388, 1237486, 0, 421, 0, 140, 80],
                noninline=True,
                parent=1237486,
            )
        )
        csvs = {
            "ModifiedCraftingReagentSlot": (
                [
                    "Name_lang",
                    "ID",
                    "Flags",
                    "PlayerConditionID",
                    "ReagentType",
                    "ReagentSource",
                    "Field_11_2_0_61476_006",
                    "Field_12_0_0_63534_007",
                ],
                ["Material", 421, 0, 0, 1, 2, 0, 1.25],
            ),
            "ModifiedCraftingCategory": (
                [
                    "ID",
                    "DisplayName_lang",
                    "Description_lang",
                    "Field_9_0_1_33978_001",
                    "MatQualityWeight",
                    "Field_10_0_0_44649_004",
                ],
                [10, "Cloth", "Quality cloth", 0, 2.5, 0],
            ),
            "ModifiedCraftingReagentItem": (
                [
                    "ID",
                    "Description_lang",
                    "ModifiedCraftingCategoryID",
                    "ItemBonusTreeID",
                    "Flags",
                    "Field_9_1_0_38511_004",
                    "ItemContextOffset",
                ],
                [20, "Cloth reagent", 10, 30, 0, 0, 0],
            ),
            "MCRSlotXMCRCategory": (
                [
                    "ID",
                    "ModifiedCraftingCategoryID",
                    "_Order",
                    "ModifiedCraftingReagentSlotID",
                ],
                [1, 10, 0, 421],
            ),
        }
        for name, (header, row) in csvs.items():
            with (self.source / (name + ".csv")).open("w", newline="") as f:
                writer = csv.writer(f)
                writer.writerow(header)
                writer.writerow(row)
        self.pins = tuple(
            replace(
                t,
                count=1,
                sha256=hashlib.sha256(
                    (self.source / t.filename).read_bytes()
                ).hexdigest(),
            )
            for t in self.mod.TABLES
        )
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute("CREATE TABLE untouched (value TEXT)")
            conn.execute("INSERT INTO untouched VALUES ('preserve me')")

    def import_fixture(self):
        with patch.object(self.mod, "TABLES", self.pins):
            return self.mod.import_data(self.db, self.source)

    def snapshot(self):
        with closing(sqlite3.connect(self.db)) as conn, conn:
            return tuple(conn.iterdump())

    def repin(self, filename):
        digest = hashlib.sha256((self.source / filename).read_bytes()).hexdigest()
        self.pins = tuple(
            replace(t, sha256=digest) if t.filename == filename else t
            for t in self.pins
        )

    def test_check_validates_sources_without_opening_database(self):
        before = self.db.read_bytes()
        with (
            patch.object(self.mod, "TABLES", self.pins),
            patch.object(
                sys,
                "argv",
                ["crafting_data.py", "--check", "--source-dir", str(self.source)],
            ),
            patch.object(
                self.mod.sqlite3, "connect", side_effect=AssertionError("DB opened")
            ),
        ):
            with patch("sys.stdout", new_callable=io.StringIO) as output:
                self.mod.main()
            report = json.loads(output.getvalue())
        self.assertEqual([row["rows"] for row in report], [1] * 7)
        self.assertEqual(self.db.read_bytes(), before)

    def test_check_rejects_layout_header_hash_count_and_missing_source(self):
        cases = (
            (
                "4545611.db2",
                lambda b: b[:156] + struct.pack("<I", 0xBAD) + b[160:],
                "layout",
            ),
            (
                "4545611.db2",
                lambda b: b[:136] + struct.pack("<I", 2) + b[140:],
                "count",
            ),
            (
                "ModifiedCraftingCategory.csv",
                lambda b: b.replace(b"DisplayName_lang", b"WrongHeader"),
                "header",
            ),
            (
                "ModifiedCraftingCategory.csv",
                lambda b: b.replace(b"Quality cloth", b"Other cloth"),
                "SHA-256",
            ),
            ("MCRSlotXMCRCategory.csv", None, "MCRSlotXMCRCategory.csv"),
        )
        before = self.db.read_bytes()
        for filename, mutate, error in cases:
            with self.subTest(error=error):
                path = self.source / filename
                original = path.read_bytes()
                if mutate is None:
                    path.unlink()
                else:
                    path.write_bytes(mutate(original))
                try:
                    with (
                        patch.object(self.mod, "TABLES", self.pins),
                        patch.object(
                            sys,
                            "argv",
                            [
                                "crafting_data.py",
                                "--check",
                                "--source-dir",
                                str(self.source),
                            ],
                        ),
                        patch.object(
                            self.mod.sqlite3,
                            "connect",
                            side_effect=AssertionError("DB opened"),
                        ),
                    ):
                        with patch("sys.stderr", new_callable=io.StringIO) as output:
                            with self.assertRaises(SystemExit) as exit_result:
                                self.mod.main()
                        self.assertEqual(exit_result.exception.code, 1)
                        self.assertIn(error, output.getvalue())
                    self.assertEqual(self.db.read_bytes(), before)
                finally:
                    path.write_bytes(original)

    def test_decoder_float_parent_and_join(self):
        report = self.import_fixture()
        with closing(sqlite3.connect(self.db)) as conn, conn:
            self.assertEqual(
                conn.execute(
                    "SELECT CraftedItemID, Field_10_0_0_44649_005, CraftSkillBonusPercent, Field_10_0_0_45141_011 FROM crafting_data"
                ).fetchone(),
                (190645, 1.5, 2.5, 6.5),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT q.ItemID, d.CraftedItemID FROM crafting_data_item_quality q JOIN crafting_data d ON d.ID=q.CraftingDataID"
                ).fetchall(),
                [(190646, 190645)],
            )
            self.assertEqual(
                conn.execute(
                    "SELECT s.SpellID, s.ReagentCount, s.ReagentReCraftCount, i.ID FROM modified_crafting_spell_slot s JOIN mcr_slot_x_mcr_category x ON x.ModifiedCraftingReagentSlotID=s.ModifiedCraftingReagentSlotID JOIN modified_crafting_category c ON c.ID=x.ModifiedCraftingCategoryID JOIN modified_crafting_reagent_item i ON i.ModifiedCraftingCategoryID=c.ID"
                ).fetchall(),
                [(1237486, 140, 80, 20)],
            )
        self.assertEqual(len(report), 7)
        self.assertTrue(
            all(
                r["build"] == "12.1.0.69933"
                and r["rows"] == 1
                and r["sha256"]
                and r["layout"]
                for r in report
            )
        )
        before = self.snapshot()
        self.assertEqual(report, self.import_fixture())
        self.assertEqual(before, self.snapshot())

    def test_reject_bad_sources_without_changing_existing_tables(self):
        self.import_fixture()
        before = self.snapshot()
        cases = [
            (
                "4545611.db2",
                lambda b: b[:156] + struct.pack("<I", 0xBAD) + b[160:],
                "layout",
            ),
            (
                "4545611.db2",
                lambda b: b[:136] + struct.pack("<I", 2) + b[140:],
                "count",
            ),
            ("3390956.db2", lambda b: b[:-8] + struct.pack("<II", 999, 0), "parent"),
            (
                "4545611.db2",
                lambda b: b[:204] + struct.pack("<Q", 1) + b[212:],
                "encrypted",
            ),
            (
                "ModifiedCraftingReagentSlot.csv",
                lambda b: b.replace(b"Name_lang", b"WrongName"),
                "header",
            ),
            (
                "ModifiedCraftingCategory.csv",
                lambda b: b.replace(b"2.5", b"nan"),
                "finite",
            ),
            (
                "ModifiedCraftingCategory.csv",
                lambda b: b.replace(b"2.5", b"bad"),
                "number",
            ),
            (
                "ModifiedCraftingCategory.csv",
                lambda b: b.replace(b"Quality cloth", b"Other cloth"),
                "SHA-256",
            ),
        ]
        for filename, mutate, error in cases:
            with self.subTest(error=error):
                path = self.source / filename
                original = path.read_bytes()
                path.write_bytes(mutate(original))
                with self.assertRaisesRegex(ValueError, error):
                    self.import_fixture()
                self.assertEqual(before, self.snapshot())
                path.write_bytes(original)

    def test_insert_failure_rolls_back_schema_and_rows(self):
        self.import_fixture()
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute(
                "CREATE VIEW dependent AS SELECT ID FROM modified_crafting_reagent_item"
            )
            conn.execute(
                "CREATE TRIGGER prevent_drop BEFORE DELETE ON untouched BEGIN SELECT RAISE(ABORT, 'no'); END"
            )
        # An existing view with a target table name makes CREATE TABLE fail after earlier replacements.
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute("DROP TABLE mcr_slot_x_mcr_category")
            conn.execute(
                "CREATE VIEW mcr_slot_x_mcr_category AS SELECT ID FROM modified_crafting_category"
            )
        before = self.snapshot()
        with self.assertRaises(sqlite3.OperationalError):
            self.import_fixture()
        self.assertEqual(before, self.snapshot())

    def test_extra_csv_row_and_missing_cells_fail_closed(self):
        self.import_fixture()
        before = self.snapshot()
        p = self.source / "MCRSlotXMCRCategory.csv"
        original = p.read_bytes()
        for contents, message in [
            (original + b"2,10,1,421\n", "count"),
            (original.replace(b"1,10,0,421", b"1,10,0"), "width"),
        ]:
            p.write_bytes(contents)
            with self.assertRaisesRegex(ValueError, message):
                self.import_fixture()
            self.assertEqual(before, self.snapshot())
        p.write_bytes(original)

    def test_duplicate_csv_ids_are_not_suppressed(self):
        self.import_fixture()
        before = self.snapshot()
        p = self.source / "MCRSlotXMCRCategory.csv"
        p.write_bytes(p.read_bytes() + b"1,10,1,421\n")
        self.repin(p.name)
        self.pins = tuple(
            replace(t, count=2) if t.filename == p.name else t for t in self.pins
        )
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.import_fixture()
        self.assertEqual(before, self.snapshot())


class RealSourceTests(unittest.TestCase):
    def test_pinned_cli_check_without_database_argument(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "crafting_data.py"),
                    "--check",
                    "--source-dir",
                    str(SOURCE),
                ],
                cwd=tmp,
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(
                [r["rows"] for r in report], [2785, 1687, 12769, 393, 598, 623, 910]
            )
            self.assertEqual(list(Path(tmp).iterdir()), [])

    def test_check_rejects_database_argument(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "sentinel.db"
            db.write_bytes(b"do not touch")
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "crafting_data.py"),
                    "--check",
                    "--db",
                    str(db),
                    "--source-dir",
                    str(SOURCE),
                ],
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(db.read_bytes(), b"do not touch")

    def test_pinned_cli_staging_import_and_real_joins(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "staging.db"
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPTS / "crafting_data.py"),
                    "--db",
                    str(db),
                    "--source-dir",
                    str(SOURCE),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertEqual(
                [r["rows"] for r in report], [2785, 1687, 12769, 393, 598, 623, 910]
            )
            with closing(sqlite3.connect(db)) as conn, conn:
                self.assertEqual(
                    conn.execute(
                        "SELECT SpellID, ReagentCount, ReagentReCraftCount FROM modified_crafting_spell_slot WHERE ID=15388"
                    ).fetchone(),
                    (1237486, 140, 80),
                )
                self.assertGreater(
                    conn.execute(
                        "SELECT COUNT(*) FROM crafting_data_item_quality q JOIN crafting_data d ON d.ID=q.CraftingDataID"
                    ).fetchone()[0],
                    1000,
                )
                self.assertGreater(
                    conn.execute(
                        "SELECT COUNT(*) FROM modified_crafting_spell_slot s JOIN modified_crafting_reagent_slot r ON r.ID=s.ModifiedCraftingReagentSlotID JOIN mcr_slot_x_mcr_category x ON x.ModifiedCraftingReagentSlotID=r.ID JOIN modified_crafting_category c ON c.ID=x.ModifiedCraftingCategoryID JOIN modified_crafting_reagent_item i ON i.ModifiedCraftingCategoryID=c.ID"
                    ).fetchone()[0],
                    1000,
                )
                self.assertEqual(
                    conn.execute(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'"
                    ).fetchone()[0],
                    7,
                )

    def test_cli_requires_explicit_paths(self):
        result = subprocess.run(
            [sys.executable, str(SCRIPTS / "crafting_data.py")],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--db", result.stderr)
        self.assertIn("--source-dir", result.stderr)


if __name__ == "__main__":
    unittest.main()
