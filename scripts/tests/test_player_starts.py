import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("player_starts", SCRIPTS / "import-player-starts.py")
starts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(starts)

# TDB1210 playercreateinfo: authentic Tauren Warrior and Human Mage rows.
ROWS = """INSERT INTO `playercreateinfo` VALUES (6,1,1,-2915.55,-257.347,59.2693,0.302378,2175,-10.7291,-7.14635,8.73113,1.56321,30,NULL,NULL,2486),(1,8,0,-8914.57,-133.909,80.5378,5.10444,2175,11.1301,-0.417182,5.18741,3.14843,29,NULL,NULL,2236);\n"""

class PlayerStartsTests(unittest.TestCase):
    def test_import_preserves_source_positions_orientation_and_npe_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            dump = Path(directory) / "world.sql"
            dump.write_text("INSERT INTO `unrelated` VALUES (99);\n" + ROWS)
            with sqlite3.connect(":memory:") as conn:
                conn.execute("CREATE TABLE unrelated (id INTEGER)")
                conn.execute("INSERT INTO unrelated VALUES (7)")
                self.assertEqual(starts.import_starts(conn, dump), 2)
                self.assertEqual(conn.execute("SELECT * FROM tdb_playercreateinfo WHERE race=6 AND class=1").fetchone(),
                    (6, 1, 1, -2915.55, -257.347, 59.2693, 0.302378, 2175, -10.7291, -7.14635, 8.73113, 1.56321, 30, None, None, 2486))
                self.assertEqual(conn.execute("SELECT id FROM unrelated").fetchall(), [(7,)])

    def test_bad_source_leaves_previous_table_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            dump = Path(directory) / "world.sql"
            dump.write_text(ROWS)
            with sqlite3.connect(":memory:") as conn:
                starts.import_starts(conn, dump)
                dump.write_text("INSERT INTO `playercreateinfo` VALUES (6,1,1);\n")
                with self.assertRaises(ValueError):
                    starts.import_starts(conn, dump)
                self.assertEqual(conn.execute("SELECT count(*) FROM tdb_playercreateinfo").fetchone()[0], 2)

    def test_live_destination_and_symlink_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            live = Path(directory) / "world.db"
            live.write_bytes(b"untouched")
            alias = Path(directory) / "scratch.db"
            alias.symlink_to(live)
            for destination in (live, alias):
                with self.assertRaises(ValueError):
                    starts.validate_destination(destination)
            self.assertEqual(live.read_bytes(), b"untouched")

if __name__ == "__main__":
    unittest.main()
