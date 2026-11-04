import importlib.util
import sqlite3
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    'healer_wcl_debuffs', Path(__file__).resolve().parents[1] / 'healer_wcl_debuffs.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class DebuffIntakeTest(unittest.TestCase):
    def test_source_qualified_periodic_children_and_reapplication(self):
        db = sqlite3.connect(':memory:')
        db.executescript('CREATE TABLE spell_categories(SpellID, DifficultyID, DispelType);'
                         'INSERT INTO spell_categories VALUES(1294569,0,1),(17,0,0);'
                         'CREATE TABLE spell_effect(SpellID,DifficultyID,EffectTriggerSpell);'
                         'INSERT INTO spell_effect VALUES(1294569,0,42),(42,0,1294569);')
        base = {'targetID': 1, 'sourceID': 10, 'sourceInstance': 2, 'abilityGameID': 1294569}
        events = [dict(base, type='applydebuff', fight_ms=100),
                  dict(base, type='applydebuffstack', fight_ms=150, stack=2),
                  dict(base, type='refreshdebuff', fight_ms=200),
                  dict(base, type='removedebuff', fight_ms=500),
                  dict(base, type='applydebuff', fight_ms=600),
                  dict(base, type='applydebuff', fight_ms=0, abilityGameID=17)]
        tick = {'target_actor': 1, 'source_actor': 10, 'source_instance': 2,
                'spell_id': 42, 'periodic': True, 'time_ms': 300}
        hits = [dict(tick), dict(tick, source_instance=3), dict(tick, target_actor=2),
                dict(tick, periodic=False), dict(tick, time_ms=550), dict(tick, time_ms=700)]
        episodes = module.convert(events, hits, db, [1, 2, 3, 4, 5], 1000)
        self.assertEqual(len(episodes), 2)
        self.assertEqual(episodes[0]['end_ms'], 500)
        self.assertEqual([update['stacks'] for update in episodes[0]['updates']], [1, 2, 2])
        self.assertEqual([hit.get('debuff_episode') for hit in hits], [0, None, None, None, None, 1])

    def test_ambiguous_overlapping_parent_spells_fail(self):
        db = sqlite3.connect(':memory:')
        db.executescript('CREATE TABLE spell_categories(SpellID, DifficultyID, DispelType);'
                         'INSERT INTO spell_categories VALUES(10,0,1),(11,0,1);'
                         'CREATE TABLE spell_effect(SpellID,DifficultyID,EffectTriggerSpell);'
                         'INSERT INTO spell_effect VALUES(10,0,12),(11,0,12);')
        events = [{'targetID': 1, 'sourceID': 2, 'abilityGameID': spell,
                   'type': 'applydebuff', 'fight_ms': 0} for spell in (10, 11)]
        hits = [{'target_actor': 1, 'source_actor': 2, 'source_instance': 0,
                 'spell_id': 12, 'time_ms': 10, 'periodic': True}]
        with self.assertRaisesRegex(ValueError, 'ambiguous debuff tick episode'):
            module.convert(events, hits, db, [1, 3, 4, 5, 6], 100)


if __name__ == '__main__':
    unittest.main()
