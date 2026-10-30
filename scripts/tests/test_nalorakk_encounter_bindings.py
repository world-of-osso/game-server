import csv
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from nalorakk_encounter_bindings import prepare_bindings


class NalorakkBindingsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.db2 = Path(self.tmp.name)
        for name, fields, rows in (
            ('JournalEncounter.csv', ('ID', 'DungeonEncounterID'), [(2776, 3207), (2777, 3208), (2778, 3209)]),
            ('JournalEncounterCreature.csv', ('ID', 'JournalEncounterID', 'CreatureDisplayInfoID'),
             [(5941, 2776, 129344), (5948, 2777, 129418), (5978, 2778, 129989)]),
        ):
            with (self.db2 / name).open('w', newline='') as output:
                writer = csv.writer(output)
                writer.writerow(fields)
                writer.writerows(rows)
        self.db = sqlite3.connect(':memory:')
        self.addCleanup(self.db.close)
        self.db.executescript('''
            CREATE TABLE dungeon_encounter (ID INTEGER, MapID INTEGER, Bit INTEGER);
            INSERT INTO dungeon_encounter VALUES (3207,2825,0),(3208,2825,1),(3209,2825,2);
            CREATE TABLE content_creature (map INTEGER, id1 INTEGER);
            INSERT INTO content_creature VALUES (2825,241812),(2825,244100),(2825,246404);
            CREATE TABLE raid_encounter_creature (
                map_id INTEGER, creature_entry INTEGER, encounter_id INTEGER, bit INTEGER,
                journal_encounter_id INTEGER, journal_creature_id INTEGER, display_id INTEGER,
                UNIQUE(map_id, creature_entry));
        ''')

    def test_imports_observed_bosses_idempotently(self):
        prepare_bindings(self.db, self.db2)
        prepare_bindings(self.db, self.db2)
        self.assertEqual(self.db.execute('SELECT * FROM raid_encounter_creature ORDER BY bit').fetchall(), [
            (2825, 241812, 3207, 0, 2776, 5941, 129344),
            (2825, 244100, 3208, 1, 2777, 5948, 129418),
            (2825, 246404, 3209, 2, 2778, 5978, 129989),
        ])

    def test_conflicting_binding_fails_without_partial_import(self):
        self.db.execute('INSERT INTO raid_encounter_creature VALUES (2825,244100,999,1,2777,5948,129418)')
        with self.assertRaisesRegex(ValueError, 'binding conflicts'):
            prepare_bindings(self.db, self.db2)
        self.assertEqual(self.db.execute('SELECT creature_entry FROM raid_encounter_creature').fetchall(), [(244100,)])


if __name__ == '__main__':
    unittest.main()
