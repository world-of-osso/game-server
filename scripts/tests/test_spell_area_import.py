import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from mysql_dump import load_table


class SpellAreaImportTests(unittest.TestCase):
    def test_authored_quest_masks_and_unsigned_race_mask_survive_import(self):
        conn = sqlite3.connect(':memory:')
        conn.executescript((Path(__file__).resolve().parents[1] / 'tdb_schema.sql').read_text())
        with tempfile.TemporaryDirectory() as directory:
            dump = Path(directory) / 'spell_area.sql'
            dump.write_text("INSERT INTO `spell_area` VALUES (100616,5171,0,29401,0,18446744073709551615,2,3,0,43);\n")
            self.assertEqual(load_table(conn, 'spell_area', dump, 'tdb_'), 1)
        self.assertEqual(conn.execute('SELECT spell, area, quest_end, racemask, flags, quest_end_status FROM tdb_spell_area').fetchall(), [(100616,5171,29401,-1,3,43)])


if __name__ == '__main__':
    unittest.main()
