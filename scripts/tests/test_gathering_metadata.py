"""Gathering source decoding and transactional import behavior."""
import sqlite3
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2_casc


def lock_fixture():
    # Lock2850 (Silverleaf): alternative Classic Herbalism32 or Herbalism2.
    widths = [32, 256, 128, 64, 64]
    values = [0, (32, 2, 0, 0, 0, 0, 0, 0), (0,) * 8,
              (2, 2, 0, 0, 0, 0, 0, 0), (0,) * 8]
    data = bytearray(204 + 40 + 20 + 120)
    data[:4] = b"WDC5"
    struct.pack_into("<9I", data, 136, 1, 5, 68, 0, 0, 0xDA418152, 2850, 2850, 0)
    struct.pack_into("<HH7I", data, 172, 4, 0, 5, 0, 0, 120, 0, 0, 1)
    struct.pack_into("<Q8I", data, 204, 0, len(data), 1, 0, 0, 4, 0, 0, 0)
    offset = 0
    for index, width in enumerate(widths):
        struct.pack_into("<HHIIIII", data, 264 + 24 * index, offset, width, 0, 0, 0, 0, 0)
        offset += width
    data.extend(struct.pack("<I8I8H8B8BI", *[values[0], *values[1], *values[2], *values[3], *values[4], 2850]))
    return bytes(data)


class GatheringMetadataTests(unittest.TestCase):
    def test_lock_alternatives_decode_as_eight_typed_slots(self):
        _, rows = db2_casc.decode(lock_fixture(), inline_arrays={1: 8, 2: 8, 3: 8, 4: 8})
        self.assertEqual(rows[2850], [2850, 0, 32, 2, 0, 0, 0, 0, 0, 0,
                                     *([0] * 8), 2, 2, 0, 0, 0, 0, 0, 0, *([0] * 8)])


if __name__ == "__main__":
    unittest.main()
