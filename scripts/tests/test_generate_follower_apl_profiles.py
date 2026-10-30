import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location(
    "follower_apl_profiles", SCRIPTS / "generate_follower_apl_profiles.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

PROFILE = """paladin="MID2_Paladin_Retribution"
spec=retribution
level=90
race=human
role=attack
talents=CYEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
potion=lights_potential_2
actions.precombat=snapshot_stats
actions=auto_attack
actions+=/judgment,if=holy_power<5
main_hand=,id=237728,bonus_id=12350/12229,enchant_id=7981
"""


class GenerateFollowerAplProfilesTest(unittest.TestCase):
    def test_profile_keeps_literal_actions_consumables_and_provenance(self):
        with tempfile.TemporaryDirectory() as root:
            simc = Path(root)
            source = simc / "profiles/MID2/MID2_Paladin_Retribution.simc"
            source.parent.mkdir(parents=True)
            source.write_text(PROFILE)
            profile = module.build(70, "MID2_Paladin_Retribution", simc)
        self.assertEqual(profile["spec_id"], 70)
        self.assertEqual(profile["profile"], "profiles/MID2/MID2_Paladin_Retribution.simc")
        self.assertEqual(profile["actions"], [
            "actions.precombat=snapshot_stats", "actions=auto_attack",
            "actions+=/judgment,if=holy_power<5"])
        self.assertEqual(profile["consumables"], {"potion": "lights_potential_2"})
        self.assertEqual(profile["equipment"][0]["item_id"], 237728)
        self.assertEqual(profile["level"], "90")
        self.assertEqual(profile["simc_revision"], module.REVISION)

    def test_rejects_unpinned_simc_checkout(self):
        with tempfile.TemporaryDirectory() as root:
            simc = Path(root)
            subprocess.run(["git", "init", "-q", str(simc)], check=True)
            subprocess.run(["git", "-C", str(simc), "-c", "user.email=t@t", "-c", "user.name=t",
                            "commit", "-q", "--allow-empty", "-m", "x"], check=True)
            with self.assertRaisesRegex(ValueError, "expected 76f8118"):
                module.write(simc, simc / "out")

    def test_tracked_profiles_cover_every_driven_dps_and_tank_spec(self):
        tracked = {int(path.name.split(".")[0]) for path in module.OUTPUT.glob("*.profile.json")}
        self.assertEqual(tracked, set(module.SPECS))
        for spec_id in module.SPECS:
            profile = json.loads((module.OUTPUT / f"{spec_id}.profile.json").read_text())
            self.assertEqual(profile["spec_id"], spec_id)
            self.assertTrue(profile["actions"], spec_id)


if __name__ == "__main__":
    unittest.main()
