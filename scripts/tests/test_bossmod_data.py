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
function mod:OnEngage()
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

    def test_spell_validation_and_copy_preserve_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            source, target = Path(tmp)/'world.db', Path(tmp)/'imported.db'
            with closing(sqlite3.connect(source)) as db:
                db.executescript('CREATE TABLE spell(ID INTEGER); CREATE TABLE spell_name(ID INTEGER,Name_lang TEXT); INSERT INTO spell VALUES(6306); INSERT INTO spell_name VALUES(6306,"Acid Splash"); CREATE TABLE marker(value TEXT); INSERT INTO marker VALUES("unchanged"); CREATE TABLE db2_build(table_name TEXT,build TEXT); INSERT INTO db2_build VALUES("spell","12.1.0.69933"),("spell_name","12.1.0.69933");')
            original = source.read_bytes()
            result = bm.extract_bigwigs(SOURCE, 'Cookie.lua', 'LittleWigs', 'abc123')
            bm.write_copy(source, target, [result])
            self.assertEqual(source.read_bytes(), original)
            with closing(sqlite3.connect(target)) as db:
                self.assertEqual(db.execute('SELECT count(*) FROM bossmod_rule').fetchone()[0], 3)
                self.assertEqual(db.execute('SELECT value FROM marker').fetchone()[0], 'unchanged')
            with self.assertRaises(ValueError): bm.write_copy(source, source, [result])
            result['rules'][0]['spell_id'] = 99999999
            with self.assertRaisesRegex(ValueError, '99999999'): bm.write_copy(source, Path(tmp)/'bad.db', [result])

if __name__ == '__main__': unittest.main()
