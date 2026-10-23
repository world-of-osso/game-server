"""Behavior tests using verbatim lines from the pinned SimC Arms probe."""
import importlib.util
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location("oracle", HERE / "simc_spell_oracle.py")
oracle = importlib.util.module_from_spec(spec)
if spec.loader and Path(spec.origin).exists():
    spec.loader.exec_module(oracle)


class OracleTests(unittest.TestCase):
    def parse(self):
        return oracle.parse_log((HERE / "fixtures/arms-recorded.txt").read_text())

    def test_actor_name_apostrophe_preserves_blood_spell_formula(self):
        data = oracle.parse_log((HERE / "fixtures/blood-recorded.txt").read_text())
        self.assertEqual(len(data["events"]), 1)
        self.assertEqual(data["events"][0]["actor"], "MID2_Death_Knight_Blood_San'layn")
        self.assertEqual(data["events"][0]["spell_id"], 206930)

    def test_actor_wide_modifier_preserves_source_without_fabricating_spell(self):
        data = oracle.parse_log((HERE / "fixtures/actor-modifier-recorded.txt").read_text())
        modifier = data["modifiers"][0]
        self.assertIsNone(modifier["spell_id"])
        self.assertEqual(modifier["actor"], "MID2_Warrior_Arms")
        self.assertEqual(modifier["kind"], "all_crit_damage_multiplier")
        self.assertEqual(modifier["source_spell_id"], 1258209)
        self.assertEqual(modifier["value"], 0.6)

    def test_presence_and_consumed_stack_modifiers_are_not_passives(self):
        data = oracle.parse_log((HERE / "fixtures/conditional-recorded.txt").read_text())
        self.assertEqual(len(data["modifiers"]), 3)
        present, consumed, override = data["modifiers"]
        self.assertEqual(present["application"], "buff_present")
        self.assertEqual(consumed["application"], "per_consumed_buff_stack")
        self.assertEqual(consumed["kind"], "cost")
        self.assertEqual(consumed["value"], -100)
        self.assertTrue(override["value_override"])

    def test_records_direct_and_child_periodic_factors(self):
        data = self.parse()
        direct = next(e for e in data["events"] if e["event"] == "direct_amount")
        self.assertEqual(direct["spell_id"], 12294)
        self.assertEqual(direct["factors"]["a_mod"], 5.20121)
        tick = next(e for e in data["events"] if e["event"] == "periodic_amount")
        self.assertEqual(tick["spell_id"], 262115)
        self.assertEqual(tick["factors"]["a_mod"], 0.242466)

    def test_preserves_both_result_stages_without_counting_them_as_casts(self):
        results = [e for e in self.parse()["events"] if e["event"] == "result"]
        self.assertEqual(len(results), 2)
        self.assertEqual(results[0]["spell_id"], 12294)
        self.assertEqual(results[0]["factors"]["mitigated_amount"], 0)
        self.assertGreater(results[1]["factors"]["mitigated_amount"], 0)

    def test_itemises_passive_and_conditional_modifiers(self):
        mods = self.parse()["modifiers"]
        passive = next(m for m in mods if m["source_spell_id"] == 137049)
        self.assertEqual((passive["effect_index"], passive["kind"], passive["value"]), (1, "direct", 15))
        avatar = next(m for m in mods if m["source_spell_id"] == 107574)
        self.assertEqual(avatar["application"], "per_buff_stack")
        mastery = next(m for m in mods if m["source_spell_id"] == 1258398)
        self.assertEqual(mastery["application"], "mastery_scaled")

    def test_summary_reports_printed_armor_and_missing_sp(self):
        summary = self.parse()["summary"]["12294"]
        self.assertEqual(summary["armor_mitigation_fraction"], [0.3])
        self.assertEqual(summary["ap_coefficient"], [5.20121])
        self.assertEqual(summary["sp_coefficient"], [0])
        self.assertAlmostEqual(summary["passive_multipliers"]["direct"]["product"], 1.15 * 1.06 * 1.10 * 1.10)
        self.assertIsNone(self.parse()["summary"]["262115"]["armor_mitigation_fraction"])

    def test_profile_replaces_all_action_lists_and_preserves_gear(self):
        source = 'mage="Test"\nactions.precombat=flask\nactions=fireball\nactions.foo+=/scorch\nhead=hat,id=1\n'
        actual = oracle.build_profile(source, ["fire_blast", "scorch"])
        self.assertNotIn("fireball", actual)
        self.assertNotIn("precombat", actual)
        self.assertIn("head=hat,id=1", actual)
        self.assertIn("actions=fire_blast\nactions+=/scorch", actual)

    def test_report_parent_uses_explicit_child_alias_without_changing_ids(self):
        import simc_spell_oracle_batch as batch
        channel = {"name": "fists_of_fury", "spell_id": 113656}
        events = [{"event": "direct_amount", "action": "fists_of_fury_damage", "spell_id": 117418}]
        actual = batch.matching_formulas(269, channel, events)
        self.assertEqual(actual, events)
        self.assertEqual(actual[0]["spell_id"], 117418)
        self.assertEqual(batch.matching_formulas(71, channel, events), [])

    def test_child_drivers_include_required_trigger_actions(self):
        import simc_spell_oracle_batch as batch
        self.assertIn("bestial_wrath", batch.action_list(253, {"name": "bloodshed"}))
        self.assertIn("eternity_surge", batch.action_list(1467, {"name": "azure_sweep"}))
        self.assertNotIn("melee", batch.action_list(253, {"name": "melee"}))

    def test_top_channels_combine_equal_spell_pet_contributions(self):
        import simc_spell_oracle_batch as batch
        rows = [{"name": "kill_command", "spell_id": 83381, "actor": "duck", "damage": 10, "share_pct": 10}, {"name": "kill_command", "spell_id": 83381, "actor": "animal_companion", "damage": 20, "share_pct": 20}]
        actual = batch.top_channels({"actions": rows})
        self.assertEqual(len(actual), 1)
        self.assertEqual(actual[0]["share_pct"], 30)
        self.assertEqual(actual[0]["actors"], ["duck", "animal_companion"])

    def test_reference_options_preserve_raid_state_and_nondefault_flags(self):
        import json
        proof = json.loads((HERE / "fixtures/arms-reference.proof.json").read_text())
        options = oracle.reference_options(proof)
        self.assertIn("optimal_raid=0", options)
        self.assertIn("report_details=1", options)
        self.assertIn("target_level=93", options)
        self.assertNotIn("max_time=300", options)
        self.assertFalse(any(option.startswith(("output=", "json=")) for option in options))
        proof["command"].append("override.target_armor=0")
        self.assertIn("override.target_armor=0", oracle.reference_options(proof))

    def test_default_reference_discovery_matches_profile_and_rejects_missing(self):
        import json
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            profile = root / "arms.simc"
            profile.touch()
            reference = root / "71.single.proof.json"
            reference.write_text(json.dumps({"command": ["simc", str(profile), "optimal_raid=0"]}))
            with patch.object(oracle, "REFERENCE_ROOT", root):
                self.assertEqual(oracle.resolve_reference_proof(profile), reference)
                with self.assertRaises(ValueError):
                    oracle.resolve_reference_proof(root / "missing.simc")

    def test_timeout_keeps_command_hashes_and_failure_proof(self):
        import json
        import subprocess
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            profile = root / "test.simc"
            profile.write_text('mage="Test"\n')
            binary = root / "simc"
            binary.write_bytes(b"test binary")
            output = root / "probe.json"
            reference = root / "reference.proof.json"
            reference.write_text(json.dumps({"command": ["simc", str(profile), "optimal_raid=0"]}))
            with patch.object(oracle, "BINARY", binary), patch.object(oracle.subprocess, "run", side_effect=subprocess.TimeoutExpired(["simc"], 180)):
                with self.assertRaises(subprocess.TimeoutExpired):
                    oracle.run_oracle(profile, ["scorch"], output, reference_proof=reference)
            proof = json.loads(output.with_suffix(".proof.json").read_text())
            self.assertEqual(proof["binary_sha256"], oracle.sha256(binary))
            self.assertEqual(proof["profile_sha256"], oracle.sha256(profile))
            self.assertIn("TimeoutExpired", proof["error"])
            self.assertTrue(output.with_suffix(".simc").exists())

    def test_profile_accepts_specialization_name_without_hero_ambiguity(self):
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            profiles = root / "profiles/MID2"
            profiles.mkdir(parents=True)
            expected = profiles / "MID2_Mage_Fire.simc"
            expected.touch()
            (profiles / "MID2_Mage_Fire_Frostfire.simc").touch()
            with patch.object(oracle, "SIMC_ROOT", root):
                self.assertEqual(oracle.resolve_profile("Fire"), expected)
                self.assertEqual(oracle.resolve_profile("Mage_Fire"), expected)
                with self.assertRaises(ValueError):
                    oracle.resolve_profile("missing")

    def test_invalid_action_is_rejected(self):
        with self.assertRaises(ValueError):
            oracle.build_profile('mage="Test"', ["bad\nactions=other"])


if __name__ == "__main__":
    unittest.main()
