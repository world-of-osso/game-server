import sqlite3
import unittest
from encounter_cpp_bindings import (ADAROGG_PRIMARY, GOLDEN_PRIMARY, GORDOTH_PRIMARY, KORANTHAL_PRIMARY,
                                    prepare_bindings, binding_sql)


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

    def test_cobrahn_smartai_primary_requires_its_authored_ai(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('cobrahn')
        self.assertIsNotNone(profile, 'Wailing source primary is not admitted')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(586,43,1);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES(3669,'','SmartAI');
        ''')
        rows = prepare_bindings(self.db, [primary])
        self.db.executescript(binding_sql(rows))
        self.db.executescript(binding_sql(rows))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=43').fetchall(),
            [(3669,586,1,475,1046,4213)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=3669")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])

    def test_wailing_primaries_keep_all_eight_authored_normal_bits(self):
        import encounter_cpp_bindings as bindings
        names = ['anacondra', 'cobrahn', 'kresh', 'pythas', 'skum', 'serpentis', 'verdan', 'mutanus']
        missing = [name for name in names if name not in bindings.PROFILES]
        self.assertEqual(missing, [], 'Wailing source identities missing')
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(585,43,0),(586,43,1),(587,43,2),
                (588,43,3),(589,43,4),(590,43,5),(591,43,6),(592,43,7);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (3671,'','SmartAI'),(3669,'','SmartAI'),(3653,'',''),(3670,'','SmartAI'),
                (3674,'','SmartAI'),(3673,'','SmartAI'),(5775,'','SmartAI'),(3654,'','SmartAI');
        ''')
        rows = prepare_bindings(self.db, [bindings.PROFILES[name][0] for name in names])
        for _ in range(2):
            self.db.executescript(binding_sql(rows))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit FROM raid_encounter_creature WHERE map_id=43 ORDER BY bit'
        ).fetchall(), [(3671,585,0),(3669,586,1),(3653,587,2),(3670,588,3),
                      (3674,589,4),(3673,590,5),(5775,591,6),(3654,592,7)])
        self.assertEqual(self.db.execute(
            'SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(), [(12118,)])

    def test_slagmaw_source_primary_keeps_journal_preview_separate_from_live_model(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('slagmaw')
        self.assertIsNotNone(profile, 'pinned NPC_SLAGMAW primary is missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(1445,389,2);
            INSERT INTO tdb_creature_template VALUES(61463,'boss_slagmaw');
            CREATE TABLE tdb_creature_template_model(CreatureID INTEGER,CreatureDisplayID INTEGER);
            INSERT INTO tdb_creature_template_model VALUES(61463,42247);
        ''')
        rows = prepare_bindings(self.db, [primary])
        for _ in range(2):
            self.db.executescript(binding_sql(rows))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=389').fetchall(),
            [(61463,1445,2,696,1344,38391)])
        self.assertEqual(self.db.execute('SELECT * FROM tdb_creature_template_model').fetchall(), [(61463,42247)])
        self.assertEqual(self.db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(),[(12118,)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=61463")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])

    def test_source_identity_drift_refused_before_mutation(self):
        binding = dict(map_id=2813,entry=234648,encounter_id=3101,bit=1,
                       script='boss_kystia_manaheart',journal_id=2679,actor_id=5847,display_id=124578)
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [binding])
        self.assertEqual(self.db.execute('SELECT count(*) FROM raid_encounter_creature').fetchone()[0],1)

    def test_ragefire_primaries_bind_source_bits_beside_existing_koranthal(self):
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(1443,389,0),(1444,389,1),(1446,389,3);
            INSERT INTO tdb_creature_template VALUES(61408,'boss_adarogg'),
                (61412,'boss_dark_shaman_koranthal'),(61528,'boss_lava_guard_gordoth');
            INSERT INTO raid_encounter_creature VALUES(389,61412,1444,1,695,1321,42101);
        ''')
        rows = prepare_bindings(self.db, [ADAROGG_PRIMARY, GORDOTH_PRIMARY, KORANTHAL_PRIMARY])
        self.db.executescript(binding_sql(rows))
        self.assertEqual(self.db.execute('SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=389 ORDER BY bit').fetchall(),
                         [(61408,1443,0,694,1343,38759),(61412,1444,1,695,1321,42101),(61528,1446,3,697,1345,42100)])


if __name__ == '__main__':
    unittest.main()
