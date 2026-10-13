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
CREATE TABLE `npc_spellclick_spells` (
  `npc_entry` int unsigned NOT NULL,
  `spell_id` int unsigned NOT NULL,
  `cast_flags` tinyint unsigned NOT NULL,
  `user_type` smallint unsigned NOT NULL,
  PRIMARY KEY (`npc_entry`,`spell_id`)
) ENGINE=InnoDB;
INSERT INTO `npc_spellclick_spells` VALUES (60283,124491,1,0);
CREATE TABLE `transports` (
  `guid` bigint unsigned NOT NULL DEFAULT '0',
  `entry` int unsigned NOT NULL DEFAULT '0',
  `name` mediumtext,
  `phaseUseFlags` tinyint unsigned NOT NULL DEFAULT '0',
  `phaseid` int NOT NULL DEFAULT '0',
  `phasegroup` int NOT NULL DEFAULT '0',
  `ScriptName` varchar(64) NOT NULL DEFAULT '',
  PRIMARY KEY (`guid`)
) ENGINE=InnoDB;
INSERT INTO `transports` VALUES (35,278407,'Sword of Dawn',0,0,0,'transport_seething_shore'),(36,279254,'The warbringer',0,0,0,'transport_seething_shore');
CREATE TABLE `areatrigger_create_properties_orbit` (
  `AreaTriggerCreatePropertiesId` int unsigned NOT NULL,
  `IsCustom` tinyint unsigned NOT NULL,
  `ExtraTimeForBlending` int NOT NULL DEFAULT '0',
  `CircleRadius` float NOT NULL DEFAULT '0',
  `BlendFromRadius` float NOT NULL DEFAULT '0',
  `InitialAngle` float NOT NULL DEFAULT '0',
  `ZOffset` float NOT NULL DEFAULT '0',
  `CounterClockwise` tinyint unsigned NOT NULL DEFAULT '0',
  `CanLoop` tinyint unsigned NOT NULL DEFAULT '0',
  `VerifiedBuild` int unsigned NOT NULL DEFAULT '0',
  PRIMARY KEY (`AreaTriggerCreatePropertiesId`,`IsCustom`)
) ENGINE=InnoDB;
INSERT INTO `areatrigger_create_properties_orbit` VALUES (5838,0,0,0,0,0,0,0,1,52129),(13339,0,0,5,0,0,0,0,1,55165);
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
                self.assertEqual(conn.execute('SELECT * FROM tdb_npc_spellclick_spells').fetchall(), [(60283,124491,1,0)])
                self.assertEqual(conn.execute('SELECT guid,entry,name,ScriptName FROM tdb_transports ORDER BY guid').fetchall(), [
                    (35,278407,'Sword of Dawn','transport_seething_shore'),
                    (36,279254,'The warbringer','transport_seething_shore'),
                ])
                self.assertEqual(conn.execute('SELECT CircleRadius,BlendFromRadius,CounterClockwise,CanLoop FROM tdb_areatrigger_create_properties_orbit WHERE AreaTriggerCreatePropertiesId=13339').fetchone(), (5.0,0.0,0,1))


if __name__ == '__main__':
    unittest.main()
