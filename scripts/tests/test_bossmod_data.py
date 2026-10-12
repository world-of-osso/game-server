"""Observable static extraction and copy-only import contracts."""
import importlib.util
import sqlite3
from contextlib import closing
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bossmod_data as bm

SOURCE = '''local mod, CL = BigWigs:NewBoss("Cookie", 36, 2632)
mod:RegisterEnableMob(645)
mod:SetEncounterID(mod:Retail() and 2986 or 2746)
function mod:OnBossEnable()
 self:Log("SPELL_CAST_START", "AcidSplash", 6306)
end
function mod:OnEncounterStart()
 self:CDBar(6306, 2.4)
end
function mod:AcidSplash(args)
 self:Message(args.spellId, "orange")
 if self:Retail() then
  self:CDBar(args.spellId, 13.3)
 else
  self:CDBar(args.spellId, 55.0)
 end
end
'''

class BossmodImportTests(unittest.TestCase):
    def test_retail_engage_uses_encounter_start_not_legacy_on_engage(self):
        source = SOURCE.replace('OnEncounterStart', 'OnEngage') + '''
if mod:Retail() then
 function mod:OnEncounterStart()
  self:CDBar(6306, 5)
 end
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual([r['delay_ms'] for r in result['rules'] if r['trigger'] == 'engage'], [5000])

    def test_retail_method_replacement_removes_obsolete_cast_registration(self):
        source = SOURCE.replace('OnEncounterStart', 'OnEngage') + '''
if mod:Retail() then
 function mod:OnBossEnable()
 end
 function mod:OnEncounterStart()
  self:CDBar(6306, 5)
 end
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertFalse(any(r['trigger'] == 'cast_start' for r in result['rules']))
        self.assertEqual([r['delay_ms'] for r in result['rules'] if r['trigger'] == 'engage'], [5000])

    def test_retail_replacement_uses_final_handler_body_and_lexical_binding(self):
        source = SOURCE + '''
do
 local delay = 7
 function mod:AcidSplash(args)
  self:CDBar(args.spellId, delay)
 end
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual([r['delay_ms'] for r in result['rules'] if r['trigger'] == 'cast_start'], [7000])

    def test_static_timer_tables_calls_and_dbm_dispatch(self):
        source = '''local mod = DBM:NewMod(1, "DBM", 1)
mod:SetEncounterID(343)
mod:SetCreatureID(11490)
mod:SetZone(429)
local initial = 3.7
local timers = { [1] = { [22478] = 8.5 } }
local timerPain = mod:NewTimer(12, "Pain", nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, nil, 22478)
function mod:OnCombatStart()
 timerPain:Start(initial)
end
function mod:SPELL_CAST_START(args)
 local spellId = args.spellId
 if spellId == 22478 then
  timerPain:Start(timers[1][spellId])
 elseif spellId == 17228 then
  timerPain:Start(17)
 end
