import importlib.util
import struct
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("db2_casc", Path(__file__).parents[1] / "db2_casc.py")
db2 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(db2)


def relationship_fixture(second_relation=1):
    data = bytearray(344)
    data[:4] = b"WDC5"
    struct.pack_into("<9I", data, 136, 2, 2, 8, 0, 0, 1, 1, 2, 2)
    struct.pack_into("<HH7I", data, 172, 0, 0, 2, 0, 0, 48, 0, 0, 1)
    struct.pack_into("<Q8I", data, 204, 0, 300, 2, 0, 0, 0, 28, 0, 0)
    struct.pack_into("<HHIIIII", data, 252, 0, 32, 0, 0, 0, 0, 0)
    struct.pack_into("<HHIIIII", data, 276, 32, 32, 0, 0, 0, 0, 0)
    struct.pack_into("<4I", data, 300, 1, 0, 2, 1)
    struct.pack_into("<7I", data, 316, 2, 1, 2, 0, 0, second_relation, 1)
    return bytes(data)


class SectionTests(unittest.TestCase):
    def test_inline_relationship_preserves_schema_width_and_verifies_parent(self):
        _, rows = db2.decode(relationship_fixture(), relationship_column=1)
        self.assertEqual(rows, {1: [1, 0], 2: [2, 1]})
        with self.assertRaisesRegex(ValueError, "relationship"):
            db2.decode(relationship_fixture(second_relation=530), relationship_column=1)

    def test_noninline_relationship_remains_an_additional_column(self):
        _, rows = db2.decode(relationship_fixture())
        self.assertEqual(rows, {1: [1, 0, 0], 2: [2, 1, 1]})

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
