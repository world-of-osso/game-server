"""Guild reward import on a private copy: real TDB row shapes, re-runnable, copy-only."""
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

DUMP = """-- MySQL dump
INSERT INTO `guild_rewards` VALUES (61931,4,0,15000000),(62287,6,33555378,200000000),(63138,7,4294967295,3000000);
INSERT INTO `gossip_menu` VALUES (1,2,3);
INSERT INTO `guild_rewards_req_achievements` VALUES (62287,4949),(62287,5158),(63138,5812),(63138,5892);
"""


class ImportGuildRewardsTests(unittest.TestCase):
    def run_import(self, db, dump):
        return subprocess.run(
            [sys.executable, str(SCRIPTS / "import_guild_rewards.py"), "--db", str(db), "--dump", str(dump)],
            capture_output=True, text=True, check=False,
        )

    def test_copy_receives_rewards_and_achievements_and_reimport_replaces(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dump = root / "TDB_full_world.sql"
            dump.write_text(DUMP)
            db = root / "copy.sqlite"
            sqlite3.connect(db).close()
            for _ in range(2):
                result = self.run_import(db, dump)
                self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.split("\n")[:2], [
                "tdb_guild_rewards: 3", "tdb_guild_rewards_req_achievements: 4"])
            with closing(sqlite3.connect(db)) as conn:
                self.assertEqual(conn.execute(
                    "SELECT ItemID, MinGuildRep, RaceMask, Cost FROM tdb_guild_rewards ORDER BY ItemID").fetchall(),
                    [(61931, 4, 0, 15000000), (62287, 6, 33555378, 200000000), (63138, 7, 4294967295, 3000000)])
                self.assertEqual(conn.execute(
                    "SELECT AchievementRequired FROM tdb_guild_rewards_req_achievements WHERE ItemID=62287 ORDER BY 1").fetchall(),
                    [(4949,), (5158,)])
            self.assertEqual((root / "guild_rewards.sql").read_text().count("INSERT INTO `guild_rewards` "), 1)
            self.assertNotIn("gossip_menu", (root / "guild_rewards_req_achievements.sql").read_text())

    def test_refuses_a_file_named_world_db(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dump = root / "dump.sql"
            dump.write_text(DUMP)
            db = root / "world.db"
            sqlite3.connect(db).close()
            result = self.run_import(db, dump)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("not world.db", result.stderr)
            with closing(sqlite3.connect(db)) as conn:
                self.assertEqual(conn.execute("SELECT count(*) FROM sqlite_master").fetchone(), (0,))


if __name__ == "__main__":
    unittest.main()
