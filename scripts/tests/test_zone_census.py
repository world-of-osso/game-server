"""Concrete zone attribution and ranked census, including unassigned source rows."""
import csv
from contextlib import closing
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import world_population


class ZoneCensusTest(unittest.TestCase):
    def test_db2_zone_rollup_coordinate_attribution_and_quest_ranking(self):
        with tempfile.TemporaryDirectory() as directory, closing(sqlite3.connect(":memory:")) as db:
            root = Path(directory)
            for filename, columns, rows in [
                ("UiMap.csv", ["ID", "Name_lang", "Type", "System"], [[100, "Coast", 3, 0], [101, "Hills", 3, 0]]),
                ("UiMapAssignment.csv", ["UiMapID", "MapID", "AreaID", "WMODoodadPlacementID", "WMOGroupID", "Region_0", "Region_1", "Region_3", "Region_4"],
                 [[100, 7, 10, 0, 0, 0, 0, 10, 10], [101, 7, 20, 0, 0, 10, 0, 20, 10]])]:
                with (root / filename).open("w", newline="") as handle:
                    writer = csv.writer(handle)
                    writer.writerow(columns)
                    writer.writerows(rows)
            db.executescript("""
                CREATE TABLE area_table(ID,AreaName_lang,ContinentID,ParentAreaID);
                INSERT INTO area_table VALUES(10,'Coast',7,0),(11,'Village',7,10),(20,'Hills',7,0);
                CREATE TABLE tdb_creature(guid,id,map,zoneId,areaId,position_x,position_y,phaseUseFlags,PhaseId,PhaseGroup,terrainSwapMap,spawnDifficulties);
                INSERT INTO tdb_creature VALUES(1,40,7,0,11,1,1,0,0,0,-1,'0'),(2,41,7,0,0,15,5,0,0,0,-1,'0'),(3,42,7,0,0,99,99,0,0,0,-1,'0');
                CREATE TABLE content_creature(guid,id1,map,zoneId,areaId,position_x,position_y);
                INSERT INTO content_creature VALUES(20000001,40,7,0,11,1,1);
                CREATE TABLE tdb_gameobject AS SELECT * FROM tdb_creature WHERE 0;
                CREATE TABLE content_gameobject(guid,id,map,zoneId,areaId,position_x,position_y);
                CREATE TABLE tdb_quest_template(ID,QuestSortID);
                INSERT INTO tdb_quest_template VALUES(50,10),(51,11),(52,20);
                CREATE TABLE content_quest_template AS SELECT * FROM tdb_quest_template WHERE ID=50;
                CREATE TABLE tdb_creature_queststarter(id,quest);
                INSERT INTO tdb_creature_queststarter VALUES(40,50),(40,51),(41,52);
                CREATE TABLE content_creature_queststarter AS SELECT * FROM tdb_creature_queststarter WHERE quest=50;
                CREATE TABLE tdb_creature_questender AS SELECT * FROM tdb_creature_queststarter;
                CREATE TABLE content_creature_questender AS SELECT * FROM content_creature_queststarter;
                CREATE TABLE tdb_gameobject_queststarter(id,quest);
                CREATE TABLE content_gameobject_queststarter(id,quest);
                CREATE TABLE tdb_gameobject_questender(id,quest);
                CREATE TABLE content_gameobject_questender(id,quest);
            """)
            rows = world_population.zone_census(db, [7], root)
            self.assertEqual([10,20,0], [r["zone"] for r in rows])
            coast, hills, unassigned = rows
            self.assertEqual((2,1,1,1), (coast["tdb_quests"], coast["content_quests"], coast["tdb_creatures"], coast["content_creatures"]))
            self.assertEqual((1,1,0), (hills["tdb_creatures"], hills["tdb_inferred_creatures"], hills["content_creatures"]))
            self.assertEqual(1, unassigned["tdb_creatures"])
            self.assertEqual((1,1), (coast["tdb_quest_givers"], coast["content_quest_givers"]))
            self.assertEqual([100], coast["ui_maps"])


if __name__ == "__main__":
    unittest.main()
