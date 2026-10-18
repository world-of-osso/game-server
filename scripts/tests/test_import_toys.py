"""Selected real Toy rows must not replace the existing catalog."""
import csv
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CSV_ROOT = Path(os.environ.get("DB2_CSV_ROOT", ROOT.parent / "game-engine/data/db2"))
TOY_CSV = CSV_ROOT / "12.1.5.70448/Toy.csv"
SELECTED_ITEMS = (281136, 281615)
EXISTING_ITEM = 32782


class SelectedToyImportTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name)
        self.database = self.path / "world.db"
        self.report = self.path / "report.json"
        with TOY_CSV.open() as handle:
            self.source = {
                int(row["ItemID"]): row
                for row in csv.DictReader(handle)
                if int(row["ItemID"]) in (*SELECTED_ITEMS, EXISTING_ITEM)
            }
        self.assertEqual(set(self.source), {*SELECTED_ITEMS, EXISTING_ITEM})
        with sqlite3.connect(self.database) as conn:
            conn.executescript("""
                CREATE TABLE toy(ID INTEGER PRIMARY KEY, ItemID INTEGER UNIQUE,
                    Flags INTEGER, SourceTypeEnum INTEGER, SourceText_lang TEXT);
                CREATE TABLE item(ID INTEGER PRIMARY KEY);
                CREATE TABLE item_sparse(ID INTEGER PRIMARY KEY);
                CREATE TABLE item_x_item_effect(ItemID INTEGER, ItemEffectID INTEGER);
                CREATE TABLE item_effect(ID INTEGER, SpellID INTEGER,
                    TriggerType INTEGER, LegacySlotIndex INTEGER);
                CREATE TABLE spell_effect(SpellID INTEGER, DifficultyID INTEGER);
            """)
            row = self.source[EXISTING_ITEM]
            conn.execute("INSERT INTO toy VALUES(?,?,?,?,?)", self.values(row))
            for item, effect, spell in [(281136, 236737, 1310234), (281615, 236864, 1311934)]:
                conn.execute("INSERT INTO item VALUES(?)", (item,))
                conn.execute("INSERT INTO item_sparse VALUES(?)", (item,))
                conn.execute("INSERT INTO item_x_item_effect VALUES(?,?)", (item, effect))
                conn.execute("INSERT INTO item_effect VALUES(?,?,0,0)", (effect, spell))
                conn.execute("INSERT INTO spell_effect VALUES(?,0)", (spell,))

    @staticmethod
    def values(row):
        return (
            int(row["ID"]), int(row["ItemID"]), int(row["Flags"]),
            int(row["SourceTypeEnum"]), row["SourceText_lang"],
        )

    def run_import(self, *items):
        command = [
            "python3", str(ROOT / "scripts/import-toys.py"),
            str(TOY_CSV), str(self.database), "--report", str(self.report),
        ]
        for item in items:
            command.extend(["--item-id", str(item)])
        return subprocess.run(command, capture_output=True, text=True)

    def catalog(self):
        with sqlite3.connect(self.database) as conn:
            return conn.execute("SELECT * FROM toy ORDER BY ItemID").fetchall()

    def test_selected_hearthstones_are_source_exact_idempotent_and_preserve_catalog(self):
        expected = sorted(
            (self.values(row) for row in self.source.values()), key=lambda row: row[1]
        )
        for _ in range(2):
            result = self.run_import(*SELECTED_ITEMS)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(self.catalog(), expected)
            report = json.loads(self.report.read_text())
            self.assertEqual(report["counts"], {
                "toys": 2, "item": 2, "item_sparse": 2, "on_use": 2, "spell": 2,
            })
            self.assertEqual(report["unresolved"], [])

    def test_missing_requested_source_row_leaves_existing_catalog_unchanged(self):
        before = self.catalog()
        result = self.run_import(281136, 1)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("requested toy items absent from source: [1]", result.stderr)
        self.assertEqual(self.catalog(), before)
        self.assertFalse(self.report.exists())


if __name__ == "__main__":
    unittest.main()
