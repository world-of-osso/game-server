"""Concrete WDC5 Lock rows and the exported runtime contract."""
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from import_game_object_locks import decode_locks


def lock_fixture():
    # Actual Lock43 kneeling-open and Lock1673 Deactivating Jewel alternatives.
    data = bytearray(384)
    data[:4] = b"WDC5"
    struct.pack_into("<9I", data, 136, 2, 5, 68, 0, 0, 0xDA418152, 43, 1673, 0)
    struct.pack_into("<HH", data, 172, 4, 0)
    struct.pack_into("<7I", data, 176, 5, 0, 0, 120, 0, 0, 1)
    struct.pack_into("<Q8I", data, 204, 0, 384, 2, 0, 520, 8, 0, 0, 0)
    for index, (offset, width) in enumerate([(0, 32), (32, 256), (288, 128), (416, 64), (480, 64)]):
        struct.pack_into("<HHIIIII", data, 264 + index * 24, offset, width, 0, 0, 0, 0, 0)
    for item, kind in [(13, 2), (24337, 1)]:
        indices, types = [0] * 8, [0] * 8
        indices[1], types[1] = item, kind
        data.extend(struct.pack("<I8I8H8B8B", 0, *indices, *([0] * 8), *types, *([0] * 8)))
    data.extend(struct.pack("<2I", 43, 1673))
    return bytes(data)


class LockImportTests(unittest.TestCase):
    def test_authored_array_slots_become_individual_lock_requirements(self):
        self.assertEqual(decode_locks(lock_fixture()), {
            "layout_hash": 0xDA418152,
            "locks": {
                "43": [{"kind": 2, "index": 13, "skill": 0, "action": 0}],
                "1673": [{"kind": 1, "index": 24337, "skill": 0, "action": 0}],
            },
        })

    def test_unknown_layout_fails_instead_of_guessing_array_order(self):
        data = bytearray(lock_fixture())
        struct.pack_into("<I", data, 156, 0)
        with self.assertRaisesRegex(ValueError, "DA418152"):
            decode_locks(data)


if __name__ == "__main__":
    unittest.main()
