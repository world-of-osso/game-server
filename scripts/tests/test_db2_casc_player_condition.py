"""Real PlayerCondition 1D743D99 declares an array pallet before scalar pallets."""

import hashlib
import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2_casc

SOURCE = Path(os.environ["PLAYER_CONDITION_DB2"])
SHA256 = "b7ba11a5d3f6bf1770ea5be475b460e509f9c0275df06ae7e37aaf8849bb2522"
# Decoded columns: ID list first, then stored fields 48 MovementFlags (array pallet),
# 49 WeaponSubclassMask and 56 ContentTuningID (scalar pallets).
COLUMNS = (49, 50, 57)


class PlayerConditionPalletOrderTest(unittest.TestCase):
    def test_scalar_pallets_are_read_before_the_earlier_declared_array_pallet(self):
        data = SOURCE.read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), SHA256)
        with self.assertWarnsRegex(UserWarning, "zero-filled DB2 section"):
            header, rows = db2_casc.decode(data, record_ids=[62, 36577, 36583, 36584])
        self.assertEqual(header["layout_hash"], 0x1D743D99)
        # Field order instead reads 1048866/958 into every row and shifts MovementFlags.
        expected = {62: (0, 0, 0), 36577: (1, 0, 0), 36583: (8, 0, 0), 36584: (4, 0, 0)}
        for record_id, values in expected.items():
            with self.subTest(record_id=record_id):
                self.assertEqual(tuple(rows[record_id][c] for c in COLUMNS), values)


if __name__ == "__main__":
    unittest.main()
