import importlib.util
import struct
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("db2_casc", Path(__file__).parents[1] / "db2_casc.py")
db2 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(db2)


class SectionTests(unittest.TestCase):
    def test_decrypted_sections_and_virtual_string_offsets(self):
        data = bytearray(400)
        data[:4] = b"WDC5"
        struct.pack_into("<9I", data, 136, 2, 2, 8, 4, 0, 1, 1, 2, 2)
        struct.pack_into("<HH7I", data, 172, 0, 0, 2, 0, 0, 48, 0, 0, 2)
        struct.pack_into("<Q8I", data, 204, 0, 340, 1, 2, 0, 0, 0, 0, 0)
        struct.pack_into("<Q8I", data, 244, 123, 360, 1, 2, 0, 0, 0, 0, 0)
        struct.pack_into("<HHIIIII", data, 292, 0, 32, 0, 0, 0, 0, 0)
        struct.pack_into("<HHIIIII", data, 316, 32, 32, 0, 0, 0, 0, 0)
        struct.pack_into("<II", data, 340, 1, 12)
        data[348:350] = b"a\0"
        struct.pack_into("<II", data, 360, 2, 6)
        data[368:370] = b"b\0"
        _, rows = db2.decode(bytes(data), {1})
        self.assertEqual(rows, {1: [1, "a"], 2: [2, "b"]})


if __name__ == "__main__":
    unittest.main()
