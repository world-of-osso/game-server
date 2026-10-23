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

    def test_missing_spell_and_nonfinite_factor_fail_explicitly(self):
        with self.assertRaises(ValueError):
            diff.select_impact(self.recorded(), 999999)
        native = deepcopy(self.recorded())
        next(e for e in native["events"] if e["event"] == "direct_amount")["factors"]["a_mod"] = float("nan")
        with self.assertRaises(ValueError):
            diff.diff_receipts(native, self.recorded(), 12294)


if __name__ == "__main__":
    unittest.main()
