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

    def test_simc_scenarios_are_fixed_300_seconds_and_capped_at_three_targets(self):
        profile = {"profile": "/profiles/MID2/MID2_Paladin_Protection.simc", "level": "90"}
        single = PARITY.build_simc_arguments(profile, "single", Path("/evidence/66"))
        packs = PARITY.build_simc_arguments(profile, "three", Path("/evidence/66"))
        for arguments in [single, packs]:
            self.assertIn(profile["profile"], arguments)
            self.assertIn("max_time=300", arguments)
            self.assertIn("fixed_time=1", arguments)
            self.assertIn("vary_combat_length=0", arguments)
            self.assertIn("threads=1", arguments)
            self.assertIn("target_level=93", arguments)
            self.assertNotIn("fight_style=DungeonSlice", arguments)
        self.assertIn("desired_targets=1", single)
        self.assertFalse(any(arg.startswith("raid_events=") for arg in single))
        self.assertIn("desired_targets=1", packs)
        self.assertIn("raid_events=adds,count=2,count_range=0,first=0,cooldown=30,duration=30,cooldown_stddev=0,duration_stddev=0", packs)
        self.assertIn("json=/evidence/66.single.json", single)
        self.assertIn("json=/evidence/66.three.json", packs)

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

    def test_selected_tiered_talents_use_allocated_entries_not_consumed_debug_rank(self):
        rows = [
            {"tree": "spec", "node_id": 110418, "entry_id": 137021, "ranks": 2, "max_ranks": 2},
            {"tree": "spec", "node_id": 110418, "entry_id": 137022, "ranks": 1, "max_ranks": 1},
        ]
        selected = PARITY.parse_selected_talents(json.dumps(rows))
        self.assertEqual(selected, [
            {"tree": "spec", "node_id": 110418, "entry_id": 137021, "ranks": 2},
            {"tree": "spec", "node_id": 110418, "entry_id": 137022, "ranks": 1},
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

    def test_stats_first_comparison_flags_strictly_over_five_percent(self):
        rows = PARITY.compare_stat_sheets(
            {"attribute": {"strength": 100}, "stats": {"attack_power": 200, "crit_rating": 40}},
            {"attribute": {"strength": 110}, "stats": {"attack_power": 190, "crit_rating": 40}})
        self.assertEqual(rows, [
            {"stat": "attribute.strength", "reference": 100, "server": 110,
             "delta_pct": 10.0, "flag": True, "status": "measured"},
            {"stat": "stats.attack_power", "reference": 200, "server": 190,
             "delta_pct": -5.0, "flag": False, "status": "measured"},
            {"stat": "stats.crit_rating", "reference": 40, "server": 40,
             "delta_pct": 0.0, "flag": False, "status": "measured"},
        ])

    def test_missing_and_zero_reference_stats_never_become_fake_percentages(self):
        rows = PARITY.compare_stat_sheets(
            {"attribute": {"intellect": 100}, "stats": {"attack_power": 0, "spell_power": 0}},
            {"attribute": {"agility": 50}, "stats": {"attack_power": 25, "spell_power": 0}})
        self.assertEqual([row["status"] for row in rows],
                         ["missing_reference", "missing_server", "zero_reference", "measured"])
        self.assertEqual([row["delta_pct"] for row in rows], [None, None, None, 0.0])
        self.assertEqual([row["flag"] for row in rows], [True, True, True, False])

    def test_nonfinite_stats_fail_explicitly_before_delta_classification(self):
        with self.assertRaisesRegex(ValueError, "finite"):
            PARITY.compare_stat_sheets({"stats": {"attack_power": 10}},
                                      {"stats": {"attack_power": float("nan")}})

    def test_malformed_gear_fails_instead_of_dropping_identity(self):
        with self.assertRaisesRegex(ValueError, "item.*id"):
            PARITY.parse_profile('mage="Fire"\nhead=made_up,enchant_id=7961\n')


if __name__ == "__main__":
    unittest.main()
