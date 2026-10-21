"""Kyrioss source producer changes only its explicit schedule binding."""
import sqlite3
import unittest

from encounter_summons import import_kyrioss_schedule


class KyriossScheduleTests(unittest.TestCase):
    def fixture(self):
        connection = sqlite3.connect(':memory:')
        connection.executescript('''
            CREATE TABLE tdb_creature_template(entry INTEGER, AIName TEXT, ScriptName TEXT);
            INSERT INTO tdb_creature_template VALUES(209230,'','');
            CREATE TABLE content_creature_template(entry INTEGER, AIName TEXT, ScriptName TEXT, HealthModifier REAL);
            INSERT INTO content_creature_template VALUES(209230,'','',35),(207205,'','',50);
            CREATE TABLE raid_encounter_creature(map_id INTEGER, creature_entry INTEGER, encounter_id INTEGER, bit INTEGER);
            INSERT INTO raid_encounter_creature VALUES(2648,209230,2816,0),(2648,207205,2861,2);
            CREATE TABLE content_creature_template_difficulty(entry INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER);
            INSERT INTO content_creature_template_difficulty VALUES(209230,1,2973);
            CREATE TABLE content_creature(guid INTEGER, id1 INTEGER, map INTEGER, spawnDifficulties TEXT);
            INSERT INTO content_creature VALUES(30001927,209230,2648,'1,2,8,23');
        ''')
        self.addCleanup(connection.close)
        return connection

    def test_repeatable_binding_preserves_native_difficulty_population_and_other_boss(self):
        connection = self.fixture()
        before = {table: connection.execute(f'SELECT * FROM {table}').fetchall()
                  for table in ['raid_encounter_creature', 'content_creature_template_difficulty', 'content_creature']}
        for _ in range(2):
            self.assertEqual(import_kyrioss_schedule(connection), [209230])
        self.assertEqual(connection.execute('SELECT * FROM content_creature_template ORDER BY entry').fetchall(),
                         [(207205, '', '', 50), (209230, '', 'littlewigs_kyrioss', 35)])
        for table, expected in before.items():
            self.assertEqual(connection.execute(f'SELECT * FROM {table}').fetchall(), expected)

    def test_conflicting_runtime_script_fails_without_replacing_it(self):
        connection = self.fixture()
        connection.execute("UPDATE content_creature_template SET ScriptName='another_owner' WHERE entry=209230")
        with self.assertRaisesRegex(ValueError, 'runtime AI identity conflicts'):
            import_kyrioss_schedule(connection)
        self.assertEqual(connection.execute('SELECT ScriptName FROM content_creature_template WHERE entry=209230').fetchone(), ('another_owner',))

    def test_changed_primary_binding_is_not_repaired_by_guessing(self):
        connection = self.fixture()
        connection.execute('UPDATE raid_encounter_creature SET bit=1 WHERE creature_entry=209230')
        with self.assertRaisesRegex(ValueError, 'native primary'):
            import_kyrioss_schedule(connection)
        self.assertEqual(connection.execute('SELECT ScriptName FROM content_creature_template WHERE entry=209230').fetchone(), ('',))


if __name__ == '__main__':
    unittest.main()
