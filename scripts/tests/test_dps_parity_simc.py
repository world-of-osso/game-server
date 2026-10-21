"""Opt-in pinned CLI integration: assert actual targets, not argument construction."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import unittest

SPEC = importlib.util.spec_from_file_location(
    "dps_parity", Path(__file__).resolve().parents[1] / "dps_parity.py")
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)


@unittest.skipUnless(os.environ.get("DPS_PARITY_SIMC"), "requires opt-in pinned SimC CLI and evidence directory")
class SimcScenarioTests(unittest.TestCase):
    def test_three_target_reference_really_spawns_two_fixed_adds(self):
        directory = Path(os.environ["DPS_PARITY_OUTPUT"])
        profile = json.loads((directory / "66.profile.json").read_text())
        prefix = directory / os.environ["DPS_PARITY_TEST_PREFIX"]
        arguments = PARITY.build_simc_arguments(profile, "three", prefix)
        subprocess.run([os.environ["DPS_PARITY_SIMC"], *arguments,
                        "iterations=1", "item_db_source=local"],
                       cwd=directory, check=True, timeout=120)
        report = json.loads(Path(f"{prefix}.three.json").read_text())["sim"]
        # SimC serializes enemy pets' debuffs under their main target, not as
        # separate top-level targets. Positive uptime proves actual recipients.
        recipients = {buff["target"] for target in report["targets"]
                      for buff in target["buffs"] if buff.get("uptime", 0) > 0}
        self.assertEqual(recipients, {"Fluffy_Pillow", "Fluffy_Pillow_1", "Fluffy_Pillow_2"},
                         "actual main target and both adds must receive effects")
        self.assertTrue(any(event["type"] == "adds" for event in report.get("raid_events", [])))
        self.assertEqual(report["players"][0]["collected_data"]["fight_length"]["mean"], 300.0)
