"""Behavioral fixtures for the observed WDC5 Sparse + Index variant."""

import os
import struct
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import db2_casc


def sparse_fixture(records=None):
    records = records or [(101, b"A\0"), (102, b"Longer name\0")]
    metadata = [(0, 32), (16, 16), (24, 24), (0, 32), (0, 32)]
    data = bytearray(204 + 80)
    data[:4] = b"WDC5"
    struct.pack_into(
        "<9I", data, 136, len(records) + 1, 5, 17, 0, 0, 0x1234, 101, 900, 0
    )
    struct.pack_into("<HH7I", data, 172, 5, 0xFFFF, 5, 0, 0, 120, 0, 0, 2)
    for bits, _ in metadata:
        data.extend(struct.pack("<hH", bits, 0))
    for _, width in metadata:
        data.extend(struct.pack("<HHIIIII", 0, width, 0, 0, 0, 0, 0))
    start = len(data)
    entries = []
    for record_id, text in records:
        row = text + struct.pack("<h3BfI", -7, 2, 5, 255, 1.5, 123456)
        entries.append((len(data), len(row)))
        data.extend(row)
    end = len(data)
    data.extend(struct.pack(f"<{len(records)}I", *(r[0] for r in records)))
    data.extend(struct.pack("<II", 103, 101))
    for entry in entries:
        data.extend(struct.pack("<IH", *entry))
    data.extend(struct.pack("<5I", 1, 77, 77, 77, 0))
    data.extend(struct.pack(f"<{len(records)}I", *(r[0] for r in records)))
    struct.pack_into(
        "<Q8I",
        data,
        204,
        0,
        start,
        len(records),
        0,
        end,
        4 * len(records),
        20,
        len(records),
        1,
    )
    encrypted_start = len(data)
    data.extend(bytes(40))
    # Encrypted record bytes stay zero; sparse ID metadata identifies row 900.
    struct.pack_into("<I", data, encrypted_start + 17 + 4 + 6, 900)
    struct.pack_into(
        "<Q8I", data, 244, 123, encrypted_start, 1, 0, encrypted_start + 17, 4, 0, 1, 0
    )
    return bytes(data)


def decode_fixture(data, ids):
    return db2_casc.decode(
        data,
        text_columns={1},
        inline_arrays={2: 3},
        field_types={1: "INTEGER", 3: "REAL"},
        record_ids=ids,
    )


class SparseDecodeTest(unittest.TestCase):
    def test_variable_strings_preserve_following_typed_arrays_and_copy_relationship(
        self,
    ):
        _, rows = decode_fixture(sparse_fixture(), {101, 102, 103})
        self.assertEqual(
            rows,
            {
                101: [101, "A", -7, 2, 5, 255, 1.5, 123456, 77],
                102: [102, "Longer name", -7, 2, 5, 255, 1.5, 123456, 0],
                103: [103, "A", -7, 2, 5, 255, 1.5, 123456, 77],
            },
        )

    def test_requested_encrypted_or_missing_row_fails(self):
        for record_id in (900, 999):
            with (
                self.subTest(record_id=record_id),
                self.assertRaisesRegex(
                    ValueError, f"missing readable DB2 rows:.*{record_id}"
                ),
            ):
                decode_fixture(sparse_fixture(), {101, record_id})

    def test_full_table_reports_unreadable_section_and_exports_readable_records(self):
        with self.assertWarnsRegex(UserWarning, 'Skipping zero-filled DB2 section'):
            _, rows = decode_fixture(sparse_fixture(), None)
        self.assertEqual(set(rows), {101, 102, 103})
        self.assertEqual(rows[102][1], 'Longer name')

    def test_sparse_secondary_key_flag_preserves_foreign_relationship(self):
        data = bytearray(sparse_fixture())
        struct.pack_into('<H', data, 172, 7)
        end = struct.unpack_from('<I', data, 224)[0]
        relation_at = end + 8 + 8 + 12
        relation = bytearray(data[relation_at:relation_at + 20])
        # Flag 0x2 relationships use record IDs, not ordinal record indexes.
        struct.pack_into('<I', relation, 16, 101)
        ids = data[relation_at + 20:relation_at + 28]
        # Active 70340 secondary-key sparse tables put offset-map IDs before relations.
        data[relation_at:relation_at + 28] = ids + relation
        _, rows = decode_fixture(bytes(data), {101, 103})
        self.assertEqual(rows[101][-1], 77)
        self.assertEqual(rows[103][-1], 77)

    def test_decrypted_keyed_section_is_readable(self):
        data = bytearray(sparse_fixture())
        struct.pack_into('<Q', data, 204, 456)
        _, rows = decode_fixture(bytes(data), {101, 102})
        self.assertEqual(rows[101][1], 'A')
        self.assertEqual(rows[102][1], 'Longer name')

    def test_malformed_source_string_reports_exact_row_without_synthesizing_text(self):
        data = sparse_fixture([(101, b'Foxton\xa0text\0'), (102, b'Valid text\0')])
        # Remove the copy referring to the unreadable source from this fixture.
        end = struct.unpack_from('<I', data, 224)[0]
        data = bytearray(data)
        struct.pack_into('<II', data, end + 8, 103, 102)
        with self.assertWarnsRegex(UserWarning, 'unreadable UTF-8.*101'):
            _, rows = decode_fixture(bytes(data), None)
        self.assertEqual(set(rows), {102, 103})
        self.assertEqual(rows[102][1], 'Valid text')
        with self.assertRaisesRegex(ValueError, 'missing readable DB2 rows:.*101'):
            decode_fixture(bytes(data), {101})

    def test_unterminated_string_cannot_read_into_next_record(self):
        data = bytearray(sparse_fixture())
        start = struct.unpack_from("<I", data, 212)[0]
        end = struct.unpack_from("<I", data, 224)[0]
        # First offset-map entry follows the ID list and one copy.
        struct.pack_into("<H", data, end + 8 + 8 + 4, 1)
        data[start] = ord("X")
        with self.assertRaisesRegex(ValueError, "unterminated.*string"):
            decode_fixture(bytes(data), {101})

    def test_truncated_numeric_array_fails(self):
        data = bytearray(sparse_fixture())
        end = struct.unpack_from("<I", data, 224)[0]
        struct.pack_into("<H", data, end + 8 + 8 + 4, 6)
        with self.assertRaisesRegex(ValueError, "truncated sparse row"):
            decode_fixture(bytes(data), {101})


