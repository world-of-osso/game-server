import importlib.util
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


derive = load_module('derive_wcl_creature_abilities')
emit = load_module('emit_wcl_creature_ability_sql')

SPELL_DB2 = """
CREATE TABLE spell_cooldowns(SpellID INTEGER,DifficultyID INTEGER,RecoveryTime INTEGER,CategoryRecoveryTime INTEGER,StartRecoveryTime INTEGER);
CREATE TABLE spell_misc(SpellID INTEGER,DifficultyID INTEGER,CastingTimeIndex INTEGER,DurationIndex INTEGER);
CREATE TABLE spell_cast_times(ID INTEGER,Base INTEGER);
CREATE TABLE spell_duration(ID INTEGER,Duration INTEGER);
CREATE TABLE spell_effect(SpellID INTEGER,DifficultyID INTEGER,EffectIndex INTEGER,Effect INTEGER,EffectAura INTEGER,ImplicitTarget_0 INTEGER,ImplicitTarget_1 INTEGER);
INSERT INTO spell_cast_times VALUES(1,0);
INSERT INTO spell_duration VALUES(21,-1),(18,20000);
-- Rotten Ground 1297701: periodic damage + screen effect on the target, never expiring.
INSERT INTO spell_misc VALUES(1297701,0,1,21);
INSERT INTO spell_effect VALUES(1297701,0,0,6,3,25,0),(1297701,0,1,6,260,25,0);
-- Same aura shape with Rotten Ground's 20-second area duration.
INSERT INTO spell_misc VALUES(500001,0,1,18);
INSERT INTO spell_effect VALUES(500001,0,0,6,3,25,0);
-- Never-expiring aura the caster puts on itself.
INSERT INTO spell_misc VALUES(500002,0,1,21);
INSERT INTO spell_effect VALUES(500002,0,0,6,3,1,0);
"""


def write_report(directory, casts):
    prefix = Path(directory) / 'den-report-1'
    actors = {'reportData': {'report': {'masterData': {'actors': [
        {'id': 966, 'type': 'NPC', 'subType': 'NPC', 'gameID': 241813},
        {'id': 963, 'type': 'Player', 'gameID': 0},
        {'id': 964, 'type': 'Player', 'gameID': 0},
        {'id': 1, 'type': 'Pet', 'gameID': 0},
    ]}}}}
    Path(f'{prefix}-actors.json').write_text(json.dumps(actors))
    Path(f'{prefix}-friendly-damage.jsonl').write_text(json.dumps({'targetID': 966, 'targetInstance': 10, 'fight_ms': 50000}) + '\n')
    Path(f'{prefix}-damage.jsonl').write_text('')
    rows = [
        {'type': kind, 'sourceID': 966, 'sourceInstance': 10, 'targetID': target, 'abilityGameID': spell, 'fight_ms': time}
        for kind, spell, time, target in casts
    ]
    Path(f'{prefix}-casts.jsonl').write_text(''.join(json.dumps(row) + '\n' for row in rows))
    return prefix


def derive_rows(casts):
    db = sqlite3.connect(':memory:')
    db.executescript(SPELL_DB2)
    results = {}
    with tempfile.TemporaryDirectory() as directory:
        derive.collect_report(write_report(directory, casts), 'VxKH96NhkT487pnC/18', results, db)
    spells, refused = derive.split_refused(db, results)
    return {row['spell']: row for row in spells}, {row['spell']: row['reasons'] for row in refused}


