import sqlite3
import unittest
from encounter_cpp_bindings import GOLDEN_PRIMARY, prepare_bindings, binding_sql


class PrimaryActorTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(':memory:')
        self.db.executescript('''
            CREATE TABLE dungeon_encounter(ID INTEGER,MapID INTEGER,Bit INTEGER);
            CREATE TABLE tdb_creature_template(entry INTEGER,ScriptName TEXT);
            CREATE TABLE raid_encounter_creature(map_id INTEGER,creature_entry INTEGER,
                encounter_id INTEGER,bit INTEGER,journal_encounter_id INTEGER,
                journal_creature_id INTEGER,display_id INTEGER,
                PRIMARY KEY(map_id,creature_entry));
            INSERT INTO dungeon_encounter VALUES(3101,2813,0);
            INSERT INTO tdb_creature_template VALUES(234648,'boss_kystia_manaheart');
            INSERT INTO tdb_creature_template VALUES(234660,'boss_kystia_manaheart_nibbles');
            INSERT INTO raid_encounter_creature VALUES(409,12118,663,0,0,0,0);
        ''')

    def tearDown(self):
        self.db.close()

    def test_primary_only_and_unrelated_binding_preserved_idempotently(self):
        binding = dict(map_id=2813,entry=234648,encounter_id=3101,bit=0,
                       script='boss_kystia_manaheart',journal_id=2679,actor_id=5847,display_id=124578)
        rows = prepare_bindings(self.db, [binding])
        sql = binding_sql(rows)
        self.db.executescript(sql)
        self.db.executescript(sql)
        self.assertEqual(self.db.execute('SELECT map_id,creature_entry,encounter_id,bit FROM raid_encounter_creature ORDER BY map_id').fetchall(),
                         [(409,12118,663,0),(2813,234648,3101,0)])
        self.assertEqual(self.db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=234660').fetchone()[0], 0)

    def test_golden_primary_does_not_grant_animated_gold_credit(self):
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(2139,1762,0);
            INSERT INTO tdb_creature_template VALUES(135322,'boss_the_golden_serpent');
            INSERT INTO tdb_creature_template VALUES(135406,'npc_animated_gold');
        ''')
        self.db.executescript(binding_sql(prepare_bindings(self.db, [GOLDEN_PRIMARY])))
        self.assertEqual(self.db.execute('SELECT creature_entry,encounter_id,bit FROM raid_encounter_creature WHERE map_id=1762').fetchall(), [(135322,2139,0)])
        self.assertEqual(self.db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=135406').fetchone()[0],0)
        self.assertEqual(self.db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(),[(12118,)])

    def test_source_identity_drift_refused_before_mutation(self):
        binding = dict(map_id=2813,entry=234648,encounter_id=3101,bit=1,
                       script='boss_kystia_manaheart',journal_id=2679,actor_id=5847,display_id=124578)
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [binding])
        self.assertEqual(self.db.execute('SELECT count(*) FROM raid_encounter_creature').fetchone()[0],1)


if __name__ == '__main__':
    unittest.main()
