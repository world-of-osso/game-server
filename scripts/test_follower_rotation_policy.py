"""Serialized rotation-policy contract, sourced from SimC76f8118 (not its70391 coefficients)."""
import json
from pathlib import Path
import sqlite3
import unittest
from scripts.follower_spec_kits import combat_policy, kit_rows


class WindwalkerRotationPolicyTests(unittest.TestCase):
    def test_combo_actions_publish_one_shared_no_repeat_group(self):
        spells = {
            'Tiger Palm': 100780, 'Blackout Kick': 100784,
            'Rising Sun Kick': 107428, 'Fists of Fury': 113656,
            'Spinning Crane Kick': 101546, 'Strike of the Windlord': 392983,
            'Touch of Death': 322109,
        }
        for name, spell in spells.items():
            with self.subTest(spell=spell):
                _, _, conditions = combat_policy(269, name, spell, [], [])
                self.assertEqual(set(conditions.get('no_repeat_group', [])), set(spells.values()))

    def test_fists_and_rising_sun_precede_blackout_spending(self):
        priority = lambda name, spell: combat_policy(269, name, spell, [], [])[1]
        self.assertLess(priority('Fists of Fury', 113656), priority('Rising Sun Kick', 107428))
        self.assertLess(priority('Rising Sun Kick', 107428), priority('Blackout Kick', 100784))
        self.assertLess(priority('Fists of Fury', 113656), priority('Spinning Crane Kick', 101546))

    def test_other_specs_do_not_gain_combo_restrictions(self):
        _, _, conditions = combat_policy(66, 'Judgment', 275779, [], [])
        self.assertNotIn('no_repeat_group', conditions)


class DemonologyActionProjectionTests(unittest.TestCase):
    def test_native_talents_enable_cast_actions_only_at_their_actual_levels(self):
        root = Path(__file__).resolve().parents[1]
        export = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-kits-2026-10-18/native-source-v2.json')
        source = json.loads(export.read_text())
        selected = [row for row in source['loadouts'] if row['spec_id'] == 266 and row['level'] in (10, 11, 14, 15)]
        self.assertEqual(len(selected), 4)
        with sqlite3.connect((root / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as db:
            rows, _, _ = kit_rows(db, dict(loadouts=selected))
        available = lambda spell, level: any(row[0] == 266 and row[1] == spell and row[2] <= level <= row[3] and row[4] == 'attack' for row in rows)
        self.assertFalse(available(105174, 10))
        self.assertTrue(available(105174, 11), 'SimC init245-247 maps native1250273 to actual cast105174')
        self.assertFalse(available(264178, 14))
        self.assertTrue(available(264178, 15), 'SimC init249-250 maps native426115 to actual cast264178')


class FeralActionProjectionTests(unittest.TestCase):
    def test_berserk_visible_action_yields_to_native_incarnation_override(self):
        root = Path(__file__).resolve().parents[1]
        export = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-kits-2026-10-18/native-source-v2.json')
        source = json.loads(export.read_text())
        selected = [row for row in source['loadouts'] if row['spec_id'] == 103 and row['level'] in (28, 29, 58, 59)]
        with sqlite3.connect((root / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as db:
            rows, _, _ = kit_rows(db, dict(loadouts=selected))
        available = lambda spell, level: any(row[0] == 103 and row[1] == spell and row[2] <= level <= row[3] and row[4] == 'buff' for row in rows)
        self.assertFalse(available(106951, 28))
        self.assertTrue(available(106951, 29), 'native343223 VisibleSpellID106951, SimC spec.berserk_cat')
        self.assertTrue(available(106951, 58))
        self.assertFalse(available(106951, 59), 'native102543 overrides106951')
        self.assertTrue(available(102543, 59))


if __name__ == '__main__':
    unittest.main()
