"""Concrete frozen69933 CASC records, not fabricated DB2 fixtures."""
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import import_toy_cooking_focus as focus


class CookingFocusSourceTests(unittest.TestCase):
    def test_exports_only_authored_cooking_focus_requirements(self):
        payload = Path(os.environ['TOY_FOCUS_DB2']).read_bytes()
        spells = focus.cooking_spell_ids(payload)
        self.assertEqual(len(spells), 651)
        self.assertEqual(spells, sorted(set(spells)))
        self.assertIn(2538, spells)  # Charred Wolf Meat, source row38
        self.assertIn(2540, spells)  # Roasted Boar Meat, source row40
        self.assertNotIn(53, spells)  # Backstab, source row2 has focus0

    def test_rejects_another_layout_without_writing_output(self):
        payload = bytearray(Path(os.environ['TOY_FOCUS_DB2']).read_bytes())
        payload[156:160] = (0).to_bytes(4, 'little')
        with self.assertRaisesRegex(ValueError, 'layout'):
            focus.cooking_spell_ids(payload)


if __name__ == '__main__':
    unittest.main()
