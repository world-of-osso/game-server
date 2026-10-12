import sqlite3
import tempfile
import unittest
from pathlib import Path
from contextlib import closing

from import_world_quests import QUEST_COLUMNS, copy_and_import, import_definitions


def fixture(conn):
    text = {"LogTitle", "LogDescription", "QuestDescription", "AreaDescription", "QuestCompletionLog"}
    columns = {source: "TEXT DEFAULT ''" if source in text else "INTEGER DEFAULT 0" for source in QUEST_COLUMNS.values()}
    columns.update(ManagedWorldStateID="INTEGER DEFAULT 0", FlagsEx="INTEGER DEFAULT 0", QuestInfoID="INTEGER DEFAULT 0")
    conn.execute("CREATE TABLE tdb_quest_template (" + ",".join(f'{k} {v}' for k, v in columns.items()) + ")")
    targets = {target: "TEXT DEFAULT ''" if source in text else "INTEGER DEFAULT 0" for target, source in QUEST_COLUMNS.items()}
    targets.update(QuestLevel="INTEGER DEFAULT 0", MinLevel="INTEGER DEFAULT 0", RewardMoney="INTEGER DEFAULT 0")
    for n in range(1, 5):
        targets.update({f"RequiredNpcOrGo{n}": "INTEGER DEFAULT 0", f"RequiredNpcOrGoCount{n}": "INTEGER DEFAULT 0", f"ObjectiveText{n}": "TEXT DEFAULT ''"})
    for n in range(1, 7):
        targets.update({f"RequiredItemId{n}": "INTEGER DEFAULT 0", f"RequiredItemCount{n}": "INTEGER DEFAULT 0"})
    targets["RequiredPlayerKills"] = "INTEGER DEFAULT 0"
    conn.execute("CREATE TABLE content_quest_template (" + ",".join(f'{k} {v}' for k, v in targets.items()) + ")")
    conn.executescript("""
        CREATE TABLE quest_info (ID INTEGER, Modifiers INTEGER);
        INSERT INTO quest_info VALUES (112,256), (1,0);
        CREATE TABLE quest_v2 (ID INTEGER);
        INSERT INTO quest_v2 VALUES (52847), (52848), (999);
        CREATE TABLE content_tuning_level (ID INTEGER, minlevel INTEGER, maxlevel INTEGER);
        INSERT INTO content_tuning_level VALUES (464,10,50);
        CREATE TABLE quest_money_reward (ID INTEGER, Difficulty_0 INTEGER);
        INSERT INTO quest_money_reward VALUES (50,0);
        CREATE TABLE tdb_quest_objectives (QuestID INTEGER, ID INTEGER, Type INTEGER, ObjectID INTEGER, Amount INTEGER, Description TEXT, "Order" INTEGER, StorageIndex INTEGER);
        INSERT INTO tdb_quest_objectives VALUES (52847,340072,0,138122,1,'',0,0);
        CREATE TABLE tdb_creature_template (entry INTEGER, name TEXT);
        INSERT INTO tdb_creature_template VALUES (138122,'Doom''s Howl');
        CREATE TABLE tdb_quest_poi (QuestID INTEGER, Idx1 INTEGER, BlobIndex INTEGER, ObjectiveIndex INTEGER, MapID INTEGER, UiMapID INTEGER, Priority INTEGER, Flags INTEGER, VerifiedBuild INTEGER);
        INSERT INTO tdb_quest_poi VALUES (52847,0,0,32,0,14,0,2,66384);
        CREATE TABLE tdb_quest_poi_points (QuestID INTEGER, Idx1 INTEGER, Idx2 INTEGER, X INTEGER, Y INTEGER, VerifiedBuild INTEGER);
        INSERT INTO tdb_quest_poi_points VALUES (52847,0,0,-1061,-2482,66384);
        CREATE TABLE content_quest_poi (QuestID INTEGER, id INTEGER, ObjectiveIndex INTEGER, MapID INTEGER, WorldMapAreaId INTEGER, Floor INTEGER, Priority INTEGER, Flags INTEGER, VerifiedBuild INTEGER);
        CREATE TABLE content_quest_poi_points (QuestID INTEGER, Idx1 INTEGER, Idx2 INTEGER, X INTEGER, Y INTEGER, VerifiedBuild INTEGER);
    """)
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, QuestSortID, ContentTuningID, LogTitle, AllowableRaces) VALUES (52847,112,113,9734,464,'Doom''s Howl',6130900294268439629)")
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, QuestSortID, LogTitle) VALUES (52848,112,0,9734,'Unknown duration')")
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, LogTitle) VALUES (999,1,113,'Ordinary quest')")
    for index in range(1,10):
        conn.execute(f"ALTER TABLE quest_money_reward ADD COLUMN Difficulty_{index} INTEGER DEFAULT 0")
    conn.commit()


def receipt(duration=604800):
    return {"managed_world_states": {"113": {"ID": 113, "UpTimeSecs": duration}}}


class WorldQuestImportTests(unittest.TestCase):
    def test_world_quest_import_preserves_authentic_duration_objective_pin_and_race_mask(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            self.assertEqual(import_definitions(conn, receipt()), [52847])
            self.assertEqual(conn.execute("SELECT quest_id,zone_id,duration_seconds FROM content_world_quest").fetchall(), [(52847,9734,604800)])
            row = conn.execute("SELECT ID,LogTitle,MinLevel,RequiredNpcOrGo1,RequiredNpcOrGoCount1,AllowableRaces FROM content_quest_template").fetchone()
            self.assertEqual(row, (52847,"Doom's Howl",10,138122,1,6130900294268439629))
            self.assertEqual(conn.execute("SELECT X,Y FROM content_quest_poi_points").fetchall(), [(-1061,-2482)])
            self.assertEqual(conn.execute("SELECT MapID,WorldMapAreaId FROM content_quest_poi").fetchall(), [(0,14)])
            self.assertEqual(import_definitions(conn, receipt()), [52847])
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 1)

    def test_world_quest_import_rejects_invalid_duration_without_guess(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            with self.assertRaisesRegex(ValueError, "duration"):
                import_definitions(conn, receipt(0))
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 0)

    def test_world_quest_import_private_copy_never_mutates_or_overwrites_source(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory)/"source.db", Path(directory)/"private.db"
            with closing(sqlite3.connect(source)) as conn:
                fixture(conn)
            before = source.read_bytes()
            self.assertEqual(copy_and_import(source, output, receipt()), [52847])
            self.assertEqual(source.read_bytes(), before)
            with self.assertRaises(FileExistsError):
                copy_and_import(source, output, receipt())
            with self.assertRaises(ValueError):
                copy_and_import(source, source, receipt())


if __name__ == "__main__":
    unittest.main()