class WclCreatureAbilityTests(unittest.TestCase):
    def test_same_timestamp_multi_target_cast_is_one_event_per_actor(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory) / 'den-report-1'
            actors = {'reportData': {'report': {'masterData': {'actors': [
                {'id': 11, 'type': 'NPC', 'subType': 'NPC', 'gameID': 241813},
                {'id': 1, 'type': 'Player', 'gameID': 100},
            ]}}}}
            Path(f'{prefix}-actors.json').write_text(json.dumps(actors))
            def save(kind, rows):
                Path(f'{prefix}-{kind}.jsonl').write_text(''.join(json.dumps(row) + '\n' for row in rows))
            save('friendly-damage', [{'targetID': 11, 'targetInstance': 1, 'fight_ms': 100}])
            save('damage', [])
            save('casts', [
                {'type': 'cast', 'sourceID': 11, 'sourceInstance': 1, 'targetID': target, 'abilityGameID': 1297701, 'fight_ms': time}
                for time, target in [(500, 1), (500, 2), (1500, 1), (1500, 2), (2200, 1)]
            ])
            results = {}
            db = sqlite3.connect(':memory:')
            db.executescript('CREATE TABLE spell_cooldowns(SpellID INTEGER,DifficultyID INTEGER,RecoveryTime INTEGER,CategoryRecoveryTime INTEGER,StartRecoveryTime INTEGER); CREATE TABLE spell_misc(SpellID INTEGER,DifficultyID INTEGER,CastingTimeIndex INTEGER); CREATE TABLE spell_cast_times(ID INTEGER,Base INTEGER);')
            derive.collect_report(prefix, 'ReportA/1', results, db)
            row = results[(241813, 1297701)]
            self.assertEqual([x['delay_ms'] for x in row['delay_proxy_ms']], [400])
            self.assertEqual([x['interval_ms'] for x in row['interval_ms']], [1000, 700])

    def test_refuses_timer_for_never_expiring_aura_on_players(self):
        # Spaced WCL casts on players: no area-entry cluster, only the DB2 duration differs.
        casts = [('cast', spell, time, target) for spell in (1297701, 500001, 500002)
                 for time, target in [(51659, 963), (54021, 964), (56840, 963)]]
        spells, refused = derive_rows(casts)
        self.assertEqual(set(refused), {1297701})
        self.assertEqual([reason['reason'] for reason in refused[1297701]], ['non_expiring_aura_on_enemy'])
        self.assertEqual([(effect['effect_index'], effect['aura']) for effect in refused[1297701][0]['effects']], [(0, 3), (1, 260)])
        self.assertEqual(set(spells), {500001, 500002})
        self.assertEqual([sample['interval_ms'] for sample in spells[500001]['interval_ms']], [2362, 2819])

    def test_refuses_area_entry_signature_without_begincast(self):
        # VxKH96NhkT487pnC/18 instance 10: 963 at 51659, then 1 and 964 at 51660.
        entry = [(51659, 963), (51660, 1), (51660, 964), (69404, 963), (69404, 964)]
        spells, refused = derive_rows([('cast', 500001, time, target) for time, target in entry])
        self.assertEqual(list(refused), [500001])
        self.assertEqual(refused[500001][0]['reason'], 'area_entry_signature')
        self.assertEqual([(c['fight_ms'], c['separation_ms']) for c in refused[500001][0]['clusters']],
                         [(51659, 1), (51660, 0), (69404, 0)])
        self.assertEqual(spells, {})

        cast_bar = [('begincast', 500001, 50159, 963)] + [('cast', 500001, time, target) for time, target in entry]
        spells, refused = derive_rows(cast_bar)
        self.assertEqual(refused, {})
        self.assertEqual(list(spells), [500001])

        same_target_or_spaced = [(51659, 963), (51660, 963), (51681, 964)]
        spells, refused = derive_rows([('cast', 500001, time, target) for time, target in same_target_or_spaced])
        self.assertEqual(refused, {})
        self.assertEqual(list(spells), [500001])

    def test_db2_recovery_and_cast_time_collapse_short_intervals(self):
        db = sqlite3.connect(':memory:')
        db.executescript('CREATE TABLE spell_cooldowns(SpellID INTEGER,DifficultyID INTEGER,RecoveryTime INTEGER,CategoryRecoveryTime INTEGER,StartRecoveryTime INTEGER); CREATE TABLE spell_misc(SpellID INTEGER,DifficultyID INTEGER,CastingTimeIndex INTEGER); CREATE TABLE spell_cast_times(ID INTEGER,Base INTEGER); INSERT INTO spell_cooldowns VALUES(123,0,900,1200,300); INSERT INTO spell_misc VALUES(123,0,7); INSERT INTO spell_cast_times VALUES(7,300); INSERT INTO spell_misc VALUES(1297701,0,1); INSERT INTO spell_cast_times VALUES(1,0);')
        bound = derive.read_spell_bound(db, 123)
        self.assertEqual(bound['minimum_interval_ms'], 1500)
        self.assertEqual(derive.successive_cast_intervals([(1000,1),(1001,2),(2499,3),(2500,4),(4000,1)], 1500), [1500,1500])
        missing = derive.read_spell_bound(db, 1297701)
        self.assertIsNone(missing['recovery_time_ms'])
        self.assertEqual(missing['cast_time_ms'], 0)
        self.assertEqual(missing['minimum_interval_ms'], 0)
        self.assertEqual(derive.successive_cast_intervals([(1000,1),(1001,2)], 0), [1])

    def test_sql_rejects_zero_repeat_before_writing_samples(self):
        abilities = [{'entry': 241813, 'spell': 1297701, 'target_type': 5}]
        sample = {'entry': 241813, 'spell': 1297701,
                  'delay_proxy_ms': [{'report': 'ReportA/1', 'actor': [11, 1], 'delay_ms': 400}],
                  'interval_ms': [{'report': 'ReportA/1', 'actor': [11, 1], 'interval_ms': 1000}]}
        sql = emit.render_sql(2825, abilities, [sample])
        db = sqlite3.connect(':memory:')
        db.executescript("""CREATE TABLE content_creature_template(entry INTEGER,AIName TEXT,ScriptName TEXT);
          INSERT INTO content_creature_template VALUES(241813,'','');
          CREATE TABLE tdb_smart_scripts(entryorguid INTEGER,source_type INTEGER);
          CREATE TABLE spell_effect(SpellID INTEGER); INSERT INTO spell_effect VALUES(1297701);
          CREATE TABLE content_inferred_creature_ability(map INTEGER,difficulty INTEGER,entry INTEGER,spell INTEGER,target_type INTEGER,provenance TEXT,retirement_condition TEXT);
          CREATE TABLE content_inferred_creature_ability_sample(map INTEGER,difficulty INTEGER,entry INTEGER,spell INTEGER,sample_id INTEGER,timing_kind TEXT,delay_ms INTEGER,report TEXT,fight INTEGER,source_actor INTEGER,source_instance INTEGER);""")
        bad_sql = sql.replace("'successive_cast',1000,", "'successive_cast',0,")
        self.assertNotEqual(sql, bad_sql)
        with self.assertRaises(sqlite3.IntegrityError):
            db.executescript(bad_sql)
        db.rollback()
        self.assertEqual(db.execute('SELECT count(*) FROM content_inferred_creature_ability').fetchone()[0], 0)
        db.executescript(sql)
        self.assertEqual(db.execute('SELECT count(*) FROM content_inferred_creature_ability_sample').fetchone()[0], 4)
        sample['interval_ms'][0]['interval_ms'] = 0
        with self.assertRaisesRegex(ValueError, 'successive_cast delay_ms=0'):
            emit.render_sql(2825, abilities, [sample])

    def test_new_world_schema_rejects_zero_successive_delay(self):
        db = sqlite3.connect(':memory:')
        schema = (ROOT / 'world_schema.sql').read_text().split('DROP TABLE IF EXISTS game_event_creature;')[0]
        db.executescript(schema)
        db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES(2825,8,241813,1297701,1,'first_activity_proxy',0,'ReportA',1,11,1)")
        with self.assertRaises(sqlite3.IntegrityError):
            db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES(2825,8,241813,1297701,2,'successive_cast',0,'ReportA',1,11,1)")

    def test_existing_database_guard_rejects_zero_on_insert_and_update(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE content_inferred_creature_ability_sample(timing_kind TEXT NOT NULL,delay_ms INTEGER NOT NULL)')
        db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES('successive_cast',0)")
        db.commit()
        guard = (ROOT / 'guard_inferred_creature_ability_samples.sql').read_text()
        with self.assertRaises(sqlite3.IntegrityError):
            db.executescript(guard)
        db.rollback()
        self.assertEqual(db.execute("SELECT count(*) FROM sqlite_master WHERE type='trigger'").fetchone()[0], 0)
        db.execute('DELETE FROM content_inferred_creature_ability_sample')
        db.commit()
        db.executescript(guard)
        db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES('first_activity_proxy',0)")
        db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES('successive_cast',1000)")
        with self.assertRaisesRegex(sqlite3.IntegrityError, 'invalid inferred ability sample timing'):
            db.execute("INSERT INTO content_inferred_creature_ability_sample VALUES('successive_cast',0)")
        with self.assertRaisesRegex(sqlite3.IntegrityError, 'invalid inferred ability sample timing'):
            db.execute("UPDATE content_inferred_creature_ability_sample SET delay_ms=0 WHERE timing_kind='successive_cast'")


if __name__ == '__main__':
    unittest.main()
