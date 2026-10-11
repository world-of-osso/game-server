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

    def test_import_preserves_authored_skin_loot_and_offsets_resource_nodes(self):
        import gathering_metadata
        conn = sqlite3.connect(":memory:")
        conn.executescript((Path(__file__).parents[1] / "world_schema.sql").read_text())
        conn.executescript((Path(__file__).parents[1] / "tdb_schema.sql").read_text())
        conn.execute("CREATE TABLE content_item (ID INTEGER PRIMARY KEY)")
        conn.executemany("INSERT INTO content_item VALUES (?)", [(2318,), (2934,)])
        conn.execute("CREATE TABLE content_creature_loot_template AS SELECT * FROM creature_loot_template")
        conn.execute("CREATE TABLE content_gameobject_template AS SELECT * FROM gameobject_template")
        conn.execute("INSERT INTO tdb_gameobject_template (entry,type,Data1) VALUES (1617,50,1414)")
        conn.execute("INSERT INTO content_gameobject_template (entry,type,Data1) VALUES (1617,50,1414)")
        conn.commit()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "lock.db2").write_bytes(lock_fixture())
            (root / "tdb.sql").write_text("INSERT INTO `skinning_loot_template` VALUES (113,0,2318,39.2962,0,1,1,1,1,NULL),(113,0,2934,60.7038,0,1,1,1,1,NULL);\n")
            gathering_metadata.import_metadata(conn, root / "lock.db2", root / "tdb.sql")
            self.assertEqual(conn.execute("SELECT Index_0,Index_1,Type_0,Type_1 FROM gathering_lock WHERE ID=2850").fetchone(), (32, 2, 2, 2))
            self.assertEqual(conn.execute("SELECT Entry,Item,Chance FROM content_skinning_loot_template ORDER BY Item").fetchall(), [(20000113, 2318, 39.2962), (20000113, 2934, 60.7038)])
            self.assertEqual(conn.execute("SELECT Data1 FROM content_gameobject_template WHERE entry=1617").fetchone(), (20001414,))
            gathering_metadata.import_metadata(conn, root / "lock.db2", root / "tdb.sql")
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM content_skinning_loot_template").fetchone(), (2,))


if __name__ == "__main__":
    unittest.main()
