"""Serialized rotation-policy contract, sourced from SimC76f8118 (not its70391 coefficients)."""
from contextlib import closing
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
        with closing(sqlite3.connect((root / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as db:
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
        with closing(sqlite3.connect((root / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as db:
            rows, _, _ = kit_rows(db, dict(loadouts=selected))
        available = lambda spell, level: any(row[0] == 103 and row[1] == spell and row[2] <= level <= row[3] and row[4] == 'buff' for row in rows)
        self.assertFalse(available(106951, 28))
        self.assertTrue(available(106951, 29), 'native343223 VisibleSpellID106951, SimC spec.berserk_cat')
        self.assertTrue(available(106951, 58))
        self.assertFalse(available(106951, 59), 'native102543 overrides106951')
        self.assertTrue(available(102543, 59))


class RotationResourcePolicyTests(unittest.TestCase):
    def test_windwalker_tiger_palm_reserves_room_for_native_two_chi(self):
        conditions = combat_policy(269, 'Tiger Palm', 100780, [], [])[2]
        self.assertEqual(conditions.get('resources'), [dict(power='Chi', min_deficit=2)])

    def test_feral_finishers_require_five_points(self):
        for name, spell in [('Rip', 1079), ('Ferocious Bite', 22568), ('Primal Wrath', 285381)]:
            with self.subTest(spell=spell):
                conditions = combat_policy(103, name, spell, [], [])[2]
                self.assertIn(dict(power='ComboPoints', min=5), conditions.get('resources', []))

    def test_demonbolt_requires_real_core_and_shard_room(self):
        conditions = combat_policy(266, 'Demonbolt', 264178, [], [])[2]
        self.assertEqual(conditions.get('required_aura'), 264173)
        self.assertEqual(conditions.get('resources'), [dict(power='SoulShards', below=4)])

    def test_hand_of_guldan_banks_three_real_shards(self):
        conditions = combat_policy(266, "Hand of Gul'dan", 105174, [], [])[2]
        self.assertEqual(conditions.get('resources'), [dict(power='SoulShards', min=3)])


class RotationRefreshAndDefensePolicyTests(unittest.TestCase):
    def test_native_rows_preserve_child_and_retire_sotr_missing_only_gate(self):
        root = Path(__file__).resolve().parents[1]
        export = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-kits-2026-10-18/native-source-v2.json')
        source = json.loads(export.read_text())
        selected = [row for row in source['loadouts'] if row['spec_id'] in (66, 73, 103) and row['level'] == 32]
        with closing(sqlite3.connect((root / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as db:
            rows, _, _ = kit_rows(db, dict(loadouts=selected))
        conditions = {(row[0], row[1]): json.loads(row[6]) for row in rows}
        self.assertNotIn('missing_aura', conditions[66, 53600])
        self.assertFalse(conditions[66, 53600].get('attacked', False))
        self.assertEqual(conditions[73, 2565]['missing_aura'], 132404)
        self.assertEqual(conditions[73, 2565]['refresh_seconds'], 10)
        self.assertEqual(conditions[103, 1822]['missing_aura'], 155722)
        self.assertEqual(conditions[103, 1822]['refresh_fraction'], 0.30)

    def test_feral_uses_fresh_pandemic_window(self):
        for name, spell in [('Rip', 1079), ('Rake', 1822)]:
            with self.subTest(spell=spell):
                self.assertEqual(combat_policy(103, name, spell, [], [])[2].get('refresh_fraction'), 0.30)

    def test_shield_block_tracks_native_aura_and_ten_second_window(self):
        conditions = combat_policy(73, 'Shield Block', 2565, [], [])[2]
        self.assertEqual((conditions.get('missing_aura'), conditions.get('refresh_seconds')), (132404, 10))

    def test_shield_of_righteous_is_a_damage_spender_even_with_armor_up(self):
        conditions = combat_policy(66, 'Shield of the Righteous', 53600, [], [])[2]
        self.assertEqual((conditions.get('attacked', False), conditions.get('missing_aura')), (False, None))

    def test_tank_majors_share_named_sequencing_without_duplicate_aura_gate(self):
        for spec, name, spell in [(66, 'Ardent Defender', 31850), (66, 'Guardian of Ancient Kings', 86659), (73, 'Shield Wall', 871)]:
            with self.subTest(spell=spell):
                _, priority, conditions = combat_policy(spec, name, spell, [], [])
                actual = (priority, conditions.get('health_below'), conditions.get('attacked'), conditions.get('tank_defensive_sequence'), conditions.get('missing_aura'))
                self.assertEqual(actual, (30, 0.5, True, True, None))


if __name__ == '__main__':
    unittest.main()
