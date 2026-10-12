from contextlib import closing
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))


class CraftingOrdersImportTests(unittest.TestCase):
    def test_copy_import_keeps_source_and_selects_only_fixed_outputs(self):
        import crafting_orders_import as importer
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, output, csv_path = root / 'source.db', root / 'copy.db', root / 'orders.csv'
            with closing(sqlite3.connect(source)) as db:
                db.executescript('CREATE TABLE skill_line_ability (ID INT, Spell INT, SkillLine INT); CREATE TABLE spell_effect (SpellID INT, Effect INT, DifficultyID INT, EffectItemType INT, EffectBasePointsF REAL); INSERT INTO skill_line_ability VALUES(10,100,164),(20,200,165); INSERT INTO spell_effect VALUES(100,24,0,500,2),(200,288,0,600,1);')
                db.commit()
            before = source.read_bytes()
            csv_path.write_text('ID,SkillLineAbilityID,CraftingOrderHouseID,Field_10_0_2_46479_002\n1,10,1,0\n2,20,1,0\n')
            receipt = importer.import_copy(source, output, csv_path)
            self.assertEqual(source.read_bytes(), before)
            self.assertEqual(receipt['fixed_output_rows'], 1)
            with closing(sqlite3.connect(output)) as db:
                self.assertEqual(db.execute('SELECT ID,SkillLineAbilityID FROM crafting_order ORDER BY ID').fetchall(), [(1,10),(2,20)])
            with self.assertRaises(ValueError):
                importer.import_copy(source, source, csv_path)
            with self.assertRaises(FileExistsError):
                importer.import_copy(source, output, csv_path)

    def test_export_authentic_payload(self):
        import export_db2_csv
        raw = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/crafting-2026-10-11/db2/4529122.db2')
        rows = export_db2_csv.crafting_order_rows(raw.read_bytes())
        self.assertEqual(len(rows), 2047)
        self.assertTrue(all(len(row) == 4 and row[0] > 0 for row in rows))
        with self.assertRaises(ValueError):
            export_db2_csv.crafting_order_rows(b'invalid')


if __name__ == '__main__':
    unittest.main()
