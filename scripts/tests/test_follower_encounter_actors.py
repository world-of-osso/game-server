import csv
import sqlite3
import tempfile
import unittest
from pathlib import Path

from scripts.follower_encounter_actors import encounter_actor_bits


class EncounterActors(unittest.TestCase):
    def test_unique_journal_participant_is_joined_without_granting_credit(self):
        db = sqlite3.connect(':memory:')
        db.executescript('''
            CREATE TABLE raid_encounter_creature(map_id, journal_encounter_id, bit, creature_entry);
            CREATE TABLE tdb_creature(map, id);
            CREATE TABLE tdb_creature_template_model(CreatureID, CreatureDisplayID);
            INSERT INTO raid_encounter_creature VALUES(2813,2679,0,234648);
            INSERT INTO tdb_creature VALUES(2813,234648),(2813,234660),(2813,999), (2813,1000);
            INSERT INTO tdb_creature_template_model VALUES(234648,124578),(234660,126199),
                                                          (999,100),(1000,100);
        ''')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'journal.csv'
            with path.open('w', newline='') as stream:
                writer = csv.writer(stream)
                writer.writerow(['JournalEncounterID', 'CreatureDisplayInfoID'])
                writer.writerows([(2679,124578),(2679,126199),(2679,100)])
            self.assertEqual(encounter_actor_bits({'map_id': 2813}, db, path),
                             {234648: 0, 234660: 0})
        self.assertEqual(db.execute('SELECT creature_entry FROM raid_encounter_creature').fetchall(),
                         [(234648,)])
