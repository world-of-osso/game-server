import sqlite3
import csv
import subprocess
import sys
import json
import tempfile
import unittest
from pathlib import Path
from contextlib import closing

from import_world_quests import QUEST_COLUMNS, copy_and_import, import_definitions


def fixture(conn):
    text = {"LogTitle", "LogDescription", "QuestDescription", "AreaDescription", "QuestCompletionLog"}
    columns = {source: "TEXT DEFAULT ''" if source in text else "INTEGER DEFAULT 0" for source in QUEST_COLUMNS.values()}
    columns.update(ManagedWorldStateID="INTEGER DEFAULT 0", FlagsEx="INTEGER DEFAULT 0", QuestInfoID="INTEGER DEFAULT 0", RewardCurrencyID1="INTEGER DEFAULT 0", RewardCurrencyQty1="INTEGER DEFAULT 0", RewardFactionCapIn1="INTEGER DEFAULT 0")
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
        CREATE TABLE tdb_quest_objectives (QuestID INTEGER, ID INTEGER, Type INTEGER, ObjectID INTEGER, Amount INTEGER, Description TEXT, "Order" INTEGER, StorageIndex INTEGER, Flags INTEGER DEFAULT 0, ProgressBarWeight REAL DEFAULT 0);
        INSERT INTO tdb_quest_objectives VALUES (52847,340072,0,138122,1,'',0,0,0,0);
        CREATE TABLE criteria_tree(ID INTEGER,Parent INTEGER,Amount INTEGER,Operator INTEGER,CriteriaID INTEGER,Flags INTEGER);
        CREATE TABLE criteria(ID INTEGER,Type INTEGER,Asset INTEGER,ModifierTreeId INTEGER DEFAULT 0,StartEvent INTEGER DEFAULT 0,StartAsset INTEGER DEFAULT 0,StartTimer INTEGER DEFAULT 0,FailEvent INTEGER DEFAULT 0,FailAsset INTEGER DEFAULT 0,Flags INTEGER DEFAULT 32,EligibilityWorldStateID INTEGER DEFAULT 0,EligibilityWorldStateValue INTEGER DEFAULT 0);
        CREATE TABLE tdb_creature_template_difficulty (Entry INTEGER,DifficultyID INTEGER,CreatureDifficultyID INTEGER);
        CREATE TABLE tdb_creature_template (entry INTEGER, name TEXT);
        INSERT INTO tdb_creature_template VALUES (138122,'Doom''s Howl');
        CREATE TABLE tdb_quest_poi (QuestID INTEGER, Idx1 INTEGER, BlobIndex INTEGER, ObjectiveIndex INTEGER, MapID INTEGER, UiMapID INTEGER, Priority INTEGER, Flags INTEGER, VerifiedBuild INTEGER);
        INSERT INTO tdb_quest_poi VALUES (52847,0,0,32,0,14,0,2,66384);
        CREATE TABLE tdb_quest_poi_points (QuestID INTEGER, Idx1 INTEGER, Idx2 INTEGER, X INTEGER, Y INTEGER, VerifiedBuild INTEGER);
        INSERT INTO tdb_quest_poi_points VALUES (52847,0,0,-1061,-2482,66384);
        CREATE TABLE content_quest_poi (QuestID INTEGER, id INTEGER, ObjectiveIndex INTEGER, MapID INTEGER, WorldMapAreaId INTEGER, Floor INTEGER, Priority INTEGER, Flags INTEGER, VerifiedBuild INTEGER);
        CREATE TABLE content_quest_poi_points (QuestID INTEGER, Idx1 INTEGER, Idx2 INTEGER, X INTEGER, Y INTEGER, VerifiedBuild INTEGER);
    """)
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, QuestSortID, ContentTuningID, LogTitle, AllowableRaces, RewardCurrencyID1, RewardCurrencyQty1, RewardFactionID1, RewardFactionOverride1, RewardFactionCapIn1) VALUES (52847,112,113,9734,464,'Doom''s Howl',6130900294268439629,1717,1,2159,7500,7)")
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, QuestSortID, LogTitle) VALUES (52848,112,0,9734,'Unknown duration')")
    conn.execute("INSERT INTO tdb_quest_template (ID, QuestInfoID, ManagedWorldStateID, LogTitle) VALUES (999,1,113,'Ordinary quest')")
    for index in range(1,10):
        conn.execute(f"ALTER TABLE quest_money_reward ADD COLUMN Difficulty_{index} INTEGER DEFAULT 0")
    conn.execute('ALTER TABLE tdb_quest_objectives ADD COLUMN Flags2 INTEGER DEFAULT 0')
    conn.commit()


def receipt(duration=604800):
    return {"managed_world_states": {"113": {"ID": 113, "UpTimeSecs": duration}}}


def areas():
    return {"creature_labels": [], "xp": [dict(ID=30, **{f'Difficulty_{n}': 1234 if n == 5 else 0 for n in range(10)})],
            "faction_rewards": {1: [0,10,25,75,150,250,350,500,1000,0], 2: [0,-10,-25,-75,-150,-250,-350,-500,-1000,0]},
            "blobs": [{"ID": 7, "QuestID": 52847, "MapID": 0, "UiMapID": 14, "NumPoints": 4},
                      {"ID": 8, "QuestID": 52848, "MapID": 0, "UiMapID": 14, "NumPoints": 1}],
            "points": [{"ID": n, "QuestPOIBlobID": 7, "X": x, "Y": y, "Z": 0}
                       for n, (x, y) in enumerate([(-1065,-2486),(-1057,-2486),(-1057,-2478),(-1065,-2478)])]
                      + [{"ID": 9, "QuestPOIBlobID": 8, "X": 0, "Y": 0, "Z": 0}]}


class WorldQuestImportTests(unittest.TestCase):
    def test_cli_imports_one_csv_build_without_separately_pinned_binary_tables(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source.db'
            with closing(sqlite3.connect(source)) as conn:
                fixture(conn)
            poi = areas()
            exports = {
                'QuestPOIBlob.csv': (list(poi['blobs'][0]), poi['blobs']),
                'QuestPOIPoint.csv': (list(poi['points'][0]), poi['points']),
                'QuestXP.csv': (list(poi['xp'][0]), poi['xp']),
                'CreatureLabel.csv': (['ID', 'LabelID', 'CreatureDifficultyID'], []),
                'QuestFactionReward.csv': (
                    ['ID'] + [f'Difficulty_{n}' for n in range(10)],
                    [dict(ID=key, **{f'Difficulty_{n}': value for n, value in enumerate(values)})
                     for key, values in poi['faction_rewards'].items()]),
            }
            for filename, (columns, rows) in exports.items():
                with (root / filename).open('w', newline='') as stream:
                    writer = csv.DictWriter(stream, fieldnames=columns)
                    writer.writeheader()
                    writer.writerows(rows)
            receipt_path = root / 'duration.json'
            receipt_path.write_text(json.dumps(receipt()))
            output = root / 'world.db'
            result = subprocess.run([
                sys.executable, str(Path(__file__).with_name('import_world_quests.py')),
                '--source-db', str(source), '--output-db', str(output),
                '--poi-db2-dir', str(root), '--duration-receipt', str(receipt_path),
            ], capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)['imported'], [52847])
            with closing(sqlite3.connect(output)) as conn:
                self.assertEqual(conn.execute('SELECT z FROM content_world_quest_area ORDER BY point_index').fetchall(), [(0.0,)] * 4)
                self.assertEqual(conn.execute('SELECT amount FROM content_world_quest_reward WHERE kind="reputation"').fetchall(), [(75,)])
            with closing(sqlite3.connect(source)) as conn:
                self.assertEqual(conn.execute("SELECT count(*) FROM sqlite_master WHERE name='content_world_quest_area'").fetchone()[0], 0)

    def test_source_polygon_heights_are_preserved_in_area_rows(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            poi = areas()
            heights = [-1582, -1614, -1599, -1577]
            for point, height in zip(poi['points'], heights):
                point['Z'] = height
            self.assertEqual(import_definitions(conn, receipt(), poi), [52847])
            cursor = conn.execute('SELECT * FROM content_world_quest_area ORDER BY point_index')
            names = [column[0] for column in cursor.description]
            rows = [dict(zip(names, row)) for row in cursor.fetchall()]
            self.assertEqual([row.get('z') for row in rows], heights,
                             'QuestPOIPoint source Z must not be discarded or replaced with zero')

    def test_missing_source_height_is_rejected_not_guessed(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            poi = areas()
            del poi['points'][0]['Z']
            with self.assertRaises(KeyError):
                import_definitions(conn, receipt(), poi)

    def test_polygon_policy_duration_and_point_only_exclusion(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            poi = areas()
            poi['blobs'].append({'ID': 99, 'QuestID': 999, 'MapID': 0, 'UiMapID': 14, 'NumPoints': 1})
            poi["blobs"][0]["QuestID"] = 52848
            poi["blobs"][1]["QuestID"] = 52847
            conn.execute("INSERT INTO tdb_quest_objectives VALUES (52848,340073,0,138122,1,'',0,0,0,0,0)")
            self.assertEqual(import_definitions(conn, receipt(), poi, 172800), [52848])
            self.assertEqual(conn.execute("SELECT duration_seconds,duration_source FROM content_world_quest").fetchall(), [(172800, 'policy')])
            self.assertEqual(conn.execute("SELECT map_id,ui_map_id,x,y FROM content_world_quest_area ORDER BY point_index").fetchall(),
                             [(0,14,-1065,-2486),(0,14,-1057,-2486),(0,14,-1057,-2478),(0,14,-1065,-2478)])

    def test_variable_objective_rows_preserve_authentic_optional_weighted_metadata(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            conn.execute('DELETE FROM tdb_quest_objectives')
            conn.executemany('INSERT INTO tdb_quest_objectives VALUES (?,?,?,?,?,?,?,?,?,?,?)',
                             [(52847,340072+n,0,138122+n,12,'Contributor',n,n,92,9.,0) for n in range(7)])
            conn.execute("INSERT INTO tdb_quest_objectives VALUES (52847,340090,15,0,1,'Progress',7,7,2,0,0)")
            self.assertEqual(import_definitions(conn,receipt(),areas()),[52847])
            rows=conn.execute('SELECT kind,object_id,amount,flags,weight FROM content_world_quest_objective ORDER BY ordinal').fetchall()
            self.assertEqual(len(rows),8)
            self.assertEqual(rows[0],(0,138122,12,92,9.))
            self.assertEqual(rows[-1],(15,0,1,2,0.))

    def test_missing_xp_source_is_rejected_not_imported_as_empty(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            poi=areas()
            del poi['xp']
            with self.assertRaises(KeyError):
                import_definitions(conn,receipt(),poi)

    def test_missing_reputation_cap_is_rejected_not_guessed(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            conn.execute('ALTER TABLE tdb_quest_template DROP COLUMN RewardFactionCapIn1')
            with self.assertRaises(KeyError):
                import_definitions(conn,receipt(),areas())

    def test_single_leaf_quest_criteria_preserve_kill_and_aura_sources(self):
        for kind,asset in [(0,225891),(69,420847)]:
            with closing(sqlite3.connect(':memory:')) as conn:
                fixture(conn)
                conn.execute('UPDATE tdb_quest_objectives SET Type=14,ObjectID=163278,Amount=1')
                conn.executescript('INSERT INTO criteria_tree VALUES(163278,0,0,4,0,0),(163279,163278,1,0,68193,0)')
                conn.execute('INSERT INTO criteria(ID,Type,Asset) VALUES(68193,?,?)',(kind,asset))
                self.assertEqual(import_definitions(conn,receipt(),areas()),[52847])
                self.assertEqual(conn.execute('SELECT root_id,criterion_id,kind,asset FROM content_world_quest_criterion').fetchall(),[(163278,68193,kind,asset)])
                conn.execute('UPDATE criteria SET StartTimer=10')
                self.assertEqual(import_definitions(conn,receipt(),areas()),[])

    def test_authentic_label_membership_admits_only_resolved_labels(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            conn.execute('UPDATE tdb_quest_objectives SET Type=21,ObjectID=3520,Amount=5')
            conn.execute('INSERT INTO tdb_creature_template_difficulty VALUES(222677,0,273474)')
            poi=areas()
            poi['creature_labels']=[[58889,3520,273474]]
            self.assertEqual(import_definitions(conn,receipt(),poi),[52847])
            self.assertEqual(conn.execute('SELECT label_id,creature_entry,difficulty_id FROM content_world_quest_creature_label').fetchall(),[(3520,222677,0)])
            conn.execute('UPDATE tdb_quest_objectives SET ObjectID=4346')
            self.assertEqual(import_definitions(conn,receipt(),poi),[])

    def test_authoritative_talk_and_player_kill_objectives_are_admitted(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            conn.execute('UPDATE tdb_quest_objectives SET Type=3,ObjectID=138122,Amount=1')
            conn.execute("INSERT INTO tdb_quest_objectives VALUES(52847,340073,9,0,10,'Players defeated',1,1,128,0,0)")
            self.assertEqual(import_definitions(conn,receipt(),areas()),[52847])
            self.assertEqual(conn.execute('SELECT kind,object_id,amount,flags FROM content_world_quest_objective ORDER BY ordinal').fetchall(),[(3,138122,1,0),(9,0,10,128)])

    def test_quest_bound_item_flags_are_preserved(self):
        with closing(sqlite3.connect(':memory:')) as conn:
            fixture(conn)
            conn.execute('UPDATE tdb_quest_objectives SET Type=1,ObjectID=217343,Amount=20,Flags2=1')
            self.assertEqual(import_definitions(conn,receipt(),areas()),[52847])
            self.assertEqual(conn.execute('SELECT kind,object_id,amount,flags2 FROM content_world_quest_objective').fetchall(),[(1,217343,20,1)])

    def test_unsupported_objective_is_excluded_not_silently_completed(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            conn.execute("INSERT INTO tdb_quest_objectives VALUES (52847,340073,99,138122,1,'',1,1,0,0,0)")
            self.assertEqual(import_definitions(conn, receipt(), areas()), [])
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 0)

    def test_world_quest_import_preserves_authentic_duration_objective_pin_and_race_mask(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            self.assertEqual(import_definitions(conn, receipt(), areas()), [52847])
            self.assertEqual(conn.execute("SELECT quest_id,zone_id,duration_seconds FROM content_world_quest").fetchall(), [(52847,9734,604800)])
            row = conn.execute("SELECT ID,LogTitle,MinLevel,AllowableRaces FROM content_quest_template").fetchone()
            self.assertEqual(row, (52847,"Doom's Howl",10,6130900294268439629))
            self.assertEqual(conn.execute('SELECT kind,object_id,amount FROM content_world_quest_objective').fetchall(),[(0,138122,1)])
            source = json.loads(conn.execute("SELECT source_template_json FROM content_world_quest").fetchone()[0])
            self.assertEqual((source["RewardCurrencyID1"],source["RewardCurrencyQty1"]),(1717,1))
            self.assertEqual((source["RewardFactionID1"],source["RewardFactionOverride1"],source["RewardFactionCapIn1"]),(2159,7500,7))
            self.assertEqual(conn.execute('SELECT kind,reward_id,amount,cap FROM content_world_quest_reward ORDER BY kind').fetchall(),
                             [('currency',1717,1,0), ('reputation',2159,75,7)])
            self.assertEqual(conn.execute('SELECT amount FROM content_world_quest_xp WHERE level=30 AND difficulty=5').fetchone(), (1234,))
            self.assertEqual(conn.execute("SELECT X,Y FROM content_quest_poi_points").fetchall(), [(-1061,-2482)])
            self.assertEqual(conn.execute("SELECT MapID,WorldMapAreaId FROM content_quest_poi").fetchall(), [(0,14)])
            self.assertEqual(import_definitions(conn, receipt(), areas()), [52847])
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 1)

    def test_world_quest_import_rejects_invalid_duration_without_guess(self):
        with closing(sqlite3.connect(":memory:")) as conn:
            fixture(conn)
            with self.assertRaisesRegex(ValueError, "duration"):
                import_definitions(conn, receipt(0), areas())
            self.assertEqual(conn.execute("SELECT count(*) FROM content_quest_template").fetchone()[0], 0)

    def test_world_quest_import_private_copy_never_mutates_or_overwrites_source(self):
        with tempfile.TemporaryDirectory() as directory:
            source, output = Path(directory)/"source.db", Path(directory)/"private.db"
            with closing(sqlite3.connect(source)) as conn:
                fixture(conn)
            before = source.read_bytes()
            self.assertEqual(copy_and_import(source, output, receipt(), areas()), [52847])
            self.assertEqual(source.read_bytes(), before)
            with self.assertRaises(FileExistsError):
                copy_and_import(source, output, receipt(), areas())
            with self.assertRaises(ValueError):
                copy_and_import(source, source, receipt(), areas())


if __name__ == "__main__":
    unittest.main()
