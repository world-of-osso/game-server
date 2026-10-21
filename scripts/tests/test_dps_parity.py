"""Profile instance identities are never replaced by base items or fabricated stats."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "dps_parity", Path(__file__).resolve().parents[1] / "dps_parity.py")
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)


class ParityProfileTests(unittest.TestCase):
    def test_extracts_native_item_instance_and_preserves_rotation_text(self):
        profile = PARITY.parse_profile('''paladin="MID1_Paladin_Protection"
spec=protection
level=90
race=draenei
talents=CIEAAAAAA
flask=flask_of_the_shattered_sun_2
head=spellbreakers_cover,id=237832,bonus_id=8960/12384/9627,gem_id=240983,enchant_id=7961,crafted_stats=32/36
main_hand=axe,id=268209,ilevel=344,enchant_id=8039
actions=auto_attack
actions+=/judgment,if=holy_power<5
# gear_strength=1911
''')
        self.assertEqual(profile.get("class"), "paladin")
        self.assertEqual(profile["talents"], "CIEAAAAAA")
        self.assertEqual(profile["consumables"], {"flask": "flask_of_the_shattered_sun_2"})
        self.assertEqual(profile["equipment"][0], {
            "slot": "head", "item_id": 237832, "bonus_list": [8960, 12384, 9627],
            "item_level_override": None, "enchant": 7961, "gems": [240983],
            "crafted_stats": [32, 36], "options": {}})
        self.assertEqual(profile["equipment"][1]["item_level_override"], 344)
        self.assertEqual(profile["actions"], ["actions=auto_attack", "actions+=/judgment,if=holy_power<5"])

    def test_malformed_gear_fails_instead_of_dropping_identity(self):
        with self.assertRaisesRegex(ValueError, "item.*id"):
            PARITY.parse_profile('mage="Fire"\nhead=made_up,enchant_id=7961\n')


if __name__ == "__main__":
    unittest.main()
