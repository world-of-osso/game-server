"""Journal-bound hostility is an explicit project admission, never a native faction rewrite."""
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placements import prepare_templates, read_binding_tables
from follower_placement_sources import read_db2
from scripts.tests.test_followup_placements import copy_native_actor_fixture

POLICY = 'FOLLOWER_BOSS_HOSTILE_FROM_JOURNAL_ENCOUNTER'


@unittest.skipUnless('MIDNIGHT_SOURCE_DB' in os.environ and 'FOLLOWER_PLACEMENT_DB2' in os.environ,
                     'requires explicit native70448 source')
class JournalHostility(unittest.TestCase):
    def setUp(self):
        self.source = sqlite3.connect(Path(os.environ['MIDNIGHT_SOURCE_DB']).resolve().as_uri() + '?mode=ro', uri=True)
        self.addCleanup(self.source.close)
        self.tables, _ = read_db2(Path(os.environ['FOLLOWER_PLACEMENT_DB2']), ('MapDifficulty',))
        binding, _ = read_binding_tables(Path(os.environ['FOLLOWER_PLACEMENT_DB2']))
        self.tables.update(binding)
        self.record = dict(entry=231864, map_id=2811, faction_policy=POLICY,
                           faction_donor=188252, donor_reason='No same-map native faction16 donor; faction only.',
                           content_tuning_policy='FOLLOWER_PLACEMENT_CONTENT_TUNING_FROM_NATIVE_MAP_DIFFICULTY',
                           journal_actor_source=dict(policy='FOLLOWER_BINDING_EXACT_NATIVE_NAME_DISPLAY_TO_JOURNAL_ACTOR',
                                                     journal_encounter_id=2660, journal_actor_id=5982))

    def fixture(self):
        target = copy_native_actor_fixture(self.source, 231864)
        for table, where in (('map_difficulty', 'MapID=2811'), ('content_tuning_level', 'ID=3026')):
            if table == 'map_difficulty':
                target.execute(self.source.execute('SELECT sql FROM sqlite_master WHERE name=?', (table,)).fetchone()[0])
            rows = self.source.execute(f'SELECT * FROM {table} WHERE {where}').fetchall()
            target.executemany(f"INSERT INTO {table} VALUES({','.join('?' for _ in rows[0])})", rows)
        rows = self.source.execute('SELECT * FROM tdb_creature_template WHERE entry=188252').fetchall()
        target.executemany(f"INSERT INTO tdb_creature_template VALUES({','.join('?' for _ in rows[0])})", rows)
        return target

    def test_native_zero_gets_only_faction16_with_hashed_journal_chain(self):
        with closing(self.fixture()) as target:
            donors, difficulties = prepare_templates(target, [self.record], Path(os.environ['FOLLOWER_PLACEMENT_CACHE']), self.tables)
            self.assertEqual(target.execute('SELECT faction,ContentTuningID,HealthModifier FROM content_creature_template WHERE entry=231864').fetchone(), (16, 0, self.source.execute('SELECT HealthModifier FROM tdb_creature_template_difficulty WHERE Entry=231864 AND DifficultyID=0').fetchone()[0]))
            self.assertEqual(target.execute('SELECT faction FROM tdb_creature_template WHERE entry=231864').fetchone(), (0,))
            self.assertEqual(difficulties[0]['source_difficulty_id'], 0)
            proof = donors[0]['hostility_source']
            self.assertEqual(proof['policy'], POLICY)
            self.assertEqual({name: row['row']['ID'] for name, row in proof['rows'].items()},
                             dict(JournalEncounterCreature='5982', JournalEncounter='2660', DungeonEncounter='3073', MapDifficulty='5610'))
            for row in proof['rows'].values():
                self.assertEqual(row['sha256'], hashlib.sha256(json.dumps(row['row'], sort_keys=True, separators=(',', ':')).encode()).hexdigest())

    def test_nonzero_native_faction_is_never_overridden(self):
        with closing(self.fixture()) as target:
            target.execute('UPDATE tdb_creature_template SET faction=14 WHERE entry=231864')
            donors, _ = prepare_templates(target, [self.record], Path('/unused'), self.tables)
            self.assertEqual(donors, [])
            self.assertEqual(target.execute('SELECT faction FROM content_creature_template WHERE entry=231864').fetchone(), (14,))

    def test_ambiguous_actor_and_nonfollower_map_are_refused(self):
        for mutation in ('duplicate', 'nonfollower', 'wrong_map', 'donor_not16'):
            with self.subTest(mutation=mutation), closing(self.fixture()) as target:
                tables = copy.deepcopy(self.tables)
                if mutation == 'duplicate':
                    tables['JournalEncounterCreature'].append(dict(next(r for r in tables['JournalEncounterCreature'] if r['ID'] == '5982'), ID='0'))
                elif mutation == 'nonfollower':
                    tables['MapDifficulty'] = [r for r in tables['MapDifficulty'] if not (r['MapID'] == '2811' and r['DifficultyID'] == '205')]
                elif mutation == 'wrong_map':
                    next(r for r in tables['DungeonEncounter'] if r['ID'] == '3073')['MapID'] = '2993'
                else:
                    target.execute('UPDATE tdb_creature_template SET faction=14 WHERE entry=188252')
                with self.assertRaises(ValueError):
                    prepare_templates(target, [self.record], Path(os.environ['FOLLOWER_PLACEMENT_CACHE']), tables)


if __name__ == '__main__':
    unittest.main()
