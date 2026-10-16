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

    def test_silverlaine_smartai_primary_is_idempotent_and_does_not_admit_unported_bosses(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('silverlaine')
        self.assertIsNotNone(profile, 'pinned Shadowfang primary is missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(1070,33,1);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (3887,'','SmartAI'),(46962,'',''),(4278,'',''),(46963,'',''),(46964,'','');
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=33').fetchall(),
            [(3887,1070,1,97,367,37288)])
        self.assertEqual(self.db.execute(
            'SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(), [(12118,)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=3887")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])

    def test_electrocutioner_primary_keeps_db2_bit_not_tc_ordinal(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('electrocutioner')
        self.assertIsNotNone(profile, 'source Electrocutioner primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(380,90,3);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (6235,'','SmartAI'),(6229,'','SmartAI'),(7800,'','SmartAI');
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=90').fetchall(),
            [(6235,380,3,421,991,36558)])
        self.assertEqual(self.db.execute(
            'SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(), [(12118,)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=6235")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET AIName='SmartAI' WHERE entry=6235")
        self.db.execute('UPDATE dungeon_encounter SET Bit=2 WHERE ID=380')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_pummeler_primary_keeps_db2_bit_and_existing_gnomeregan_credit(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('crowd-pummeler')
        self.assertIsNotNone(profile, 'source Crowd Pummeler primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(381,90,4),(380,90,3);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (6229,'','SmartAI'),(6235,'','SmartAI'),(7800,'','SmartAI');
            INSERT INTO raid_encounter_creature VALUES(90,6235,380,3,421,991,36558);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=90 ORDER BY bit').fetchall(),
            [(6235,380,3,421,991,36558),(6229,381,4,418,989,36560)])
        self.assertEqual(self.db.execute(
            'SELECT creature_entry FROM raid_encounter_creature WHERE map_id=409').fetchall(), [(12118,)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=6229")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET AIName='SmartAI' WHERE entry=6229")
        self.db.execute('UPDATE dungeon_encounter SET Bit=3 WHERE ID=381')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_razorlash_primary_preserves_other_credit_and_authored_ai(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('razorlash')
        self.assertIsNotNone(profile, 'source Razorlash primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(423,349,1);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (12258,'','SmartAI'),(12236,'','SmartAI');
            INSERT INTO raid_encounter_creature VALUES(90,7800,382,5,422,992,36563);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349').fetchall(),
            [(12258,423,1,424,994,12389)])
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit FROM raid_encounter_creature WHERE map_id=90').fetchall(),
            [(7800,382,5)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=12258")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET AIName='SmartAI' WHERE entry=12258")
        self.db.execute('UPDATE dungeon_encounter SET Bit=2 WHERE ID=423')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_gizlock_primary_uses_db2_bit_not_tc_ordinal(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('gizlock')
        self.assertIsNotNone(profile, 'pinned Gizlock primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(427,349,5);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES(13601,'','SmartAI');
            INSERT INTO raid_encounter_creature VALUES(349,13282,422,0,423,993,11172);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(13282,422,0,423,993,11172),(13601,427,5,425,995,7125)])
        self.db.execute('UPDATE dungeon_encounter SET Bit=2 WHERE ID=427')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_noxxion_primary_preserves_other_maraudon_credit(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('noxxion')
        self.assertIsNotNone(profile, 'pinned Noxxion primary missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(422,349,0);
            INSERT INTO tdb_creature_template VALUES(13282,'boss_noxxion');
            INSERT INTO raid_encounter_creature VALUES(349,13596,428,6,430,1000,13589);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(13282,422,0,423,993,11172),(13596,428,6,430,1000,13589)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=13282")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])

    def test_rotgrip_primary_preserves_razorlash_and_authored_ai(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('rotgrip')
        self.assertIsNotNone(profile, 'source Rotgrip primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(428,349,6);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (13596,'','SmartAI');
            INSERT INTO raid_encounter_creature VALUES(349,12258,423,1,424,994,12389);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(12258,423,1,424,994,12389),(13596,428,6,430,1000,13589)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=13596")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET AIName='SmartAI' WHERE entry=13596")
        self.db.execute('UPDATE dungeon_encounter SET Bit=5 WHERE ID=428')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_vyletongue_primary_keeps_bit_two_not_source_ordinal_three(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('vyletongue')
        self.assertIsNotNone(profile, 'source Vyletongue primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(424,349,2);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (12236,'','SmartAI'),(12225,'boss_celebras_the_cursed','');
            INSERT INTO raid_encounter_creature VALUES(349,13601,427,5,425,995,7125);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(12236,424,2,427,997,12334),(13601,427,5,425,995,7125)])
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=12236")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET AIName='SmartAI' WHERE entry=12236")
        self.db.execute('UPDATE dungeon_encounter SET Bit=3 WHERE ID=424')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_celebras_primary_keeps_bit_three_and_excludes_redeemed_and_summons(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('celebras')
        self.assertIsNotNone(profile, 'source Celebras primary missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(425,349,3);
            INSERT INTO tdb_creature_template(entry,ScriptName) VALUES
                (12225,'boss_celebras_the_cursed'),(13716,''),(13743,'');
            INSERT INTO raid_encounter_creature VALUES(349,12236,424,2,427,997,12334);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(12236,424,2,427,997,12334),(12225,425,3,428,998,12350)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=12225")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='boss_celebras_the_cursed' WHERE entry=12225")
        self.db.execute('UPDATE dungeon_encounter SET Bit=4 WHERE ID=425')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_landslide_primary_keeps_bit_four_not_ordinal_five(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('landslide')
        self.assertIsNotNone(profile, 'source Landslide primary missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(426,349,4);
            INSERT INTO tdb_creature_template(entry,ScriptName) VALUES
                (12203,'boss_landslide'),(11783,'');
            INSERT INTO raid_encounter_creature VALUES(349,12225,425,3,428,998,12350);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(12225,425,3,428,998,12350),(12203,426,4,429,999,12293)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=12203")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='boss_landslide' WHERE entry=12203")
        self.db.execute('UPDATE dungeon_encounter SET Bit=5 WHERE ID=426')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_theradras_primary_keeps_bit_seven_and_excludes_spirit(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('theradras')
        self.assertIsNotNone(profile, 'source Theradras primary missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(429,349,7);
            INSERT INTO tdb_creature_template(entry,ScriptName) VALUES
                (12201,'boss_princess_theradras'),(12238,'');
            INSERT INTO raid_encounter_creature VALUES(349,12203,426,4,429,999,12293);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=349 ORDER BY bit').fetchall(),
            [(12203,426,4,429,999,12293),(12201,429,7,431,1001,12292)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=12201")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='boss_princess_theradras' WHERE entry=12201")
        self.db.execute('UPDATE dungeon_encounter SET Bit=6 WHERE ID=429')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
            prepare_bindings(self.db, [primary])

    def test_thermaplugg_primary_preserves_bosses_and_excludes_walking_bombs(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('thermaplugg')
        self.assertIsNotNone(profile, 'source Thermaplugg primary missing')
        primary, _ = profile
        self.db.executescript('''
            ALTER TABLE tdb_creature_template ADD COLUMN AIName TEXT DEFAULT '';
            INSERT INTO dungeon_encounter VALUES(382,90,5);
            INSERT INTO tdb_creature_template(entry,ScriptName,AIName) VALUES
                (7800,'','SmartAI'),(7915,'','SmartAI');
            INSERT INTO raid_encounter_creature VALUES(90,6229,381,4,418,989,36560);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=90 ORDER BY bit').fetchall(),
            [(6229,381,4,418,989,36560),(7800,382,5,422,992,36563)])
        self.assertEqual(self.db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=7915').fetchone()[0], 0)
        self.db.execute("UPDATE tdb_creature_template SET AIName='' WHERE entry=7800")
        with self.assertRaisesRegex(ValueError, 'creature AI identity'):
            prepare_bindings(self.db, [primary])

    def test_journal_csv_authenticates_exact_encounter_and_actor(self):
        import encounter_cpp_bindings as bindings
        import tempfile
        from pathlib import Path
        primary = dict(entry=7800,encounter_id=382,journal_id=422,actor_id=992,display_id=36563)
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / 'JournalEncounter.csv'
            actors = Path(directory) / 'JournalEncounterCreature.csv'
            journal.write_text('ID,DungeonEncounterID\n422,382\n')
            actors.write_text('ID,JournalEncounterID,CreatureDisplayInfoID\n992,422,36563\n')
            bindings.validate_journal(journal, actors, primary)
            actors.write_text('ID,JournalEncounterID,CreatureDisplayInfoID\n992,422,36560\n')
            with self.assertRaisesRegex(ValueError, 'journal primary actor'):
                bindings.validate_journal(journal, actors, primary)
            journal.write_text('ID,DungeonEncounterID\n422,381\n')
            with self.assertRaisesRegex(ValueError, 'journal encounter'):
                bindings.validate_journal(journal, actors, primary)

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

    def test_gargolmar_primary_uses_db2_bit_not_source_ordinal_and_excludes_companions(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('gargolmar')
        self.assertIsNotNone(profile, 'pinned Watchkeeper Gargolmar primary is missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(1893,543,2);
            INSERT INTO tdb_creature_template VALUES(17306,'boss_watchkeeper_gargolmar');
            INSERT INTO raid_encounter_creature VALUES(349,12201,429,7,431,1001,12292);
        ''')
        rows = prepare_bindings(self.db, [primary])
        for _ in range(2):
            self.db.executescript(binding_sql(rows))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=543').fetchall(),
            [(17306,1893,2,527,1104,18236)])
        self.assertEqual(self.db.execute('SELECT creature_entry FROM raid_encounter_creature WHERE map_id=349').fetchall(), [(12201,)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=17306")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])

    def test_omor_primary_uses_installed_bit_zero_and_keeps_gargolmar(self):
        import encounter_cpp_bindings as bindings
        profile = bindings.PROFILES.get('omor')
        self.assertIsNotNone(profile, 'pinned Omor primary is missing')
        primary, _ = profile
        self.db.executescript('''
            INSERT INTO dungeon_encounter VALUES(1891,543,0);
            INSERT INTO tdb_creature_template VALUES(17308,'boss_omor_the_unscarred'),(17540,'');
            INSERT INTO raid_encounter_creature VALUES(543,17306,1893,2,527,1104,18236);
        ''')
        for _ in range(2):
            self.db.executescript(binding_sql(prepare_bindings(self.db, [primary])))
        self.assertEqual(self.db.execute(
            'SELECT creature_entry,encounter_id,bit,journal_encounter_id,journal_creature_id,display_id '
            'FROM raid_encounter_creature WHERE map_id=543 ORDER BY bit').fetchall(),
            [(17308,1891,0,528,1105,18237),(17306,1893,2,527,1104,18236)])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='' WHERE entry=17308")
        with self.assertRaisesRegex(ValueError, 'creature script identity'):
            prepare_bindings(self.db, [primary])
        self.db.execute("UPDATE tdb_creature_template SET ScriptName='boss_omor_the_unscarred' WHERE entry=17308")
        self.db.execute('UPDATE dungeon_encounter SET Bit=1 WHERE ID=1891')
        with self.assertRaisesRegex(ValueError, 'encounter identity'):
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