end
'''
        result = bm.extract_dbm(source, 'Zevrim.lua', 'DBM', 'pinned')
        self.assertEqual(result['map_id'], 429)
        self.assertEqual([(r['trigger'], r['trigger_spell_id'], r['delay_ms']) for r in result['rules']],
                         [('engage', None, 3700), ('cast_start', 22478, 8500), ('cast_start', 17228, 17000)])
        bw = SOURCE.replace('self:CDBar(6306, 2.4)', 'local timers = { [1] = { 2.4, 13.3 } }\n self:CDBar(6306, timers[1][1])')
        result = bm.extract_bigwigs(bw, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual(result['rules'][0]['delay_ms'], 2400)

    def test_duration_tables_preserve_lookup_path_without_inventing_cooldowns(self):
        source = SOURCE + '''do
 local timers = { [6306] = { [1] = { [40] = 31 }, [2] = { [49.5] = 80 } } }
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'BigWigs', 'pinned')
        self.assertEqual(result['timer_tables'], [dict(name='timers', source_line=19,
            values=[dict(lookup_path=[6306, 1, 40], seconds=31),
                    dict(lookup_path=[6306, 2, 49.5], seconds=80)])])
        self.assertEqual(len([r for r in result['rules'] if r['action'] == 'timer']), 2)

    def test_unknown_else_guards_and_mutable_counts_never_become_fixed_timers(self):
        guarded = SOURCE.replace('if self:Retail() then',
            'if self:IsTank() then\n  self:Message(args.spellId, "orange")\n elseif spellId == 6306 then')
        result = bm.extract_bigwigs(guarded, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual([r['delay_ms'] for r in result['rules'] if r['action']=='timer'], [2400])
        counted = 'local count = 1\nlocal timers = {2.4, 13.3}\n' + SOURCE.replace(
            'self:CDBar(args.spellId, 13.3)', 'count = count + 1\n  self:CDBar(args.spellId, timers[count])')
        result = bm.extract_bigwigs(counted, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual([r['delay_ms'] for r in result['rules'] if r['action']=='timer'], [2400])

    def test_handler_early_returns_and_branch_assignments(self):
        early = SOURCE.replace('self:CDBar(6306, 2.4)', 'if self:IsTank() then return end\n self:CDBar(6306, 2.4)')
        result = bm.extract_bigwigs(early, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertFalse(any(r['trigger']=='engage' and r['action']=='timer' for r in result['rules']))
        assigned = SOURCE.replace('self:CDBar(6306, 2.4)', 'local delay = 5\n if self:Retail() then delay = 10 end\n self:CDBar(6306, delay)')
        result = bm.extract_bigwigs(assigned, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual(result['rules'][0]['delay_ms'], 10000)

    def test_lexical_captures_table_keys_and_simultaneous_assignment(self):
        capture = 'local delay = 2\n' + SOURCE.replace('self:CDBar(6306, 2.4)', 'self:CDBar(6306, delay)') + 'local delay = 13\n'
        result = bm.extract_bigwigs(capture, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual(result['rules'][0]['delay_ms'], 2000)
        keyed = SOURCE.replace('self:CDBar(6306, 2.4)', 'local timers = {[1+1]=13}\n self:CDBar(6306, timers[1])')
        result = bm.extract_bigwigs(keyed, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertFalse(any(r['trigger']=='engage' and r['action']=='timer' for r in result['rules']))
        swapped = SOURCE.replace('self:CDBar(6306, 2.4)', 'local a,b=2,13\n a,b=b,a\n self:CDBar(6306,b)')
        result = bm.extract_bigwigs(swapped, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertEqual(result['rules'][0]['delay_ms'], 2000)

    def test_registered_events_follow_retail_guards_and_cannot_change_spell(self):
        guarded = SOURCE.replace('self:Log("SPELL_CAST_START", "AcidSplash", 6306)',
            'if self:Classic() then self:Log("SPELL_CAST_START", "AcidSplash", 6306) end')
        result = bm.extract_bigwigs(guarded, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertFalse(any(r['trigger']=='cast_start' for r in result['rules']))
        contradiction = SOURCE.replace('if self:Retail() then', 'if args.spellId == 17228 then')
        result = bm.extract_bigwigs(contradiction, 'Cookie.lua', 'LittleWigs', 'pinned')
        self.assertFalse(any(r['action']=='timer' and r['trigger']=='cast_start' for r in result['rules']))

    def test_locale_only_enus_assignments_override_default(self):
        source = 'L.warn="English"\nif GetLocale()=="deDE" then L.warn="Deutsch" end\nif GetLocale()=="enUS" then L.warn="US English" end'
        self.assertEqual(bm.locale_literals(source, 'L')['L.warn'], 'US English')
        default = 'L.warn="English"\nif GetLocale()=="deDE" then L.warn="Deutsch" end'
        self.assertEqual(bm.locale_literals(default, 'L')['L.warn'], 'English')

    def test_custom_dbm_timer_name_is_not_a_spell_identity(self):
        source = '''local mod=DBM:NewMod(1,"DBM",1)
mod:SetEncounterID(343)
mod:SetCreatureID(11490)
local timer = mod:NewTimer(12,22478)
function mod:OnCombatStart() timer:Start() end
'''
        result = bm.extract_dbm(source, 'Custom.lua', 'DBM', 'pinned')
        self.assertFalse(any(r['action']=='timer' for r in result['rules']))
        self.assertTrue(any('spellId' in g['reason'] for g in result['issues']))

    def test_unreachable_dynamic_timers_are_audited_not_scheduled(self):
        source = SOURCE + '''function mod:Dynamic(eventInfo)
 self:Bar(6306, eventInfo.duration)
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'BigWigs', 'pinned')
        self.assertTrue(any('eventInfo.duration' in gap['reason'] for gap in result['issues']))
        self.assertEqual(len([r for r in result['rules'] if r['action'] == 'timer']), 2)

    def test_enus_warning_format_uses_pinned_spell_name(self):
        result = bm.extract_bigwigs(SOURCE.replace('self:Message(args.spellId, "orange")',
            'self:Message(args.spellId, "orange", CL.casting:format(args.spellName))'),
            'Cookie.lua', 'LittleWigs', 'pinned')
        bm.render_warnings(result, {6306: 'Acid Splash'}, {'CL.casting': 'Casting %s'})
        warning = next(r for r in result['rules'] if r['action'] == 'warning')
        self.assertEqual(warning['warning_text'], 'Casting Acid Splash')
        self.assertEqual(warning['warning_key'], 'CL.casting:format(args.spellName)')
        unresolved = bm.extract_bigwigs(SOURCE.replace('self:Message(args.spellId, "orange")',
            'self:Message(args.spellId, "orange", L.missing)'), 'Cookie.lua', 'LittleWigs', 'pinned')
        bm.render_warnings(unresolved, {6306: 'Acid Splash'}, {})
        self.assertFalse(any(r['action'] == 'warning' for r in unresolved['rules']))
        self.assertTrue(any('locale' in gap['reason'] for gap in unresolved['issues']))

    def test_exboss_declared_timeline_not_runtime_mutations(self):
        source = '''local TRIGGER_AI = "AI"
local encounterTriggers = { [343] = {trigger=TRIGGER_AI, durationRules={{time=31,eventID=12}}} }
for encounterID,config in pairs(encounterTriggers) do
 encounterTriggers[encounterID].trigger = NormalizeTrigger(config.trigger)
end
error("addon code must never run")
'''
        self.assertEqual(bm.extract_exboss_timeline(source), {343: {'trigger':'AI', 'durationRules':{1:{'time':31,'eventID':12}}}})

    def test_exboss_missing_times_are_per_encounter_gaps(self):
        source = '''local EncounterData = { maps = { [429] = {bosses = { [343] = {
 events = { [12] = {spellID = 22478, castType = "begincast", firstSeenSec = nil, cdSeriesSec = nil} }
 } }} } }'''
        result = bm.extract_exboss(source, 'EncounterData.lua', 'EXBossData', 'pinned')[0]
        self.assertTrue(any('firstSeenSec' in gap['reason'] for gap in result['issues']))
        self.assertFalse(any(r['action'] == 'timer' for r in result['rules']))

    def test_retail_initial_repeat_warning_and_provenance(self):
        result = bm.extract_bigwigs(SOURCE, "Cookie.lua", "LittleWigs", "abc123")
        self.assertEqual(result['encounter_id'], 2986)
        self.assertEqual(result['npcs'], [645])
        self.assertEqual(result['map_id'], 36)
        rules = result['rules']
        self.assertEqual([(r['trigger'], r['spell_id'], r['delay_ms']) for r in rules if r['action'] == 'timer'],
                         [('engage', 6306, 2400), ('cast_start', 6306, 13300)])
        warning = next(r for r in rules if r['action'] == 'warning')
        self.assertEqual((warning['spell_id'], warning['severity'], warning['warning_key']), (6306, 1, 'spellName'))
        self.assertEqual((warning['source_file'], warning['source_line'], warning['version']), ('Cookie.lua', 11, 'abc123'))

    def test_unknown_conditions_never_become_unconditional_timers(self):
        result = bm.extract_bigwigs(SOURCE.replace('self:Retail() then\n  self:CDBar', 'self:Mythic() then\n  self:CDBar'), 'Cookie.lua', 'LittleWigs', 'abc123')
        timers = [r for r in result['rules'] if r['action'] == 'timer']
        self.assertEqual([r['delay_ms'] for r in timers], [2400])
        self.assertTrue(result['issues'])

    def test_health_phase_threshold_is_not_an_initial_cooldown(self):
        source = SOURCE + '''function mod:UNIT_HEALTH(_, unit)
 if self:GetHealth(unit) < 35 then
  self:SetStage(2)
  self:CDBar(6306, 5)
 end
end
'''
        result = bm.extract_bigwigs(source, 'Cookie.lua', 'LittleWigs', 'abc123')
        phase = next(r for r in result['rules'] if r['action'] == 'phase')
        self.assertEqual((phase['trigger'], phase['health_pct'], phase['phase']), ('health', 35, 2))
        timer = next(r for r in result['rules'] if r.get('health_pct') == 35 and r['action'] == 'timer')
        self.assertEqual(timer['delay_ms'], 5000)

    def test_sole_schedule_authority_and_no_arbitrary_addon_priority(self):
        cookie = bm.extract_bigwigs(SOURCE, 'Cookie.lua', 'LittleWigs', 'abc123')
        self.assertEqual(bm.schedule_authorities([cookie]), {2986: 'LittleWigs'})
        other = dict(cookie, addon='DBM')
        with self.assertRaisesRegex(ValueError, '2986'):
            bm.schedule_authorities([cookie, other])

    def test_spell_validation_and_copy_preserve_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp)/'world.db', Path(tmp)/'imported.db'
            with closing(sqlite3.connect(source)) as db:
                db.executescript('CREATE TABLE spell(ID INTEGER); CREATE TABLE spell_name(ID INTEGER,Name_lang TEXT); INSERT INTO spell VALUES(6306); INSERT INTO spell_name VALUES(6306,"Acid Splash"); CREATE TABLE marker(value TEXT); INSERT INTO marker VALUES("unchanged"); CREATE TABLE dungeon_encounter(ID INTEGER,MapID INTEGER,Bit INTEGER); INSERT INTO dungeon_encounter VALUES(2986,36,7); CREATE TABLE tdb_creature(id INTEGER,map INTEGER); INSERT INTO tdb_creature VALUES(645,36); CREATE TABLE db2_build(table_name TEXT,build TEXT); INSERT INTO db2_build VALUES("spell","12.1.0.69933"),("spell_name","12.1.0.69933");')
            original = source.read_bytes()
            result = bm.extract_bigwigs(SOURCE, 'Cookie.lua', 'LittleWigs', 'abc123')
            bm.write_copy(source, target, [result])
            self.assertEqual(source.read_bytes(), original)
            with closing(sqlite3.connect(target)) as db:
                self.assertEqual(db.execute('SELECT count(*) FROM bossmod_rule').fetchone()[0], 3)
                self.assertEqual(db.execute('SELECT value FROM marker').fetchone()[0], 'unchanged')
                self.assertEqual(db.execute('SELECT * FROM bossmod_binding').fetchall(), [(36, 645, 2986, 7)])
                self.assertEqual(db.execute('SELECT * FROM bossmod_authority').fetchall(), [(2986, 'LittleWigs')])
            with self.assertRaises(ValueError): bm.write_copy(source, source, [result])
            result['rules'][0]['spell_id'] = 99999999
            with self.assertRaisesRegex(ValueError, '99999999'): bm.write_copy(source, Path(tmp)/'bad.db', [result])

if __name__ == '__main__': unittest.main()
