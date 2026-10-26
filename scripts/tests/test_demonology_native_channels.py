"""Actual 300s native266 report regressions; opt in with DEMO_NATIVE_REPORT.

Run the ordinary dps_parity_measure.py follower fixture first. These assertions
consume its observed combat events, not a synthetic model or registry census.
"""
import json
import os
from pathlib import Path
import unittest


@unittest.skipUnless(os.environ.get("DEMO_NATIVE_REPORT"), "requires actual private266 measurement")
class DemonologyNativeChannels(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = json.loads(Path(os.environ["DEMO_NATIVE_REPORT"]).read_text())
        if cls.report.get("spec_id") != 266:
            raise AssertionError("requires spec266's actual native measurement")

    def damage(self, spells):
        return sum(row["damage"] for row in self.report["combat"]["actions"]
                   if row["spell_id"] in spells)

    def test_literal_primary_target_keeps_actual_wild_imps_attacking(self):
        self.assertGreater(self.damage({104318}), 0, "native HoG Imps must hit the fixture's actual primary")

    def test_actual_dreadstalkers_execute_their_native_dreadbite(self):
        self.assertGreater(self.damage({271971}), 0, "ordinary Call Dreadstalkers must produce pet Dreadbite damage")

    def test_dreadstalkers_selected_vilefiend_companion_deals_native_damage(self):
        self.assertGreater(self.damage({267997, 267999}), 0,
                           "selected Vilefiend must join the real Dreadstalker cast and attack")

    def test_dominion_cast_progression_summons_real_damaging_guardians(self):
        self.assertGreater(self.damage({1282507, 1282534, 1280460, 1292391}), 0,
                           "selected Dominion must produce actual guardian damage")

    def test_grimoire_imp_lord_uses_its_real_funded_bolt(self):
        self.assertGreater(self.damage({1277116}), 0,
                           "selected Imp Lord must use its actual Energy pool and Greater Felbolt")

    def test_dominion_native_refund_funds_the_next_spender(self):
        gains = [row for row in self.report["combat"]["energize"] if row["spell_id"] == 1276318]
        self.assertGreater(sum(row["effective_gain"] for row in gains), 0,
                           "selected Dominion III must execute its actual native shard Energize")


if __name__ == "__main__":
    unittest.main()
