"""Sourced specialization/order data: staging import, floats and relationship joins."""
from contextlib import closing
import importlib
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

SOURCE = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-2026-10-11/db2')


class ProfessionDataTests(unittest.TestCase):
    def test_real_sources_preserve_numeric_values_and_relationships(self):
        mod = importlib.import_module('profession_data')
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / 'staging.db'
            with closing(sqlite3.connect(database)) as db, db:
                db.execute('CREATE TABLE untouched (value TEXT)')
                db.execute("INSERT INTO untouched VALUES ('keep')")
            report = mod.import_data(database, SOURCE)
            self.assertEqual([r['rows'] for r in report], [27, 91, 1403, 104, 15, 915, 5929, 138, 2047])
            with closing(sqlite3.connect(database)) as db:
                self.assertEqual(db.execute('SELECT CraftSkillBonusPercent, ReCraftSkillBonusPercent, ConcentrationSkillCurveID, ConcentrationDifficultyCurveID FROM crafting_difficulty WHERE ID=1').fetchone(), (25.0, 6.25, 77360, 77361))
                self.assertEqual(db.execute('SELECT "Order", CraftingQualityID, QualityPercentage, CraftingDifficultyID FROM crafting_difficulty_quality WHERE ID=2').fetchone(), (1, 2, 50.0, 1))
                self.assertEqual(db.execute('SELECT ItemID, ModifiedCraftingCategoryID FROM crafting_reagent_quality WHERE ID=1').fetchone(), (190099, 57))
                self.assertEqual(db.execute('SELECT TraitTreeID FROM prof_trait_tree WHERE ID=58').fetchone(), (239,))
                self.assertEqual(db.execute('SELECT RequiredPointsInThisNode, Flags FROM prof_trait_perk_node WHERE ID=2181').fetchone(), (5, 0))
                self.assertEqual(db.execute('SELECT SkillLineAbilityID, CraftingOrderHouseID FROM crafting_order WHERE ID=624').fetchone(), (48716, 1))
                self.assertEqual(db.execute('SELECT value FROM untouched').fetchone(), ('keep',))
                before = tuple(db.iterdump())
            self.assertEqual(report, mod.import_data(database, SOURCE))
            with closing(sqlite3.connect(database)) as db:
                self.assertEqual(tuple(db.iterdump()), before)

    def test_preflight_opens_no_database(self):
        mod = importlib.import_module('profession_data')
        with patch.object(sqlite3, 'connect', side_effect=AssertionError('DB opened')):
            report = mod.load_sources(SOURCE)[1]
        self.assertEqual(report[5]['rows'], 915)

    def test_corrupt_new_table_leaves_existing_database_untouched(self):
        mod = importlib.import_module('profession_data')
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source'
            source.mkdir()
            for path in SOURCE.glob('*.db2'):
                (source / path.name).write_bytes(path.read_bytes())
            database = Path(directory) / 'staging.db'
            mod.import_data(database, source)
            before = database.read_bytes()
            path = source / '4542084.db2'
            raw = bytearray(path.read_bytes())
            raw[-1] ^= 1
            path.write_bytes(raw)
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                mod.import_data(database, source)
            self.assertEqual(database.read_bytes(), before)

    def test_new_table_failure_rolls_back_prior_replacements(self):
        mod = importlib.import_module('profession_data')
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / 'staging.db'
            mod.import_data(database, SOURCE)
            with closing(sqlite3.connect(database)) as db, db:
                db.execute('DROP TABLE crafting_order')
                db.execute('CREATE VIEW crafting_order AS SELECT ID FROM profession')
                before = tuple(db.iterdump())
            with self.assertRaises(sqlite3.OperationalError):
                mod.import_data(database, SOURCE)
            with closing(sqlite3.connect(database)) as db:
                self.assertEqual(tuple(db.iterdump()), before)


if __name__ == '__main__':
    unittest.main()
