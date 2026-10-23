"""Behavior tests using verbatim lines from the pinned SimC Arms probe."""
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).parent
spec = importlib.util.spec_from_file_location("oracle", HERE / "simc_spell_oracle.py")
oracle = importlib.util.module_from_spec(spec)
if spec.loader and Path(spec.origin).exists():
    spec.loader.exec_module(oracle)


class OracleTests(unittest.TestCase):
    def parse(self):
        return oracle.parse_log((HERE / "fixtures/arms-recorded.txt").read_text())

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

    def test_invalid_action_is_rejected(self):
        with self.assertRaises(ValueError):
            oracle.build_profile('mage="Test"', ["bad\nactions=other"])


if __name__ == "__main__":
    unittest.main()
