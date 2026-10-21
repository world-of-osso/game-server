"""Serialized rotation-policy contract, sourced from SimC76f8118 (not its70391 coefficients)."""
import unittest
from scripts.follower_spec_kits import combat_policy


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


if __name__ == '__main__':
    unittest.main()
