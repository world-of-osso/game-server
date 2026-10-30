"""Source-backed Azure credit preparation on an independent database copy."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from azure_encounter_bindings import prepare_azure_bindings

ROOT = Path(__file__).resolve().parents[2]
DB2 = ROOT / 'data/db2/12.1.5.70448'


class AzureEncounterBindingsTests(unittest.TestCase):
    def test_copy_bindings_are_idempotent_and_preserve_other_encounters(self):
        fixtures = ROOT / 'target/encounter-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            with sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'azure.sqlite') as db:
                source.backup(db)
                before = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id != 2515 ORDER BY map_id,creature_entry').fetchall()
                expected = [(2515, 186644, 2582, 0, 2492, 5366, 107127),
                            (2515, 186739, 2585, 3, 2505, 5402, 106829),
                            (2515, 186737, 2583, 1, 2483, 5410, 109087),
                            (2515, 186738, 2584, 2, 2508, 5399, 106802)]
                with db:
                    self.assertEqual(prepare_azure_bindings(db, DB2), expected)
                    self.assertEqual(prepare_azure_bindings(db, DB2), expected)
                self.assertEqual(db.execute('SELECT map_id,creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2515 ORDER BY creature_entry').fetchall(), sorted(expected, key=lambda row: row[1]))
                self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE map_id != 2515 ORDER BY map_id,creature_entry').fetchall(), before)
                db.execute('UPDATE raid_encounter_creature SET bit=9 WHERE map_id=2515 AND creature_entry=186737')
                with self.assertRaisesRegex(ValueError, 'existing encounter binding conflicts'):
                    prepare_azure_bindings(db, DB2)


if __name__ == '__main__':
    unittest.main()
