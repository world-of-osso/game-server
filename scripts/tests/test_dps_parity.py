"""Profile instance identities are never replaced by base items or fabricated stats."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "dps_parity", Path(__file__).resolve().parents[1] / "dps_parity.py")
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)


class ParityProfileTests(unittest.TestCase):
    def test_current_tier_manifests_use_mid2_and_label_prior_tier_evoker_gap(self):
        suffixes = ["Paladin_Protection", "Warrior_Protection", "Shaman_Elemental", "Mage_Fire",
                    "Hunter_Beast_Mastery", "Monk_Windwalker", "Warlock_Demonology", "Druid_Feral"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for tier in ["MID1", "MID2"]:
                folder = root / "profiles" / tier
                folder.mkdir(parents=True)
                names = suffixes + (["Evoker_Devastation"] if tier == "MID1" else [])
                for suffix in names:
                    actor = suffix.split("_")[0].lower()
                    (folder / f"{tier}_{suffix}.simc").write_text(f'{actor}="{tier}_{suffix}"\nlevel=90\n')
            output = root / "output"
            PARITY.write_manifests(root, output)
            paladin = json.loads((output / "66.profile.json").read_text())
            evoker = json.loads((output / "1467.profile.json").read_text())
            self.assertEqual(Path(paladin["profile"]).parent.name, "MID2")
            self.assertEqual(evoker["reference_tier"], "MID1")
            self.assertEqual(evoker["reference_gap"], "SIMC_DEVASTATION_MID2_PROFILE_ABSENT")
            healer = json.loads((output / "105.profile.json").read_text())
            self.assertIn("No shipped", healer["gap"])

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

    def test_selected_talents_keep_source_node_entry_and_rank_not_name_guesses(self):
        selected = PARITY.parse_selected_talents('''0.000 Parity adding class talent Hammer (node=103001 entry=128001 rank=1/1)
0.000 Parity adding specialization talent Shield (node=103002 entry=128002 rank=2/2)
0.000 Parity adding hero talent Light (node=103003 entry=128003 rank=1/1)
0.000 Parity adding class talent Hammer (node=103001 entry=128001 rank=1/1)
0.000 Parity ordinary spell damage 200
''')
        self.assertEqual(selected, [
            {"tree": "class", "node_id": 103001, "entry_id": 128001, "ranks": 1},
            {"tree": "specialization", "node_id": 103002, "entry_id": 128002, "ranks": 2},
            {"tree": "hero", "node_id": 103003, "entry_id": 128003, "ranks": 1},
        ])

    def test_missing_selected_talent_metadata_does_not_become_default_talents(self):
        with self.assertRaisesRegex(ValueError, "selected talent"):
            PARITY.parse_selected_talents("SimulationCraft initialized without debug metadata")

    def test_action_damage_does_not_double_count_compound_children(self):
        report = {"sim": {"players": [{"collected_data": {
            "dps": {"mean": 10}, "hps": {"mean": 0},
            "buffed_stats": {"attribute": {"strength": 100}, "stats": {"crit_pct": 0.25}}},
            "stats": [{"id": 1, "name": "parent", "type": "damage",
                       "num_executes": {"mean": 2}, "compound_amount": 300,
                       "actual_amount": {"mean": 100}, "num_direct_results": {"mean": 2},
                       "children": [{"id": 2, "name": "periodic", "type": "damage",
                                     "num_executes": {"mean": 0}, "compound_amount": 200,
                                     "actual_amount": {"mean": 200}, "num_tick_results": {"mean": 4}}]}],
            "buffs": [{"name": "test_buff", "uptime": 25.0}]}]}}
        result = PARITY.normalize_simc(report)
        self.assertEqual(result.get("dps"), 10)
        rows = result["actions"]
        self.assertEqual(sum(row["damage"] for row in rows), 300)
        self.assertEqual(rows[0]["average_hit"], 50)
        self.assertEqual(rows[1]["average_hit"], 50)
        self.assertAlmostEqual(rows[0]["share_pct"], 100 / 3)
        self.assertEqual(result["buffs"], {"test_buff": 25.0})

    def test_malformed_gear_fails_instead_of_dropping_identity(self):
        with self.assertRaisesRegex(ValueError, "item.*id"):
            PARITY.parse_profile('mage="Fire"\nhead=made_up,enchant_id=7961\n')


if __name__ == "__main__":
    unittest.main()
