import importlib.util
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "generate_follower_wcl_profiles.py"
spec = importlib.util.spec_from_file_location("follower_profiles", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

SHIRT = {"id": 52019, "itemLevel": "1", "name": "Precious's Ribbon"}
CAPS = {7: (12848, 328), 11: (13848, 344), 12: (13848, 344)}


def character(name, level, track_bonus):
    return {
        "name": name,
        "class": "Paladin",
        "spec": "Retribution",
        "server": {"name": "Stormrage", "region": "US"},
        "hardModeLevel": level,
        "report": {"code": f"report-{name}", "fightID": 3},
        "gear": [
            {
                "id": 268213,
                "itemLevel": "341",
                "bonusIDs": ["6652", track_bonus],
                "permanentEnchant": "8689",
                "gems": [{"id": "240892", "itemLevel": "295"}],
            },
            SHIRT,
            {"id": 0, "itemLevel": "0", "name": "Unknown Item"},
        ],
        "talents": [{"talentID": 102476, "points": 1}],
    }


class GenerateFollowerProfilesTest(unittest.TestCase):
    def setUp(self):
        self.selected = {
            "paladin-retribution/10-14": {
                "race": {},
                "characters": [
                    character("Untrackable", 11, "99999"),
                    character("Overgeared", 12, "12856"),
                ],
            },
            "paladin-retribution/6-9": {
                "race": {},
                "characters": [character("Hero", 7, "12854")],
            },
            "priest-holy/10-14": {"race": {}, "characters": []},
        }
        self.races = {70: {"id": 3, "source": "RACE_FROM_SIMC_PROFILE"}}
        self.decoded, self.validated = [], []

    def decode(self, profiles):
        self.decoded.extend(profiles)
        return [
            {"spec_id": 70, "entries": [[102476, 1]], "purchases": [[1, 102476, 1]], "error": None}
            for _ in profiles
        ]

    def validate(self, candidates):
        """Stands in for follower-profile-validate: bonus 99999 cannot take a track."""
        self.validated.extend(candidates)
        verdicts = []
        for candidate in candidates:
            cap, level = CAPS[candidate["actual_key"]]
            bonus = candidate["gear"][0]["bonusIDs"][1]
            if bonus == "99999":
                verdict = {"items": [], "rejections": [f"item268213: cannot take track cap list{cap}"]}
            else:
                items = [{"id": 268213, "bonus_ids": [6652, cap], "item_level": level,
                          "source_item_level": 341}]
                verdict = {"items": items, "rejections": []}
            verdicts.append({"key": candidate["key"], **verdict})
        return verdicts

    def generate(self):
        return module.generate(
            json.dumps(self.selected).encode(),
            {("paladin", "retribution"): (2, 70)},
            self.races,
            self.decode,
            self.validate,
        )

    def test_keeps_logged_items_on_the_band_track_cap(self):
        output = self.generate()
        profile = output["profiles"]["paladin-retribution/10-14"]
        self.assertEqual(profile["character"], "Overgeared")
        self.assertEqual((profile["class_id"], profile["spec_id"], profile["actual_key"]), (2, 70, 12))
        self.assertEqual(profile["gear_source"], "GEAR_ITEMS_FROM_WCL_ILVL_FROM_BAND_TRACK")
        self.assertEqual(profile["gear"][0], {
            "id": 268213,
            "itemLevel": 344,
            "bonusIDs": ["6652", "13848"],
            "permanentEnchant": "8689",
            "gems": [{"id": "240892", "itemLevel": "295"}],
            "sourceBonusIDs": ["6652", "12856"],
            "sourceItemLevel": "341",
            "sourceNativeItemLevel": 341,
        })
        self.assertEqual(profile["gear"][1:], [SHIRT, {"id": 0, "itemLevel": "0", "name": "Unknown Item"}])
        self.assertEqual((profile["median_item_level"], profile["source_median_item_level"]), (344, 341))
        self.assertEqual(profile["talents_source"], "TALENTS_FROM_WCL")
        self.assertEqual(profile["talent_purchases"], [[1, 102476, 1]])
        self.assertEqual(profile["race"], {"id": 3, "source": "RACE_FROM_SIMC_PROFILE"})
        hero = output["profiles"]["paladin-retribution/6-9"]
        self.assertEqual((hero["gear"][0]["itemLevel"], hero["gear"][0]["bonusIDs"]), (328, ["6652", "12848"]))

    def test_records_characters_whose_items_cannot_take_the_track(self):
        output = self.generate()
        rejected = output["rejected"]["paladin-retribution/10-14"]
        self.assertEqual(
            [(entry["character"], entry["reasons"]) for entry in rejected],
            [("Untrackable", ["item268213: cannot take track cap list13848"])],
        )

    def test_sends_wcl_entry_ranks_to_decoder_and_purchases_to_validator(self):
        self.generate()
        self.assertEqual(
            self.decoded[0], {"spec_id": 70, "class_id": 2, "entries": [[102476, 1]]}
        )
        self.assertEqual(self.validated[0]["purchases"], [[1, 102476, 1]])
        self.assertEqual(self.validated[0]["actual_key"], 11)

    def test_decoder_errors_and_missing_race_reject_the_character(self):
        self.decode = lambda profiles: [
            {"spec_id": 70, "entries": None, "purchases": None, "error": "loadout decode: InvalidNodes"}
            for _ in profiles
        ]
        self.races = {}
        output = self.generate()
        self.assertEqual(output["profiles"], {})
        reasons = output["rejected"]["paladin-retribution/10-14"][1]["reasons"]
        self.assertEqual(reasons, [
            "talents: engine decode: loadout decode: InvalidNodes",
            "race: unavailable in SimC, parity profiles and WCL character data",
        ])

    def test_wcl_character_race_fills_specs_without_a_roster_race(self):
        self.races = {}
        overgeared = self.selected["paladin-retribution/10-14"]["characters"][1]
        overgeared["race"] = {"id": 70, "name": "Dracthyr"}
        output = self.generate()
        profile = output["profiles"]["paladin-retribution/10-14"]
        self.assertEqual(profile["race"], {"id": 70, "source": "RACE_FROM_WCL_CHARACTER"})
        self.assertNotIn("paladin-retribution/6-9", output["profiles"])
        self.assertEqual(
            output["rejected"]["paladin-retribution/6-9"][0]["reasons"],
            ["race: unavailable in SimC, parity profiles and WCL character data"],
        )

    def test_roster_race_wins_over_wcl_character_race(self):
        self.selected["paladin-retribution/10-14"]["characters"][1]["race"] = {"id": 70}
        profile = self.generate()["profiles"]["paladin-retribution/10-14"]
        self.assertEqual(profile["race"], {"id": 3, "source": "RACE_FROM_SIMC_PROFILE"})

    def test_race_comes_from_class_follower_roster(self):
        roster = [{"spec_id": 1473, "race": 52, "race_source": "RACE_FROM_PARITY_PROFILE"}]
        self.assertEqual(
            module.read_races(roster), {1473: {"id": 52, "source": "RACE_FROM_PARITY_PROFILE"}}
        )

    def test_reads_spec_identity_from_native_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.db"
            with sqlite3.connect(path) as database:
                database.executescript("""
                    CREATE TABLE chr_classes (ID INTEGER, Name_lang TEXT);
                    CREATE TABLE chr_specialization (ID INTEGER, ClassID INTEGER, Name_lang TEXT);
                    INSERT INTO chr_classes VALUES (6, 'Death Knight');
                    INSERT INTO chr_specialization VALUES (250, 6, 'Blood');
                """)
            self.assertEqual(
                module.read_spec_catalog(path)[("deathknight", "blood")], (6, 250)
            )

    def test_rejects_ranking_outside_requested_band(self):
        self.selected["paladin-retribution/10-14"]["characters"][0]["hardModeLevel"] = 15
        with self.assertRaisesRegex(ValueError, "outside its band"):
            self.generate()


if __name__ == "__main__":
    unittest.main()
