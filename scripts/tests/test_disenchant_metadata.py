"""Real local Retail metadata through the server's full-import decoder path."""
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2
import db2_casc


class DisenchantMetadataTests(unittest.TestCase):
    def test_local_retail_rows_preserve_signed_fields_and_zero_skill(self):
        data = (SCRIPTS.parent / "data/reference/tdb1210/1134087.db2").read_bytes()
        header, raw = db2_casc.decode(data)
        self.assertEqual(header["layout_hash"], 0x5845A4AA)
        rows = db2.signed_disenchant_rows([raw[key] for key in sorted(raw)])
        self.assertEqual(len(rows), 166)
        self.assertEqual(next(row for row in rows if row[0] == 3), [3, -1, 2, 1, 8, 0, 0, 2])
        self.assertEqual({row[5] for row in rows}, {0})
        self.assertTrue(all(-128 <= row[1] <= 127 and -128 <= row[6] <= 127 for row in rows))


if __name__ == "__main__":
    unittest.main()
