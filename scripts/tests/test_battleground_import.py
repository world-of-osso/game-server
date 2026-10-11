import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'import-battlegrounds.py'
# Exact authored schema/rows abbreviated to the source fields needed by this fixture.
SOURCE = '''CREATE TABLE `battleground_template` (
  `ID` int unsigned NOT NULL,
  `AllianceStartLoc` int unsigned NOT NULL,
  `HordeStartLoc` int unsigned NOT NULL,
  `StartMaxDist` float NOT NULL,
  `Weight` tinyint unsigned NOT NULL,
  `ScriptName` varchar(64) NOT NULL,
  `Comment` varchar(32) NOT NULL,
  PRIMARY KEY (`ID`)
) ENGINE=InnoDB;
INSERT INTO `battleground_template` VALUES (1014,7051,7050,0,1,'','Warsong Gulch');
CREATE TABLE `world_state` (
  `ID` int unsigned NOT NULL,
  `DefaultValue` int NOT NULL,
  `MapIDs` text,
  `AreaIDs` text,
  `ScriptName` varchar(64) NOT NULL,
  `Comment` text,
  PRIMARY KEY (`ID`)
) ENGINE=InnoDB;
INSERT INTO `world_state` VALUES (17303,3,'489,726,2106',NULL,'','Warsong Gulch/Twin Peaks - Max flag captures (new)');
'''


class BattlegroundImportTests(unittest.TestCase):
    def test_authored_metadata_round_trip_and_idempotent_reapply(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source.sql'
            database = root / 'fixture.sqlite'
            source.write_text(SOURCE)
            with sqlite3.connect(database) as conn:
                conn.execute('CREATE TABLE unrelated (value TEXT)')
                conn.execute("INSERT INTO unrelated VALUES ('keep')")
            for _ in range(2):
                result = subprocess.run([sys.executable, str(SCRIPT), '--dump', str(source), '--db', str(database)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(database) as conn:
                self.assertEqual(conn.execute('SELECT * FROM tdb_battleground_template').fetchall(), [(1014,7051,7050,0,1,'','Warsong Gulch')])
                self.assertEqual(conn.execute('SELECT DefaultValue,MapIDs FROM tdb_world_state WHERE ID=17303').fetchone(), (3,'489,726,2106'))
                self.assertEqual(conn.execute('SELECT value FROM unrelated').fetchall(), [('keep',)])


if __name__ == '__main__':
    unittest.main()
