"""Check the class spell coverage derivation against the real imported world database."""

import os
from pathlib import Path
import sys
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import spell_coverage  # noqa: E402

WORLD_DB = Path(os.environ.get("WORLD_DB", spell_coverage.DEFAULT_DB))
REFERENCE_DIR = Path(
    os.environ.get("SPELL_REFERENCE_DIR", spell_coverage.DEFAULT_REFERENCE)
)

PALADIN, ROGUE, MAGE, WARRIOR = 2, 4, 8, 1
INTERRUPT_CAST = 68


@unittest.skipUnless(WORLD_DB.exists(), f"world database not found at {WORLD_DB}")
class SpellCoverageTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.db = spell_coverage.open_db(WORLD_DB)
        cls.class_sets = spell_coverage.derive_class_sets(cls.db)
        cls.tallies, cls.index, cls.direct = spell_coverage.aggregate(
            cls.db, cls.class_sets
        )

    @classmethod
    def tearDownClass(cls):
        cls.db.close()

    def test_crusader_strike_is_paladin_baseline_only(self):
        self.assertIn(35395, self.class_sets[PALADIN]["baseline"])
        self.assertEqual(self.index[35395], {PALADIN})

    def test_kick_is_rogue_spell_counted_under_interrupt(self):
        self.assertIn(1766, self.class_sets[ROGUE]["all"])
        interrupters = self.tallies["effect"][INTERRUPT_CAST]["spells"]
        self.assertIn(1766, interrupters)
        self.assertIn(6552, interrupters)  # Pummel
        self.assertIn(6552, self.class_sets[WARRIOR]["baseline"])

    def test_fireball_is_mage_spec_spell(self):
        self.assertIn(133, self.class_sets[MAGE]["spec"])
        self.assertIn(133, self.tallies["effect"][2]["spells"])  # SCHOOL_DAMAGE

    def test_every_class_has_talents_and_triggered_spells(self):
        for class_id, sets in self.class_sets.items():
            self.assertGreater(len(sets["talent"]), 100, class_id)
            self.assertTrue(sets["triggered"].isdisjoint(sets["direct"]), class_id)

    def test_mana_and_runic_power_attributed_to_their_classes(self):
        runic = self.tallies["power"][6]["spells"]
        self.assertTrue(runic)
        self.assertTrue(all(self.index[s] == {6} for s in runic))


@unittest.skipUnless(
    (REFERENCE_DIR / "SharedDefines.h").exists(),
    f"enum sources not found in {REFERENCE_DIR}",
)
class EnumNamesTest(unittest.TestCase):
    def test_retail_effect_and_aura_names(self):
        names = spell_coverage.load_enum_names(REFERENCE_DIR)
        self.assertEqual(names["effect"][INTERRUPT_CAST], "SPELL_EFFECT_INTERRUPT_CAST")
        self.assertEqual(names["effect"][38], "SPELL_EFFECT_DISPEL")
        self.assertEqual(names["aura"][42], "SPELL_AURA_PROC_TRIGGER_SPELL")
        self.assertEqual(names["power"][6], "POWER_RUNIC_POWER")
        self.assertEqual(names["target"][1], "TARGET_UNIT_CASTER")


if __name__ == "__main__":
    unittest.main()
