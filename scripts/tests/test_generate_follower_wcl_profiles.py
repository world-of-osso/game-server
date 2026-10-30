import importlib.util
import json
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "generate_follower_wcl_profiles.py"
spec = importlib.util.spec_from_file_location("follower_profiles", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class GenerateFollowerProfilesTest(unittest.TestCase):
    def setUp(self):
        self.selected = {
            "paladin-retribution/10-14": {
                "race": {"value": "human", "source": "simc", "path": "pinned.simc"},
                "characters": [{
                    "name": "Example", "server": {"name": "Stormrage", "region": "US"},
                    "hardModeLevel": 11, "report": {"code": "abc", "fightID": 3},
                    "gear": [{"id": 268213, "bonusIDs": ["6652"], "permanentEnchant": "8689"}],
                    "talents": [{"talentID": 102476, "points": 1}],
                }],
            },
            "priest-holy/10-14": {"race": {}, "characters": []},
        }

    def generate(self):
        return module.generate(json.dumps(self.selected).encode())

    def test_preserves_selected_source_gear_talents_and_provenance(self):
        output = self.generate()
        self.assertEqual(len(output["profiles"]), 1)
        profile = output["profiles"]["paladin-retribution/10-14"]
        self.assertEqual(profile["actual_key"], 11)
        self.assertEqual(profile["gear"][0]["id"], 268213)
        self.assertEqual(profile["gear"][0]["permanentEnchant"], "8689")
        self.assertEqual(profile["talents"], [{"talentID": 102476, "points": 1}])
        self.assertEqual(profile["report"], {"code": "abc", "fightID": 3})
        self.assertEqual(profile["race"]["source"], "simc")
        self.assertIn("not validated", output["qualification"])

    def test_rejects_ranking_outside_requested_band(self):
        self.selected["paladin-retribution/10-14"]["characters"][0]["hardModeLevel"] = 15
        with self.assertRaisesRegex(ValueError, "outside its band"):
            self.generate()


if __name__ == "__main__":
    unittest.main()