# ItemSparse.dbd 6FCC3191: stored-field indexes, not flattened output indexes.
ITEM_ARRAYS = {14: 10, 15: 10, 16: 10, 21: 2, 27: 5, 42: 2, 55: 3}
ITEM_TYPES = dict.fromkeys(
    (
        5,
        7,
        11,
        12,
        15,
        16,
        17,
        18,
        19,
        21,
        27,
        28,
        29,
        30,
        31,
        32,
        33,
        34,
        51,
        63,
        64,
        65,
        66,
    ),
    "INTEGER",
)
ITEM_TYPES.update(dict.fromkeys((6, 9, 13, 14, 25, 26), "REAL"))
ITEM_EXPECTED = {
    271655: ("Thendal Apprentice's Shirt", 4),
    271658: ("Thendal Apprentice's Pants", 7),
    271659: ("Thendal Apprentice's Boots", 8),
    271661: ("Thendal Novice's Boots", 8),
    271662: ("Thendal Novice's Pants", 7),
    271663: ("Thendal Novice's Shirt", 4),
    271665: ("Thendal Survivalist's Boots", 8),
    271666: ("Thendal Survivalist's Pants", 7),
    271668: ("Thendal Survivalist's Shirt", 4),
    271669: ("Thendal Warrior's Shirt", 4),
    271671: ("Thendal Warrior's Boots", 8),
    271672: ("Thendal Warrior's Pants", 7),
    271673: ("Grove Scoundrel's Boots", 8),
    271674: ("Grove Scoundrel's Pants", 7),
    271675: ("Grove Scoundrel's Shirt", 4),
    280399: ("Damaged Scimitar", 21),
    280400: ("Worn Bludgeon", 21),
}


class ItemSparseSourceTest(unittest.TestCase):
    def assert_selected_items(self, data):
        header, rows = db2_casc.decode(
            data,
            text_columns=set(range(1, 6)),
            inline_arrays=ITEM_ARRAYS,
            field_types=ITEM_TYPES,
            record_ids=set(ITEM_EXPECTED),
        )
        self.assertEqual(header["layout_hash"], 0x6FCC3191)
        self.assertEqual(set(rows), set(ITEM_EXPECTED))
        for record_id, (name, inventory_type) in ITEM_EXPECTED.items():
            with self.subTest(record_id=record_id):
                row = rows[record_id]
                self.assertEqual(len(row), 104)
                self.assertEqual(row[:6], [record_id, "", "", "", "", name])
                self.assertEqual(row[35:45], [-1] * 10)  # StatModifier_bonusStat
                self.assertEqual(row[49:51], [-1, -1])  # AllowableRace
                self.assertEqual(
                    row[56:61], [0, 24576 if inventory_type == 21 else 24580, 0, 0, 0]
                )
                self.assertEqual(row[84:86], [2 if inventory_type == 21 else 1, -1])
                self.assertEqual(row[89:92], [0, 0, 0])  # SocketType
                self.assertEqual(row[101:104], [inventory_type, 1, 0])
        self.assertEqual(rows[280400][7], 0.6000000238418579)

    def test_exact_real_selected_record_bytes(self):
        self.assert_selected_items(
            (
                Path(os.environ["ITEM_SPARSE_SELECTED_FIXTURE"])
            ).read_bytes()
        )

    @unittest.skipUnless(
        os.environ.get("ITEM_SPARSE_SOURCE"),
        "set ITEM_SPARSE_SOURCE for original full file",
    )
    def test_original_selected_rows_despite_unrelated_encrypted_sections(self):
        data = Path(os.environ["ITEM_SPARSE_SOURCE"]).read_bytes()
        self.assert_selected_items(data)
        with self.assertWarnsRegex(UserWarning, 'Skipping zero-filled DB2 section'):
            db2_casc.decode(
                data,
                text_columns=set(range(1, 6)),
                inline_arrays=ITEM_ARRAYS,
                field_types=ITEM_TYPES,
            )


if __name__ == "__main__":
    unittest.main()
