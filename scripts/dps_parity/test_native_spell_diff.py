"""Factor diff behavior; counterfactual native inputs derive from recorded SimC."""
from copy import deepcopy
from pathlib import Path
import importlib.util
import sys
import unittest

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
from simc_spell_oracle import parse_log

spec = importlib.util.spec_from_file_location("native_diff", HERE / "native_spell_diff.py")
diff = importlib.util.module_from_spec(spec)
if Path(spec.origin).exists():
    spec.loader.exec_module(diff)


class NativeDiffTests(unittest.TestCase):
    def recorded(self):
        return parse_log((HERE / "fixtures/arms-recorded.txt").read_text())

    def test_identical_printed_factors_match(self):
        result = diff.diff_receipts(self.recorded(), self.recorded(), 12294)
        self.assertEqual(result["status"], "match")
        self.assertTrue(all(row["status"] == "match" for row in result["factors"]))

    def test_ap_difference_reports_actual_relative_delta(self):
        native = deepcopy(self.recorded())
        event = next(e for e in native["events"] if e["event"] == "direct_amount")
        event["factors"]["a_power"] *= 1.1
        result = diff.diff_receipts(native, self.recorded(), 12294)
        row = next(r for r in result["factors"] if r["factor"] == "a_power")
        self.assertEqual(row["status"], "different")
        self.assertAlmostEqual(row["delta_pct"], 10)
        self.assertEqual(result["status"], "different")

    def test_unprinted_native_factor_is_not_a_zero_or_match(self):
        native = deepcopy(self.recorded())
        event = next(e for e in native["events"] if e["event"] == "direct_amount")
        del event["factors"]["a_mod"]
        result = diff.diff_receipts(native, self.recorded(), 12294)
        row = next(r for r in result["factors"] if r["factor"] == "a_mod")
        self.assertEqual(row["status"], "not_printed_native")
        self.assertIsNone(row["native"])
        self.assertEqual(result["status"], "incomplete")

    def test_uses_final_result_stage_without_double_counting(self):
        impact = diff.select_impact(self.recorded(), 12294)
        self.assertEqual(impact["result"]["mitigated_amount"], 78880.41)
        self.assertEqual(impact["result"]["crit_bonus"], 1.373532)
        self.assertEqual(impact["armor"]["armor_fraction"], 0.3)

    def test_unattributed_product_is_labelled_without_inventing_aura(self):
        native = deepcopy(self.recorded())
        native["events"][0]["factor_attribution"] = {"a_mod": "unattributed product"}
        result = diff.diff_receipts(native, self.recorded(), 12294)
        row = next(r for r in result["factors"] if r["factor"] == "a_mod")
        self.assertEqual(row["native_attribution"], "unattributed product")

    def test_observed_native_products_compare_to_printed_simc_components(self):
        native = deepcopy(self.recorded())
        amount = next(e for e in native["events"] if e["event"] == "direct_amount")["factors"]
        result = [e for e in native["events"] if e["event"] == "result"][-1]["factors"]
        amount["ap_term"] = amount["a_power"] * amount["a_mod"]
        amount["sp_term"] = amount["s_power"] * amount["s_mod"]
        amount["native_scaled_amount"] = amount["base"] + amount["ap_term"] + amount["sp_term"]
        result["source_product"] = result["da_mul"] * result["ply_mul"] * result["vs_mul"] * result["per_mul"]
        result["target_product"] = result["tgt_da_mul"] * result["tgt_mitg_da_mul"]
        output = diff.diff_receipts(native, self.recorded(), 12294)
        self.assertEqual(output["status"], "match")
        product = next(row for row in output["factors"] if row["factor"] == "source_product")
        self.assertIn("da_mul", product["simc_derivation"])

    def test_zero_sp_coefficients_do_not_make_unused_power_a_formula_mismatch(self):
        native = deepcopy(self.recorded())
        next(e for e in native["events"] if e["event"] == "direct_amount")["factors"]["s_power"] = 3554
        output = diff.diff_receipts(native, self.recorded(), 12294)
        power = next(row for row in output["factors"] if row["factor"] == "s_power")
        self.assertEqual(power["status"], "different")
        self.assertFalse(power["coefficient_term_active"])
        self.assertEqual(output["status"], "match")

    def test_same_id_real_fof_hits_select_the_requested_action_series(self):
        receipt = parse_log((HERE / "fixtures/fof-same-id-recorded.txt").read_text())
        tier = diff.select_impact(receipt, 117418, action="fists_of_fury_damage_mid2_2pc")
        regular = diff.select_impact(receipt, 117418, action="fists_of_fury_damage")
        self.assertEqual(tier["time"], regular["time"])
        self.assertEqual(tier["result"]["actual_amount"], 17604.07)
        self.assertEqual(regular["result"]["actual_amount"], 35208.14)
        self.assertEqual(regular["action"], "fists_of_fury_damage")

    def test_missing_action_does_not_select_another_same_id_hit(self):
        receipt = parse_log((HERE / "fixtures/fof-same-id-recorded.txt").read_text())
        with self.assertRaises(ValueError):
            diff.select_impact(receipt, 117418, action="missing_fof_action")

    def test_diff_records_independent_native_and_oracle_action_selection(self):
        receipt = parse_log((HERE / "fixtures/fof-same-id-recorded.txt").read_text())
        result = diff.diff_receipts(
            receipt, receipt, 117418,
            native_action="fists_of_fury_damage", oracle_action="fists_of_fury_damage",
        )
        self.assertEqual(result["status"], "match")
        self.assertEqual(result["native_action"], "fists_of_fury_damage")
        self.assertEqual(result["oracle_action"], "fists_of_fury_damage")
        damage = next(row for row in result["factors"] if row["factor"] == "actual_amount")
        self.assertEqual(damage["native"], 35208.14)

    def test_observed_native_emission_identity_is_preserved_without_guessing_oracle_origin(self):
        oracle = parse_log((HERE / "fixtures/fof-same-id-recorded.txt").read_text())
        native = deepcopy(oracle)
        origin = {"kind": "periodic_dummy_trigger", "aura_spell_id": 113656,
                  "effect_index": 2, "triggered_spell_id": 117418}
        formula = next(event for event in native["events"]
                       if event["event"] == "direct_amount" and event["action"] == "fists_of_fury_damage")
        formula["native_emission_origin"] = origin
        result = diff.diff_receipts(native, oracle, 117418,
                                    native_action="fists_of_fury_damage", oracle_action="fists_of_fury_damage")
        self.assertEqual(result.get("native_emission_origin"), origin)
        self.assertIsNone(result.get("oracle_emission_origin"))

    def test_empty_factor_records_are_incomplete_not_matching(self):
        empty = {"events": [{"event": "direct_amount", "spell_id": 12294, "factors": {}}]}
        self.assertEqual(diff.diff_receipts(empty, empty, 12294)["status"], "incomplete")

    def test_nonfinite_native_only_factor_fails_explicitly(self):
        native = deepcopy(self.recorded())
        next(e for e in native["events"] if e["event"] == "direct_amount")["factors"]["native_only"] = float("nan")
        with self.assertRaises(ValueError):
            diff.diff_receipts(native, self.recorded(), 12294)

    def test_missing_spell_and_nonfinite_factor_fail_explicitly(self):
        with self.assertRaises(ValueError):
            diff.select_impact(self.recorded(), 999999)
        native = deepcopy(self.recorded())
        next(e for e in native["events"] if e["event"] == "direct_amount")["factors"]["a_mod"] = float("nan")
        with self.assertRaises(ValueError):
            diff.diff_receipts(native, self.recorded(), 12294)


if __name__ == "__main__":
    unittest.main()
