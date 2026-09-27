"""Exercise the real importer without touching the development world database."""

import csv
import json
import os
from pathlib import Path
import shutil
import sqlite3
import struct
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import content_policy  # noqa: E402
import retail_content  # noqa: E402
import db2  # noqa: E402

AZEROTHCORE_SQL = Path(os.environ.get("AZEROTHCORE_SQL", "/home/osso/Repos/azerothcore/data/sql/base/db_world"))

PINNED_CSV_ROOT = Path(os.environ.get("DB2_CSV_ROOT", db2.DEFAULT_CSV_ROOT))
PINNED_BUILD_DIR = PINNED_CSV_ROOT / db2.DEFAULT_BUILD
TDB_DIR = Path(os.environ.get("TDB_DIR", SCRIPTS.parent / "data/reference/tdb1210"))
TRAINER_TDB_TABLES = ("skill_tiers", "trainer", "trainer_spell", "creature_trainer")


def tdb_tables():
    """Table names in import-azerothcore.sh TDB_TABLES."""
    script = (SCRIPTS / "import-azerothcore.sh").read_text()
    return script.split("TDB_TABLES=(", 1)[1].split(")", 1)[0].split()


def tdb_creature(guid, entry, map_id, x, y, z, difficulties="0", phase_id=0, phase_group=0, terrain_swap=-1,
                 modelid=0, unit_flags="NULL"):
    """One TDB1210 `creature` row (28 columns, TrinityCore DDL order)."""
    return (f"({guid},{entry},{map_id},0,0,'{difficulties}',0,{phase_id},{phase_group},{terrain_swap},{modelid},1,"
            f"{x},{y},{z},1.5,300,0,0,100,0,NULL,{unit_flags},NULL,NULL,'',NULL,66044)")


def tdb_template(entry, name, faction, npcflag, classification=0, unit_class=1, flags_extra=0, script=""):
    """One TDB1210 `creature_template` row (42 columns, TrinityCore DDL order)."""
    return (f"({entry},0,0,'{name}',NULL,NULL,NULL,NULL,0,0,{faction},{npcflag},1,1.14286,1,{classification},0,2000,2000,"
            f"1,1,{unit_class},0,0,0,0,0,7,0,'',0,1,0,0,0,0,1,0,{flags_extra},'{script}',NULL,66044)")


def tdb_difficulty(entry, health_modifier, gold_min, gold_max, loot_id, content_tuning=6, health_expansion=0,
                   delta=(0, 0), armor_modifier=1, damage_modifier=1, difficulty=0):
    """One TDB1210 `creature_template_difficulty` row (29 columns)."""
    return (f"({entry},{difficulty},{delta[0]},{delta[1]},{content_tuning},{health_expansion},{health_modifier},1,"
            f"{armor_modifier},{damage_modifier},0,0,0,0,{loot_id},0,0,"
            f"{gold_min},{gold_max},0,0,0,0,0,0,0,0,66044)")


def tdb_gameobject(guid, entry, map_id, x, y, z, phase_id=0):
    """One TDB1210 `gameobject` row (24 columns, TrinityCore DDL order)."""
    return (f"({guid},{entry},{map_id},0,0,'0',0,{phase_id},0,-1,{x},{y},{z},0.5,0,0,0.25,0.97,300,255,1,'',NULL,66044)")


def tdb_gameobject_template(entry, kind, display, name, data0=0, data1=0, data34=0):
    """One TDB1210 `gameobject_template` row (49 columns: Data0..Data34, ContentTuningId, RequiredLevel, …)."""
    data = [data0, data1] + [0] * 32 + [data34]
    return f"({entry},{kind},{display},'{name}','','','',1," + ",".join(map(str, data)) + ",0,0,'','',NULL,66044)"


def tdb_row(table, **values):
    """A positional TDB1210 `table` INSERT tuple; columns from tdb_schema.sql, unset ones 0 / ''."""
    schema = sqlite3.connect(":memory:")
    schema.executescript((SCRIPTS / "tdb_schema.sql").read_text())
    cells = []
    for _, name, sql_type, *_ in schema.execute(f"PRAGMA table_info(tdb_{table})"):
        value = values.pop(name, "" if sql_type == "TEXT" else 0)
        cells.append("NULL" if value is None else repr(value) if isinstance(value, str) else str(value))
    if values:
        raise KeyError(f"tdb_{table} has no columns {sorted(values)}")
    return "(" + ",".join(cells) + ")"


def write_tdb(tdb_dir, table, rows):
    (tdb_dir / f"{table}.sql").write_text(f"INSERT INTO `{table}` VALUES\n" + ",".join(rows) + ";\n")


def write_db2_rows(csv_dir, table, rows):
    """Rewrite <table>.csv with `rows` ({column: value}); other columns 0."""
    path = csv_dir / f"{table}.csv"
    header = path.read_text().splitlines()[0].split(",")
    with path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        for row in rows:
            writer.writerow(row.get(column, 0) for column in header)


def write_content_tuning(csv_dir, rows):
    """ContentTuning.csv rows (ID, MinLevelSquish, MaxLevelSquish, MinLevelScalingOffset, MaxLevelScalingOffset)."""
    path = csv_dir / "ContentTuning.csv"
    header = path.read_text().splitlines()[0].split(",")
    with path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        for tuning, low, high, low_type, high_type in rows:
            values = dict.fromkeys(header, 0) | {"ID": tuning, "MinLevelSquish": low, "MaxLevelSquish": high,
                                                "MinLevelScalingOffset": low_type, "MaxLevelScalingOffset": high_type}
            writer.writerow(values[column] for column in header)


def write_header_only_csvs(csv_dir):
    """Write one header-only CSV per DB2 table, headers taken from db2_schema.sql."""
    csv_dir.mkdir(parents=True)
    schema = sqlite3.connect(":memory:")
    schema.executescript(db2.SCHEMA_SQL.read_text())
    for name in db2.TABLES:
        columns = [row[1] for row in schema.execute(f"PRAGMA table_info({db2.table_name(name)})")]
        with (csv_dir / f"{name}.csv").open("w", newline="") as handle:
            csv.writer(handle).writerow(columns)


def write_creature_display_ids(csv_dir, ids):
    """Rewrite CreatureDisplayInfo.csv with one zeroed row per display id."""
    path = csv_dir / "CreatureDisplayInfo.csv"
    header = path.read_text().splitlines()[0].split(",")
    with path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows([display_id] + [0] * (len(header) - 1) for display_id in ids)


def dump_tables():
    """Table names in import-azerothcore.sh DUMP_TABLES."""
    script = (SCRIPTS / "import-azerothcore.sh").read_text()
    return script.split("DUMP_TABLES=(", 1)[1].split(")", 1)[0].split()


def run_db2_import(csv_root, db_path):
    return subprocess.run(
        [sys.executable, str(SCRIPTS / "db2.py"), "import", "--csv-root", str(csv_root), "--db", str(db_path)],
        capture_output=True,
        text=True,
        timeout=600,
        check=False,
    )


class CreatureEventImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        (self.root / "data").mkdir()
        for name in ("import-azerothcore.sh", "world_schema.sql", "db2.py", "db2_casc.py", "db2_schema.sql", "mysql_dump.py",
                     "content_common.py", "retail_content.py", "wowhead_loot.py",
                     "content_policy.py", "tdb_schema.sql"):
            shutil.copy2(SCRIPTS / name, scripts / name)
        (scripts / "content_overrides.csv").write_text("kind,source_id,retail_id,source_name,retail_name,reason\n")
        self.sql = self.root / "source"
        self.sql.mkdir()
        for name in dump_tables():
            (self.sql / f"{name}.sql").write_text("-- empty fixture\n")
        (self.sql / "creature.sql").write_text(
            "INSERT INTO `creature` (guid,id1,position_z) VALUES\n"
            "(96194,26012,84.2136),(79968,952,113.24),(76,1,30);\n"
        )
        (self.sql / "game_event_creature.sql").write_text(
            "INSERT INTO `game_event_creature` VALUES\n"
            "(31,96194),(-72,76),(31,76);\n"
        )
        self.csv = self.root / "csv"
        write_header_only_csvs(self.csv / db2.DEFAULT_BUILD)
        # Map.db2: Eastern Kingdoms and Kalimdor are open world (InstanceType 0), the Stockade a party dungeon.
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "Map", [
            {"ID": 0, "Directory": "Azeroth"}, {"ID": 1, "Directory": "Kalimdor"},
            {"ID": 34, "Directory": "StormwindJail", "InstanceType": 1}])
        (self.csv / content_policy.WRATH_BUILD).mkdir()
        (self.csv / content_policy.WRATH_BUILD / "SpellName.csv").write_text("ID,Name_lang\n")
        self.db = self.root / "data/world.db"
        self.tdb = self.root / "data/reference/tdb1210"
        self.tdb.mkdir(parents=True)
        for name in tdb_tables():
            (self.tdb / f"{name}.sql").write_text("-- empty fixture\n")

    def run_import(self):
        return subprocess.run(
            [str(self.root / "scripts/import-azerothcore.sh"), str(self.sql)],
            env={**os.environ, "DB2_CSV_ROOT": str(self.csv), "TDB_DIR": str(self.tdb)},
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def test_preserves_signed_memberships_and_spawn_coordinates(self):
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            memberships = conn.execute(
                "SELECT eventEntry,guid FROM game_event_creature ORDER BY guid,eventEntry"
            ).fetchall()
            self.assertEqual(memberships, [(-72, 76), (31, 76), (31, 96194)])
            self.assertEqual(conn.execute("SELECT count(*) FROM creature").fetchone()[0], 3)
            height = conn.execute(
                "SELECT position_z FROM creature WHERE guid=79968"
            ).fetchone()[0]
            self.assertEqual(height, 113.24)

    def test_missing_membership_source_does_not_destroy_existing_database(self):
        (self.sql / "game_event_creature.sql").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("game_event_creature.sql", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_missing_db2_csv_does_not_destroy_existing_database(self):
        (self.csv / db2.DEFAULT_BUILD / "SkillLineAbility.csv").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SkillLineAbility", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_missing_wrath_spell_names_does_not_destroy_existing_database(self):
        (self.csv / content_policy.WRATH_BUILD / "SpellName.csv").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SpellName.csv", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_tdb_trainer_rows_load_and_azerothcore_trainers_are_not_imported(self):
        (self.tdb / "trainer.sql").write_text("INSERT INTO `trainer` VALUES (163,2,'TDB greeting',0);\n")
        (self.tdb / "creature_trainer.sql").write_text("INSERT INTO `creature_trainer` VALUES (1346,163,4353,0);\n")
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            tables = {name for (name,) in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
            for dead in ("trainer", "trainer_spell", "npc_trainer", "creature_default_trainer", "content_trainer_spell"):
                self.assertNotIn(dead, tables)
            self.assertEqual(conn.execute("SELECT Id, Greeting FROM tdb_trainer").fetchall(), [(163, "TDB greeting")])
            self.assertEqual(conn.execute("SELECT * FROM tdb_creature_trainer").fetchall(), [(1346, 163, 4353, 0)])

    def test_missing_tdb_excerpt_does_not_destroy_existing_database(self):
        (self.tdb / "skill_tiers.sql").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("skill_tiers.sql", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_content_tables_join_quest_and_vendor_rows(self):
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name,subname,npcflag) VALUES\n"
            "(823,'Deputy Willem','',2),(54,'Corina Steele','Weaponsmith',4224),"
            "(3626,'Jenn Langston','Remy \\\"Two Times\\\"',1);\n"
        )
        (self.sql / "quest_template.sql").write_text(
            "INSERT INTO `quest_template` (ID,LogTitle,QuestDescription) VALUES\n"
            "(783,'A Threat Within','Line one.$B$BLine two\\\\'),(7,'Kobold Camp Cleanup',NULL);\n"
        )
        (self.sql / "creature_queststarter.sql").write_text("INSERT INTO `creature_queststarter` VALUES\n(823,783);\n")
        (self.sql / "item_template.sql").write_text(
            "INSERT INTO `item_template` (entry,name,BuyPrice,Quality) VALUES\n(2488,'Gladius',536,1),(2489,'Two-handed Sword',343,1);\n"
        )
        (self.sql / "npc_vendor.sql").write_text("INSERT INTO `npc_vendor` VALUES\n(54,0,2488,0,0,0,0),(54,0,2489,0,0,0,0);\n")
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute(
                    "SELECT q.ID, q.LogTitle, t.name FROM quest_template q"
                    " JOIN creature_queststarter s ON s.quest = q.ID JOIN creature_template t ON t.entry = s.id"
                ).fetchall(),
                [(783, "A Threat Within", "Deputy Willem")],
            )
            self.assertEqual(
                conn.execute("SELECT QuestDescription FROM quest_template ORDER BY ID").fetchall(),
                [(None,), ("Line one.$B$BLine two\\",)],
            )
            self.assertEqual(
                conn.execute("SELECT subname FROM creature_template WHERE entry = 3626").fetchone(),
                ('Remy "Two Times"',),
            )
            self.assertEqual(
                conn.execute(
                    "SELECT i.name, i.BuyPrice FROM npc_vendor v JOIN item_template i ON i.entry = v.item"
                    " WHERE v.entry = 54 ORDER BY v.item"
                ).fetchall(),
                [("Gladius", 536), ("Two-handed Sword", 343)],
            )

    def test_creature_addons_and_formations_keep_paths_mounts_auras_and_offsets(self):
        (self.sql / "creature_addon.sql").write_text(
            "INSERT INTO `creature_addon` VALUES\n"
            "(79968,799680,2410,0,1,0,0,'1244 7164'),(76,0,0,0,1,0,0,NULL);\n"
        )
        (self.sql / "creature_template_addon.sql").write_text(
            "INSERT INTO `creature_template_addon` VALUES\n(952,0,0,8,1,233,0,'');\n"
        )
        (self.sql / "creature_formations.sql").write_text(
            "INSERT INTO `creature_formations` VALUES\n(79968,79968,0,0,2,0,0),(79968,96194,3.5,90,515,0,0);\n"
        )
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT guid, path_id, mount, auras FROM creature_addon ORDER BY guid").fetchall(),
                [(76, 0, 0, None), (79968, 799680, 2410, "1244 7164")],
            )
            self.assertEqual(
                conn.execute("SELECT entry, bytes1, emote FROM creature_template_addon").fetchall(),
                [(952, 8, 233)],
            )
            self.assertEqual(
                conn.execute(
                    "SELECT leaderGUID, memberGUID, dist, angle, groupAI FROM creature_formations ORDER BY memberGUID"
                ).fetchall(),
                [(79968, 79968, 0.0, 0.0, 2), (79968, 96194, 3.5, 90.0, 515)],
            )

    def write_retail_spawn_fixture(self):
        """AzerothCore spawns on maps 0 and 571; TrinityCore Stormwind spawns in and out of the base phase."""
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name,minlevel,maxlevel,exp,faction,lootid) VALUES\n"
            "(68,'Stormwind City Guard',75,75,2,11,68),(2455,'Olivia Burnside',30,30,0,12,0),"
            "(26012,'Borean Tundra Wolf',70,71,2,14,26012);\n"
        )
        (self.sql / "creature.sql").write_text(
            "INSERT INTO `creature` (guid,id1,map,position_x,position_y,position_z) VALUES\n"
            "(79968,68,0,-8935.0,613.0,99.6),(120000,26012,571,3000.0,5000.0,20.0),(4000,2455,1,10.0,10.0,1.0);\n"
        )
        (self.sql / "creature_addon.sql").write_text(
            "INSERT INTO `creature_addon` VALUES\n(79968,799680,0,0,1,0,0,''),(120000,0,2410,0,1,0,0,'1244');\n"
        )
        (self.sql / "creature_formations.sql").write_text(
            "INSERT INTO `creature_formations` VALUES\n(120000,120000,0,0,2,0,0),(79968,79968,0,0,2,0,0);\n"
        )
        (self.sql / "game_event_creature.sql").write_text("INSERT INTO `game_event_creature` VALUES\n(12,79968),(12,120000);\n")
        rows = [
            tdb_creature(311, 68, 0, -8908.35, 639.24, 99.7),
            tdb_creature(312, 68, 0, -8900.0, 640.0, 99.7, phase_id=12345),
            tdb_creature(313, 68, 1, 10.0, 10.0, 1.0, difficulties="1,2"),
            tdb_creature(314, 68, 0, -8900.0, 641.0, 99.7, terrain_swap=638),
            tdb_creature(315, 42384, 0, -8910.0, 640.0, 99.7, difficulties="0,1"),
            tdb_creature(316, 68, 530, 1.0, 1.0, 1.0),
            tdb_creature(317, 68, 1, 20.0, 20.0, 1.0, phase_group=431),
        ]
        (self.tdb / "creature.sql").write_text("INSERT INTO `creature` VALUES\n" + ",".join(rows) + ";\n")
        (self.tdb / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` VALUES\n"
            + ",".join([tdb_template(68, "Stormwind City Guard", 11, 0),
                        tdb_template(42384, "Homeless Stormwind Citizen", 12, 1, flags_extra=2)]) + ";\n"
        )
        (self.tdb / "creature_template_model.sql").write_text(
            "INSERT INTO `creature_template_model` VALUES\n(42384,0,33167,1,1,66044),(42384,1,33168,1,1,66044),(68,0,3167,1,1,66044);\n"
        )
        (self.tdb / "creature_template_difficulty.sql").write_text(
            "INSERT INTO `creature_template_difficulty` VALUES\n" + tdb_difficulty(42384, 0.5, 3, 9, 42384) + ","
            + tdb_difficulty(68, 1.5, 0, 0, 0, content_tuning=864, health_expansion=-1, delta=(-1, 2),
                             armor_modifier=1.25, damage_modifier=1.75) + ";\n"
        )
        # 6: levels 5-30. 864: 1 to MaxLevel type 2, the Midnight cap 90.
        write_content_tuning(self.csv / db2.DEFAULT_BUILD, [(6, 5, 30, 0, 0), (864, 1, 0, 0, 2)])
        (self.tdb / "creature_addon.sql").write_text(
            "INSERT INTO `creature_addon` VALUES\n(311,3110,0,0,0,0,0,1,1,0,0,0,0,0,'130');\n"
        )
        (self.tdb / "creature_template_addon.sql").write_text(
            "INSERT INTO `creature_template_addon` VALUES\n(42384,0,0,0,1,0,0,1,0,0,0,0,0,0,'');\n"
        )
        (self.tdb / "waypoint_path.sql").write_text("INSERT INTO `waypoint_path` VALUES\n(3110,1,0,NULL,'guard');\n")
        (self.tdb / "waypoint_path_node.sql").write_text(
            "INSERT INTO `waypoint_path_node` VALUES\n(3110,0,-8908.0,639.0,99.7,NULL,0),(3110,1,-8890.0,650.0,99.7,1.2,5000);\n"
        )
        (self.tdb / "creature_formations.sql").write_text(
            "INSERT INTO `creature_formations` VALUES\n(311,311,0,0,515,0,0),(311,315,2,90,515,0,0);\n"
        )
        (self.tdb / "game_event_creature.sql").write_text("INSERT INTO `game_event_creature` VALUES\n(-5,315),(7,312);\n")

    def test_retail_spawns_replace_azerothcore_spawns_on_maps_0_and_1(self):
        self.write_retail_spawn_fixture()
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        offset = retail_content.RETAIL_GUID_OFFSET
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT guid, id1, map, position_x, position_y, spawntimesecs FROM content_creature"
                             " ORDER BY guid").fetchall(),
                [(120000, 26012, 571, 3000.0, 5000.0, 120),
                 (offset + 311, 68, 0, -8908.35, 639.24, 300),
                 (offset + 315, 42384, 0, -8910.0, 640.0, 300)],
            )
            self.assertEqual(conn.execute("SELECT count(*) FROM creature").fetchone()[0], 3, "raw spawns stay verbatim")
            self.assertEqual(
                conn.execute("SELECT entry, name, minlevel, maxlevel, exp, ContentTuningID, faction, npcflag, HealthModifier,"
                             " mingold, maxgold, flags_extra, detection_range FROM content_creature_template"
                             " WHERE entry = 42384").fetchall(),
                [(42384, "Homeless Stormwind Citizen", 5, 30, 0, 6, 12, 1, 0.5, 0, 0, 2, 20.0)],
                "TDB-only template: ContentTuning 6 range; no loot table anywhere, so no money",
            )
            self.assertEqual(
                conn.execute("SELECT CreatureID, Idx, CreatureDisplayID FROM content_creature_template_model"
                             " ORDER BY CreatureID, Idx").fetchall(),
                [(42384, 0, 33167), (42384, 1, 33168)],
                "TDB-only templates take TrinityCore's models; AzerothCore entries keep theirs",
            )
            self.assertEqual(
                conn.execute("SELECT entry, minlevel, maxlevel, exp, ContentTuningID, lootid FROM content_creature_template"
                             " WHERE entry IN (68, 2455) ORDER BY entry").fetchall(),
                [(68, 1, 90, 2, 864, 68), (2455, 30, 30, 0, 0, 0)],
                "AzerothCore templates keep their data; TrinityCore's ContentTuning range replaces fixed levels",
            )
            self.assertEqual(
                conn.execute("SELECT entry, HealthScalingExpansion, LevelScalingDeltaMin, LevelScalingDeltaMax,"
                             " HealthModifier, ArmorModifier, DamageModifier FROM content_creature_template"
                             " WHERE entry IN (68, 2455, 42384) ORDER BY entry").fetchall(),
                [(68, -1, -1, 2, 1.5, 1.25, 1.75), (2455, 0, 0, 0, 1.0, 1.0, 1.0), (42384, 0, 0, 0, 0.5, 1.0, 1.0)],
                "tuned templates take TrinityCore's difficulty-0 scaling expansion, level delta and modifiers",
            )
            self.assertEqual(
                conn.execute("SELECT ID, minlevel, maxlevel, MinLevelWithDelta, ScalingFactionGroup"
                             " FROM content_tuning_level WHERE ID IN (6, 864) ORDER BY ID").fetchall(),
                [(6, 5, 30, 5, 0), (864, 1, 90, 1, 0)],
                "GetContentTuningData levels are kept for the server",
            )
            self.assertEqual(
                conn.execute("SELECT guid, path_id, mount, bytes2, auras FROM content_creature_addon ORDER BY guid").fetchall(),
                [(120000, 0, 2410, 1, "1244"), (offset + 311, 3110, 0, 1 | 1 << 8, "130")],
            )
            self.assertEqual(
                conn.execute("SELECT entry FROM content_creature_template_addon").fetchall(), [(42384,)],
            )
            self.assertEqual(
                conn.execute("SELECT id, point, position_x, orientation, delay, move_type FROM content_waypoint_data"
                             " ORDER BY id, point").fetchall(),
                [(3110, 0, -8908.0, 0.0, 0, 1), (3110, 1, -8890.0, 1.2, 5000, 1)],
            )
            self.assertEqual(
                conn.execute("SELECT leaderGUID, memberGUID, dist, angle, groupAI FROM content_creature_formations"
                             " ORDER BY memberGUID").fetchall(),
                [(120000, 120000, 0.0, 0.0, 2), (offset + 311, offset + 311, 0.0, 0.0, 515),
                 (offset + 311, offset + 315, 2.0, 90.0, 515)],
            )
            self.assertEqual(
                conn.execute("SELECT eventEntry, guid FROM content_game_event_creature ORDER BY guid").fetchall(),
                [(12, 120000), (-5, offset + 315)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("creature", "guid=79968")], "map 0 spawn replaced by Retail spawns")
        self.assertEqual(reported[("content_creature_template", "entry=42384")],
                         "no loot table in either source; money 3-9 set to 0")
        self.assertEqual(reported[("creature", "guid=4000")], "map 1 spawn replaced by Retail spawns")
        self.assertEqual(reported[("creature_addon", "guid=79968")], "spawn replaced by Retail spawns")
        self.assertEqual(reported[("tdb_creature", "guid=312")], "not base phase: PhaseId 12345")
        self.assertEqual(reported[("tdb_creature", "guid=313")], "not base phase: spawnDifficulties 1,2")
        self.assertEqual(reported[("tdb_creature", "guid=314")], "not base phase: terrainSwapMap 638")
        self.assertEqual(reported[("tdb_creature", "guid=317")], "not base phase: PhaseGroup 431")
        self.assertNotIn(("tdb_creature", "guid=316"), reported, "other maps keep AzerothCore spawns silently")

    def test_retail_spawns_keep_their_modelid_and_unit_flags(self):
        """TrinityCore `creature.modelid` and `unit_flags` override the template per spawn
        (ObjectMgr::ChooseDisplayId, ChooseCreatureFlags); a modelid Retail CreatureDisplayInfo lacks is ignored."""
        self.write_retail_spawn_fixture()
        rows = (self.tdb / "creature.sql").read_text().rstrip(";\n") + "," + ",".join([
            tdb_creature(318, 68, 0, -8901.0, 642.0, 99.7, modelid=33168, unit_flags=33554432),
            tdb_creature(319, 68, 0, -8902.0, 643.0, 99.7, modelid=999999),
        ]) + ";\n"
        (self.tdb / "creature.sql").write_text(rows)
        write_creature_display_ids(self.csv / db2.DEFAULT_BUILD, [3167, 33167, 33168])
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        offset = retail_content.RETAIL_GUID_OFFSET
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT guid, modelid, unit_flags FROM content_creature WHERE guid > ? ORDER BY guid",
                             (offset,)).fetchall(),
                [(offset + 311, 0, 0), (offset + 315, 0, 0), (offset + 318, 33168, 33554432), (offset + 319, 0, 0)],
            )
            self.assertEqual(conn.execute("SELECT modelid FROM content_creature WHERE guid = 120000").fetchone(), (0,),
                             "AzerothCore spawns have no modelid")
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"]): (r["source_id"], r["reason"]) for r in csv.DictReader(handle)}
        self.assertEqual(reported[("tdb_creature", "guid=319")],
                         ("999999", "modelid absent from Retail CreatureDisplayInfo; the template's models apply"))

    def test_retail_stockade_spawns_replace_azerothcore_classic_stockade(self):
        """TrinityCore's DEFAULT_PHASE 169 spawns are base phase on every map; the Stockade (a party dungeon)
        takes its Normal-difficulty spawns and Wowhead's drops where TrinityCore has no loot."""
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name,minlevel,maxlevel,exp,faction,lootid,mingold,maxgold,rank) VALUES\n"
            "(1696,'Targorr the Dread',24,24,0,14,1696,123,266,1),(1720,'Bruegal Ironknuckle',25,25,0,14,1720,210,279,2);\n"
        )
        (self.sql / "creature.sql").write_text(
            "INSERT INTO `creature` (guid,id1,map,position_x,position_y,position_z) VALUES\n"
            "(84027,1696,34,159.582,1.25311,-25.6062),(79090,1720,34,160.051,45.709,-34.7729);\n"
        )
        (self.sql / "item_template.sql").write_text("INSERT INTO `item_template` (entry,name) VALUES\n(3228,'Jimmied Handcuffs');\n")
        write_tdb(self.tdb, "creature", [
            tdb_creature(375720, 46254, 34, 158.016, 116.564, -35.1063, difficulties="1", phase_id=169),
            tdb_creature(375808, 1720, 34, 103.112, 77.7348, -34.8562, difficulties="1", phase_id=169),
            tdb_creature(375671, 46248, 34, 145.88, 111.855, -35.1063, difficulties="2", phase_id=169),
            tdb_creature(16397, 16123, 0, -8900.0, 640.0, 99.7, phase_id=169),
        ])
        write_tdb(self.tdb, "creature_template", [tdb_template(46254, "Hogger", 14, 0, classification=1),
                                                  tdb_template(46248, "Riverpaw Basher", 14, 0, classification=1),
                                                  tdb_template(16123, "Gnome Citizen", 12, 0),
                                                  tdb_template(1720, "Bruegal Ironknuckle", 14, 0, classification=2)])
        (self.tdb / "creature_template_model.sql").write_text(
            "INSERT INTO `creature_template_model` VALUES\n(46254,0,108956,1,0,66044),(46248,0,175,1,0,66044),"
            "(16123,0,7197,1,1,66044);\n"
        )
        write_tdb(self.tdb, "creature_template_difficulty", [
            tdb_difficulty(46254, 35, 0, 0, 0, content_tuning=0),
            tdb_difficulty(46254, 35, 0, 0, 0, content_tuning=215, difficulty=1),
            tdb_difficulty(1720, 3, 210, 279, 1720, content_tuning=0),
            tdb_difficulty(16123, 1, 0, 0, 0, content_tuning=0),
        ])
        # Wowhead npc=46254 (Hogger) and npc=1720 pages, stockade-20260926 scrape (two rows each).
        scrape = self.root / "data/reference/wowhead/stockade-20260926"
        scrape.mkdir(parents=True)
        (scrape / "46254.json").write_text(json.dumps({"drops": [
            {"id": 2168, "quality": 3, "classs": 4, "stack": [1, 1],
             "modes": {"0": {"count": 41982, "outof": 135040}, "1": {"count": 41982, "outof": 135040}, "mode": [1]}},
            {"id": 3228, "quality": 2, "classs": 12, "stack": [1, 2],
             "modes": {"1": {"count": 50, "outof": 200}, "mode": [1]}}]}))
        (scrape / "1720.json").write_text(json.dumps({"drops": [
            {"id": 3228, "quality": 2, "classs": 4, "stack": [1, 1],
             "modes": {"1": {"count": 1, "outof": 2}, "mode": [1]}}]}))
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "ItemSparse", [{"ID": 3228, "Display_lang": "Jimmied Handcuffs"},
                                                                  {"ID": 2168, "Display_lang": "Corpse Rompers"}])
        write_tdb(self.tdb, "creature_loot_template", ["(1720,0,3228,60,0,1,1,1,1,'')"])
        write_content_tuning(self.csv / db2.DEFAULT_BUILD, [(215, 10, 30, 0, 0)])
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        offset = retail_content.RETAIL_GUID_OFFSET
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT guid, id1, map, position_x FROM content_creature ORDER BY guid").fetchall(),
                [(offset + 16397, 16123, 0, -8900.0), (offset + 375720, 46254, 34, 158.016),
                 (offset + 375808, 1720, 34, 103.112)],
                "Retail Hogger and Bruegal replace Classic Targorr, a phase-169 Stormwind spawn is kept, Heroic-only stays out",
            )
            self.assertEqual(
                conn.execute("SELECT entry, minlevel, maxlevel, ContentTuningID, HealthModifier FROM content_creature_template"
                             " WHERE entry = 46254").fetchall(),
                [(46254, 10, 30, 215, 35.0)],
                "Stockade creatures take their Normal (DifficultyID 1) row",
            )
            self.assertEqual(
                conn.execute("SELECT entry, lootid, mingold, maxgold FROM content_creature_template WHERE entry = 1720")
                .fetchall(),
                [(1720, retail_content.TDB_LOOT_OFFSET + 1720, 210, 279)],
                "without a Normal row, Difficulty.db2 falls Normal back to DifficultyID 0",
            )
            wowhead = retail_content.WOWHEAD_LOOT_OFFSET + 46254
            self.assertEqual(
                conn.execute("SELECT Entry, Item, round(Chance, 3), QuestRequired, GroupId, MinCount, MaxCount, Comment"
                             " FROM content_creature_loot_template ORDER BY Entry, Item").fetchall(),
                [(retail_content.TDB_LOOT_OFFSET + 1720, 3228, 60.0, 0, 1, 1, 1, ""),
                 (wowhead, 2168, 31.089, 0, 0, 1, 1, "wowhead stockade-20260926"),
                 (wowhead, 3228, 25.0, 1, 0, 1, 2, "wowhead stockade-20260926")],
                "Bruegal keeps TrinityCore's loot; Hogger (LootID 0) drops Wowhead's Normal-mode rates",
            )
            self.assertEqual(
                conn.execute("SELECT lootid, mingold, maxgold FROM content_creature_template WHERE entry = 46254").fetchall(),
                [(wowhead, 0, 0)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("creature", "guid=84027")], "map 34 spawn replaced by Retail spawns")
        self.assertEqual(reported[("tdb_creature", "guid=375671")], "not base phase: spawnDifficulties 2")
        self.assertNotIn(("tdb_creature", "guid=16397"), reported)
        self.assertEqual(reported[("wowhead_creature_drop", "entry=1720")],
                         "stockade-20260926: not Retail-spawned, TrinityCore loot kept, or no drops at its spawn difficulty")

    def write_retail_vendor_and_loot_fixture(self):
        """Corina Steele (54) spawned by TrinityCore sells Retail's list; an AzerothCore-only vendor keeps its own."""
        self.write_retail_spawn_fixture()
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name,minlevel,maxlevel,exp,faction,npcflag,lootid) VALUES\n"
            "(68,'Stormwind City Guard',75,75,2,11,0,68),(2455,'Olivia Burnside',30,30,0,12,0,0),"
            "(26012,'Borean Tundra Wolf',70,71,2,14,0,26012),(54,'Corina Steele',10,10,0,12,128,0),"
            "(999,'Arctic Vendor',70,70,2,35,128,0);\n"
        )
        (self.sql / "creature.sql").write_text(
            "INSERT INTO `creature` (guid,id1,map,position_x,position_y,position_z) VALUES\n"
            "(79968,68,0,-8935.0,613.0,99.6),(120000,26012,571,3000.0,5000.0,20.0),(4000,2455,1,10.0,10.0,1.0),"
            "(120001,999,571,3010.0,5000.0,20.0);\n"
        )
        (self.sql / "item_template.sql").write_text(
            "INSERT INTO `item_template` (entry,name) VALUES\n(2488,'Gladius'),(2489,'Two-handed Sword');\n"
        )
        (self.sql / "npc_vendor.sql").write_text("INSERT INTO `npc_vendor` VALUES\n(54,0,2488,0,0,0,0),(999,0,2488,0,0,0,0);\n")
        (self.sql / "creature_loot_template.sql").write_text(
            "INSERT INTO `creature_loot_template` VALUES\n(68,2488,0,5,0,1,0,1,1,'guard sword');\n"
        )
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "ItemSparse", [
            {"ID": 2488, "Display_lang": "Gladius"}, {"ID": 2489, "Display_lang": "Two-handed Sword"},
            {"ID": 200000, "Display_lang": "Retail Bread"}])
        rows = (self.tdb / "creature.sql").read_text().rstrip(";\n") + "," + tdb_creature(318, 54, 0, -9464.0, 99.0, 58.0)
        (self.tdb / "creature.sql").write_text(rows + ";\n")
        (self.tdb / "npc_vendor.sql").write_text(
            "INSERT INTO `npc_vendor` VALUES\n(54,1,2489,0,0,0,1,'',0,0,66044),(54,2,200000,0,0,0,1,'',0,0,66044),"
            "(54,3,1792,0,0,0,2,'',0,0,66044);\n"
        )
        (self.tdb / "creature_loot_template.sql").write_text(
            "INSERT INTO `creature_loot_template` VALUES\n(42384,0,2489,10,0,1,0,1,1,'sword'),"
            "(42384,1,500,100,0,1,0,1,1,'reference'),(42384,2,1792,100,0,1,0,1,5,'currency');\n"
        )
        (self.tdb / "reference_loot_template.sql").write_text(
            "INSERT INTO `reference_loot_template` VALUES\n(500,0,200000,50,0,1,0,1,2,'bread');\n"
        )

    def test_retail_vendors_and_loot_replace_azerothcore_for_retail_spawned_npcs(self):
        self.write_retail_vendor_and_loot_fixture()
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        loot = retail_content.TDB_LOOT_OFFSET
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT entry, slot, item FROM content_npc_vendor ORDER BY entry, slot").fetchall(),
                [(54, 1, 2489), (54, 2, 200000), (999, 0, 2488)],
                "Retail-spawned vendor sells TrinityCore's list; AzerothCore-only vendor keeps its own",
            )
            self.assertEqual(
                conn.execute("SELECT ID, Name, SourceEntry FROM content_item ORDER BY ID").fetchall(),
                [(2488, "Gladius", 2488), (2489, "Two-handed Sword", 2489), (200000, "Retail Bread", None)],
            )
            self.assertEqual(
                conn.execute("SELECT entry, lootid, mingold, maxgold FROM content_creature_template"
                             " WHERE entry IN (68, 42384) ORDER BY entry").fetchall(),
                [(68, 68, 0, 0), (42384, loot + 42384, 3, 9)],
                "TrinityCore loot (and its gold) when it has rows, AzerothCore's otherwise",
            )
            self.assertEqual(
                conn.execute("SELECT Entry, Item, Reference, Chance, MaxCount FROM content_creature_loot_template"
                             " ORDER BY Entry, Item").fetchall(),
                [(68, 2488, 0, 5.0, 1), (loot + 42384, 2489, 0, 10.0, 1), (loot + 42384, loot + 500, loot + 500, 100.0, 1)],
            )
            self.assertEqual(
                conn.execute("SELECT Entry, Item, Reference FROM content_reference_loot_template").fetchall(),
                [(loot + 500, 200000, 0)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"], r["source_id"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("tdb_npc_vendor", "entry=54", "1792")], "currency cost row; currencies are not modelled")
        self.assertEqual(reported[("tdb_creature_loot_template", f"Entry={loot + 42384}", "1792")],
                         "currency loot; currencies are not modelled")
        self.assertEqual(reported[("npc_vendor", "entry=54", "2488")], "vendor list replaced by TrinityCore's")

    def test_retail_gameobjects_replace_azerothcore_on_maps_0_and_1(self):
        (self.sql / "gameobject_template.sql").write_text(
            "INSERT INTO `gameobject_template` (entry,type,displayId,name) VALUES\n"
            "(144131,19,4127,'Mailbox'),(190000,19,4127,'Northrend Mailbox');\n"
        )
        (self.sql / "gameobject.sql").write_text(
            "INSERT INTO `gameobject` (guid,id,map,position_x,position_y,position_z) VALUES\n"
            "(26784,144131,0,-9456.0,46.0,56.6),(50000,190000,571,3000.0,5000.0,20.0);\n"
        )
        (self.tdb / "gameobject_template.sql").write_text(
            "INSERT INTO `gameobject_template` VALUES\n" + ",".join([
                tdb_gameobject_template(144131, 19, 9001, "Mailbox"),
                tdb_gameobject_template(3000, 3, 9001, "Battered Chest", data0=57, data1=77, data34=5),
                tdb_gameobject_template(3001, 3, 12345, "Invisible Chest")]) + ";\n"
        )
        (self.tdb / "gameobject_template_addon.sql").write_text(
            "INSERT INTO `gameobject_template_addon` VALUES\n(3000,94,4,0,0,0,0,0,0,0,0,0);\n"
        )
        (self.tdb / "gameobject.sql").write_text("INSERT INTO `gameobject` VALUES\n" + ",".join([
            tdb_gameobject(400, 144131, 0, -9460.0, 50.0, 57.0),
            tdb_gameobject(401, 144131, 0, -9461.0, 50.0, 57.0, phase_id=170),
            tdb_gameobject(402, 144131, 0, -9462.0, 50.0, 57.0),
            tdb_gameobject(403, 3000, 1, 10.0, 10.0, 1.0),
            tdb_gameobject(404, 3001, 1, 11.0, 10.0, 1.0)]) + ";\n")
        (self.tdb / "game_event_gameobject.sql").write_text("INSERT INTO `game_event_gameobject` VALUES\n(12,402);\n")
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "GameObjectDisplayInfo", [{"ID": 4127}, {"ID": 9001}])
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        offset = retail_content.RETAIL_GUID_OFFSET
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT guid, id, map, position_x, rotation2, spawntimesecs, state FROM content_gameobject"
                             " ORDER BY guid").fetchall(),
                [(50000, 190000, 571, 3000.0, 0.0, 0, 0), (offset + 400, 144131, 0, -9460.0, 0.25, 300, 1),
                 (offset + 403, 3000, 1, 10.0, 0.25, 300, 1)],
            )
            self.assertEqual(
                conn.execute("SELECT entry, type, displayId, Data0, Data1 FROM content_gameobject_template ORDER BY entry")
                .fetchall(),
                [(3000, 3, 9001, 57, retail_content.TDB_LOOT_OFFSET + 77), (144131, 19, 9001, 0, 0),
                 (190000, 19, 4127, 0, 0)],
                "Retail-spawned objects take TrinityCore's template; chest loot ids move beside AzerothCore's",
            )
            self.assertEqual(conn.execute("SELECT entry, faction, flags FROM content_gameobject_template_addon").fetchall(),
                             [(3000, 94, 4)])
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("gameobject", "guid=26784")], "map 0 spawn replaced by Retail spawns")
        self.assertEqual(reported[("tdb_gameobject", "guid=401")], "not base phase: PhaseId 170")
        self.assertEqual(reported[("tdb_gameobject", "guid=402")], "holiday spawn (positive game event only)")
        self.assertEqual(reported[("tdb_gameobject_template", "entry=3001")], "display not in Retail GameObjectDisplayInfo")
        self.assertEqual(reported[("tdb_gameobject", "guid=404")], "template dropped")
        self.assertEqual(reported[("tdb_gameobject_template", "entry=3000 Data34")], "Data24-34 not in the AzerothCore schema")

    def test_retail_gossip_menus_texts_and_options_for_retail_spawned_npcs(self):
        self.write_retail_spawn_fixture()
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name,gossip_menu_id) VALUES\n"
            "(68,'Stormwind City Guard',100),(2455,'Olivia Burnside',1291),(26012,'Borean Tundra Wolf',0);\n"
        )
        (self.sql / "gossip_menu.sql").write_text("INSERT INTO `gossip_menu` VALUES\n(100,4000),(1291,4001);\n")
        (self.sql / "gossip_menu_option.sql").write_text(
            "INSERT INTO `gossip_menu_option` VALUES\n(100,0,0,'Wrath option',0,1,1,0,0,0,0,'',0,0);\n"
        )
        (self.sql / "npc_text.sql").write_text(
            "INSERT INTO `npc_text` (ID,text0_0,BroadcastTextID0,Probability0) VALUES\n(4000,'Wrath guard text',0,1),"
            "(4001,'Olivia text',0,1),(5000,'Old text 5000',0,1);\n"
        )
        (self.tdb / "creature_template_gossip.sql").write_text(
            "INSERT INTO `creature_template_gossip` VALUES\n(42384,9000,66044),(68,9002,66044),(68,9001,66044);\n"
        )
        (self.tdb / "gossip_menu.sql").write_text(
            "INSERT INTO `gossip_menu` VALUES\n(9000,5000,66044),(9001,5000,66044),(9003,5001,66044),(9999,5002,66044);\n"
        )
        (self.tdb / "gossip_menu_option.sql").write_text(
            "INSERT INTO `gossip_menu_option` VALUES\n"
            "(9000,100,0,1,'I want to browse your goods.',3370,0,0,0,0,NULL,0,0,'',0,NULL,NULL,66044),"
            "(9000,101,1,0,'Tell me more.',0,0,0,9003,0,NULL,0,0,'',0,NULL,NULL,66044),"
            "(9000,102,2,10,'Auction!',0,0,0,0,0,NULL,0,0,'',0,NULL,NULL,66044),"
            "(9000,103,3,18,'Check my mail.',0,0,0,0,0,NULL,0,0,'',0,NULL,NULL,66044);\n"
        )
        (self.tdb / "npc_text.sql").write_text(
            "INSERT INTO `npc_text` VALUES\n(5000,1,0,0,0,0,0,0,0,3000,0,0,0,0,0,0,0,66044),"
            "(5001,0.5,0.5,0,0,0,0,0,0,3001,3002,0,0,0,0,0,0,66044);\n"
        )
        (self.sql / "broadcast_text.sql").write_text(
            "INSERT INTO `broadcast_text` (ID,MaleText,FemaleText) VALUES\n(3001,'Wrath text',''),"
            "(3370,'I want to browse your goods.','');\n"
        )
        write_tdb(self.tdb, "broadcast_text", [
            tdb_row("broadcast_text", Text="Hello, $n.", ID=3000, VerifiedBuild=66044),
            tdb_row("broadcast_text", Text="Old Retail text", ID=3001, VerifiedBuild=60000),
            tdb_row("broadcast_text", Text="", Text1="Retail text", ID=3001, VerifiedBuild=66044)])
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT entry, gossip_menu_id FROM content_creature_template ORDER BY entry").fetchall(),
                [(68, 9001), (2455, 1291), (26012, 0), (42384, 9000)],
                "Retail-spawned NPCs take TrinityCore's lowest menu; others keep AzerothCore's",
            )
            self.assertEqual(
                conn.execute("SELECT MenuID, TextID FROM content_gossip_menu ORDER BY MenuID").fetchall(),
                [(100, 4000), (1291, 4001), (9000, 5000), (9001, 5000), (9003, 5001)],
                "TrinityCore rows for the menus Retail-spawned NPCs reach; AzerothCore rows for the rest",
            )
            self.assertEqual(
                conn.execute("SELECT MenuID, OptionID, OptionIcon, OptionText, OptionBroadcastTextID, OptionType,"
                             " OptionNpcFlag, ActionMenuID FROM content_gossip_menu_option ORDER BY MenuID, OptionID")
                .fetchall(),
                [(100, 0, 0, "Wrath option", 0, 1, 1, 0),
                 (9000, 0, 1, "I want to browse your goods.", 3370, 3, 128, 0),
                 (9000, 1, 0, "Tell me more.", 0, 1, 1, 9003),
                 (9000, 2, 6, "Auction!", 0, 13, 2097152, 0),
                 (9000, 3, 0, "Check my mail.", 0, 0, 1, 0)],
                "GossipOptionNpc → AzerothCore option type, npcflag and icon",
            )
            self.assertEqual(
                conn.execute("SELECT ID, text0_0, BroadcastTextID0, Probability0, BroadcastTextID1, Probability1"
                             " FROM content_npc_text ORDER BY ID").fetchall(),
                [(4000, "Wrath guard text", 0, 1.0, 0, 0.0), (4001, "Olivia text", 0, 1.0, 0, 0.0),
                 (5000, None, 3000, 1.0, 0, 0.0),
                 (5001, None, 3001, 0.5, 3002, 0.5)],
            )
            self.assertEqual(
                conn.execute("SELECT ID, MaleText, FemaleText FROM content_broadcast_text ORDER BY ID").fetchall(),
                [(3000, "Hello, $n.", ""), (3001, "", "Retail text"), (3370, "I want to browse your goods.", "")],
                "TrinityCore's latest-build BroadcastText row wins; AzerothCore's rows fill the rest",
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"], r["source_id"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("tdb_creature_template_gossip", "CreatureID=68", "9002")],
                         "more than one menu; the server has no conditions, lowest MenuID 9001 used")
        self.assertEqual(reported[("tdb_gossip_menu_option", "MenuID=9000 OptionID=3", "18")],
                         "GossipOptionNpc 18 has no AzerothCore option type; hidden")
        self.assertEqual(reported[("tdb_npc_text", "ID=5001", "3002")],
                         "BroadcastText id in neither TrinityCore's nor AzerothCore's broadcast_text")

    def test_retail_quests_objectives_rewards_and_relations(self):
        self.write_retail_vendor_and_loot_fixture()
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name) VALUES\n(68,'Stormwind City Guard'),(823,'Deputy Willem'),"
            "(26012,'Borean Tundra Wolf'),(6,'Kobold Vermin'),(54,'Corina Steele'),(999,'Arctic Vendor');\n"
        )
        (self.sql / "quest_template.sql").write_text(
            "INSERT INTO `quest_template` (ID,QuestLevel,MinLevel,RewardMoney,LogTitle,RewardChoiceItemID1,"
            "RewardChoiceItemQuantity1) VALUES\n"
            "(783,2,1,100,'A Threat Within',2488,1),(999,70,68,5000,'Northrend Quest',0,0);\n"
        )
        (self.sql / "quest_template_addon.sql").write_text("INSERT INTO `quest_template_addon` (ID,PrevQuestID) VALUES\n(783,0);\n")
        (self.sql / "creature_queststarter.sql").write_text("INSERT INTO `creature_queststarter` VALUES\n(823,783),(26012,999);\n")
        (self.sql / "creature_questender.sql").write_text("INSERT INTO `creature_questender` VALUES\n(823,783);\n")
        rows = (self.tdb / "creature.sql").read_text().rstrip(";\n") + "," + tdb_creature(319, 823, 0, -8933.0, -136.0, 83.0)
        (self.tdb / "creature.sql").write_text(rows + ";\n")
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "QuestV2", [{"ID": 783}, {"ID": 999}, {"ID": 26150}])
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "SpellName", [{"ID": 8326, "Name_lang": "Ghost"}])
        # ContentTuning 6 tops out at 30: QuestMoneyReward level 30, difficulty 5.
        write_db2_rows(self.csv / db2.DEFAULT_BUILD, "QuestMoneyReward", [{"ID": 30, "Difficulty_5": 1235}])
        write_tdb(self.tdb, "quest_template", [
            tdb_row("quest_template", ID=783, QuestType=2, ContentTuningID=6, RewardXPDifficulty=1, Flags=8,
                    AllowableRaces=-1, LogTitle="A Threat Within", LogDescription="Speak with Marshal McBride.",
                    QuestDescription="Kobolds!", QuestCompletionLog="Return."),
            tdb_row("quest_template", ID=26150, QuestType=2, ContentTuningID=6, RewardMoneyDifficulty=5,
                    RewardMoneyMultiplier=0.5, RewardItem1=200000, RewardAmount1=1,
                    RewardChoiceItemID1=999999, RewardChoiceItemQuantity1=1, RewardSpell=8326,
                    RewardDisplaySpell1=77777, LogTitle="Retail Only"),
            tdb_row("quest_template", ID=31000, LogTitle="Not In QuestV2")])
        write_tdb(self.tdb, "quest_objectives", [
            tdb_row("quest_objectives", ID=1, QuestID=783, Type=0, Order=0, ObjectID=6, Amount=8,
                    Description="Kobold Vermin slain"),
            tdb_row("quest_objectives", ID=2, QuestID=783, Type=1, Order=1, ObjectID=2489, Amount=2),
            tdb_row("quest_objectives", ID=3, QuestID=783, Type=10, Order=2, ObjectID=5000, Amount=1)])
        write_tdb(self.tdb, "quest_offer_reward", [tdb_row("quest_offer_reward", ID=783, RewardText="Well done.")])
        write_tdb(self.tdb, "creature_questender", [tdb_row("creature_questender", id=68, quest=783)])
        write_tdb(self.tdb, "creature_queststarter", [tdb_row("creature_queststarter", id=42384, quest=26150),
                                                      tdb_row("creature_queststarter", id=42384, quest=31000)])
        write_tdb(self.tdb, "quest_poi", [tdb_row("quest_poi", QuestID=26150, BlobIndex=0, Idx1=0, ObjectiveIndex=-1,
                                                  MapID=0, UiMapID=37, Priority=0, Flags=3)])
        write_tdb(self.tdb, "quest_poi_points", [tdb_row("quest_poi_points", QuestID=26150, Idx1=0, Idx2=0, X=-9000, Y=-80, Z=90)])
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT ID, QuestLevel, MinLevel, ContentTuningID, RewardMoney, RewardXPDifficulty, Flags,"
                             " AllowableRaces, LogTitle, LogDescription, QuestCompletionLog FROM content_quest_template"
                             " ORDER BY ID").fetchall(),
                [(783, -1, 5, 6, 0, 1, 8, -1, "A Threat Within", "Speak with Marshal McBride.", "Return."),
                 (999, 70, 68, 0, 5000, 0, 0, 0, "Northrend Quest", None, None),
                 (26150, -1, 5, 6, 617, 0, 0, 0, "Retail Only", "", "")],
                "TrinityCore quests scale to the player (-1) from ContentTuning's minimum; money is"
                " QuestMoneyReward[ContentTuning max 30][5] 1235 × 0.5, not AzerothCore's",
            )
            self.assertEqual(
                conn.execute("SELECT RequiredNpcOrGo1, RequiredNpcOrGoCount1, ObjectiveText1, RequiredItemId1,"
                             " RequiredItemCount1, RequiredNpcOrGo2 FROM content_quest_template WHERE ID = 783").fetchall(),
                [(6, 8, "Kobold Vermin slain", 2489, 2, 0)],
            )
            self.assertEqual(
                conn.execute("SELECT RewardItem1, RewardAmount1, RewardChoiceItemID1, RewardSpell, RewardDisplaySpell"
                             " FROM content_quest_template WHERE ID = 26150").fetchall(),
                [(200000, 1, 0, 8326, 0)],
            )
            self.assertEqual(conn.execute("SELECT id, quest FROM content_creature_queststarter ORDER BY quest").fetchall(),
                             [(26012, 999), (42384, 26150)],
                             "Retail-spawned NPCs give TrinityCore's quests only; AzerothCore's Northrend giver keeps its own")
            self.assertEqual(conn.execute("SELECT RewardChoiceItemID1, RewardChoiceItemQuantity1 FROM content_quest_template"
                                          " WHERE ID = 783").fetchall(), [(2488, 1)],
                             "AzerothCore's choice rewards fill TrinityCore's empty ones")
            self.assertEqual(conn.execute("SELECT ID, RewardMoneyDifficulty, RewardMoneyMultiplier FROM content_quest_template"
                                          " ORDER BY ID").fetchall(), [(783, 0, 0.0), (999, 0, 1.0), (26150, 5, 0.5)])
            self.assertEqual(conn.execute("SELECT id, quest FROM content_creature_questender ORDER BY quest").fetchall(),
                             [(68, 783)])
            self.assertEqual(conn.execute("SELECT ID, RewardText FROM content_quest_offer_reward").fetchall(),
                             [(783, "Well done.")])
            self.assertEqual(conn.execute("SELECT ID, PrevQuestID FROM content_quest_template_addon").fetchall(),
                             [(783, 0)], "AzerothCore addon kept where TrinityCore has none")
            self.assertEqual(conn.execute("SELECT QuestID, id, ObjectiveIndex, MapID, WorldMapAreaId, Flags"
                                          " FROM content_quest_poi").fetchall(), [(26150, 0, -1, 0, 37, 3)])
            self.assertEqual(conn.execute("SELECT QuestID, Idx1, Idx2, X, Y FROM content_quest_poi_points").fetchall(),
                             [(26150, 0, 0, -9000, -80)])
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], r["key"], r["source_id"]): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("tdb_quest_objectives", "QuestID=783 ID=3", "5000")],
                         "objective type 10 not supported")
        self.assertEqual(reported[("tdb_quest_template", "ID=26150 RewardChoiceItemID1", "999999")],
                         "item not in Retail ItemSparse; zeroed")
        self.assertEqual(reported[("tdb_quest_template", "ID=26150 RewardDisplaySpell1", "77777")],
                         "spell not in Retail SpellName; zeroed")
        self.assertEqual(reported[("tdb_quest_template", "ID=31000", "31000")], "quest id not in Retail QuestV2")
        self.assertEqual(reported[("quest_template", "ID=783", "783")], "replaced by TrinityCore's quest")
        self.assertEqual(reported[("content_quest_template", "ID=783 RewardChoiceItemID", "783")],
                         "empty TrinityCore reward group filled from the same-ID AzerothCore quest")
        self.assertEqual(reported[("creature_queststarter", "id=823", "783")],
                         "relation of a Retail-spawned entry replaced by TrinityCore's")

    def write_wrath_only_display_fixture(self):
        """AzerothCore displays 6808/382/79/3167 against Retail displays 520/366/3167 and TrinityCore models."""
        (self.sql / "creature_template.sql").write_text(
            "INSERT INTO `creature_template` (entry,name) VALUES\n"
            "(2349,'Giant Moss Creeper'),(30,'Forest Spider'),(54,'Corina Steele'),"
            "(23050,'Little Noah'),(1216,'Shore Crawler'),(1822,'Venom Mist Lurker');\n"
        )
        (self.sql / "creature_template_model.sql").write_text(
            "INSERT INTO `creature_template_model` VALUES\n"
            "(2349,0,6808,1,1,12340),(30,0,382,1,1,12340),(54,0,3167,1,1,12340),"
            "(23050,0,79,1,1,12340),(1216,0,9570,1,1,12340),(1822,0,1087,1,1,12340);\n"
        )
        tdb_template = "INSERT INTO `creature_template` VALUES\n" + ",".join(
            f"({entry},0,0,'{name}',NULL,NULL,NULL,NULL" + ",0" * 34 + ")"
            for entry, name in ((2349, "Domesticated Creeper"), (30, "Forest Spider"), (54, "Corina Steele"),
                                (1216, "Shore Crawler"), (1822, "Venom Mist Lurker"))
        ) + ";\n"
        (self.tdb / "creature_template.sql").write_text(tdb_template)
        (self.tdb / "creature_template_model.sql").write_text(
            "INSERT INTO `creature_template_model` VALUES\n"
            "(2349,0,520,1,1,66044),(30,0,366,1,1,66044),(54,0,99999,1,1,66044),"
            "(1216,0,32790,1,1,66044),(1822,0,759,1,1,66044);\n"
        )
        write_creature_display_ids(self.csv / db2.DEFAULT_BUILD, [520, 366, 3167, 759])

    def test_wrath_only_displays_take_trinitycore_retail_models(self):
        self.write_wrath_only_display_fixture()
        (self.root / "scripts/content_overrides.csv").write_text(
            "kind,source_id,retail_id,source_name,retail_name,reason\n"
            "creature,2349,2349,Giant Moss Creeper,Domesticated Creeper,renamed spider\n"
        )
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT CreatureID, Idx, CreatureDisplayID, VerifiedBuild FROM content_creature_template_model"
                             " ORDER BY CreatureID").fetchall(),
                [(30, 0, 366, 66044), (54, 0, 3167, 12340), (1216, 0, 9570, 12340), (1822, 0, 759, 66044),
                 (2349, 0, 520, 66044), (23050, 0, 79, 12340)],
            )
            self.assertEqual(
                conn.execute("SELECT CreatureID, CreatureDisplayID FROM creature_template_model ORDER BY CreatureID").fetchall(),
                [(30, 382), (54, 3167), (1216, 9570), (1822, 1087), (2349, 6808), (23050, 79)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reported = {(r["table"], int(r["source_id"])): r["reason"] for r in csv.DictReader(handle)}
        self.assertEqual(reported[("creature_template_model", 23050)],
                         "display absent from Retail kept: no TrinityCore Retail template")
        self.assertEqual(reported[("creature_template_model", 1216)],
                         "display absent from Retail kept: TrinityCore Retail displays [32790] absent from Retail CreatureDisplayInfo")
        self.assertNotIn(("creature_template_model", 2349), reported)

    def test_renamed_creature_without_reviewed_override_keeps_wrath_display_and_is_reported(self):
        self.write_wrath_only_display_fixture()
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID = 2349").fetchall(),
                [(6808,)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reasons = [r["reason"] for r in csv.DictReader(handle) if r["source_id"] == "2349"]
        self.assertEqual(reasons, ["display absent from Retail kept: TrinityCore Retail name 'Domesticated Creeper' != 'Giant Moss Creeper'"])

    def test_retail_spawned_creature_takes_its_retail_models_despite_a_rename(self):
        # Pygmy Surf Crawler 3106's case: TrinityCore renamed the entry (Surf Crawler), but a
        # Retail spawn references the entry, so it is TrinityCore's creature and wears its models.
        self.write_wrath_only_display_fixture()
        (self.tdb / "creature.sql").write_text(
            "INSERT INTO `creature` VALUES\n" + tdb_creature(401, 2349, 1, 10.0, 10.0, 1.0) + ";\n")
        result = self.run_import()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(
                conn.execute("SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID = 2349").fetchall(),
                [(520,)],
            )
        with (self.root / "data/reports/content-dropped.csv").open(newline="") as handle:
            reasons = [r["reason"] for r in csv.DictReader(handle)
                       if r["table"] == "creature_template_model" and r["source_id"] == "2349"]
        self.assertEqual(reasons, [])

    def test_missing_trinitycore_models_do_not_destroy_existing_database(self):
        (self.tdb / "creature_template_model.sql").unlink()
        self.db.write_bytes(b"existing database must survive failed preflight")
        before = self.db.read_bytes()
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("creature_template_model.sql", result.stderr)
        self.assertEqual(self.db.read_bytes(), before)

    def test_unparseable_dump_fails_without_partial_tables(self):
        (self.sql / "npc_vendor.sql").write_text("INSERT INTO `npc_vendor` VALUES\n(54,0,2488,0,0,0,0),(54,0,0x1F,0,0,0,0);\n")
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unparseable value", result.stderr)


@unittest.skipUnless(AZEROTHCORE_SQL.is_dir(), f"AzerothCore dumps not present at {AZEROTHCORE_SQL}")
@unittest.skipUnless(PINNED_BUILD_DIR.is_dir(), f"pinned DB2 CSVs not present at {PINNED_BUILD_DIR}")
@unittest.skipUnless(TDB_DIR.is_dir(), f"TrinityCore Retail excerpts not present at {TDB_DIR}")
class AzerothCoreContentTests(unittest.TestCase):
    """Load real AzerothCore dumps + pinned Retail DB2, derive content_* tables, check Elwynn Forest rows."""

    TABLES = (
        "creature_template", "creature_template_model", "quest_template", "creature_queststarter", "creature_questender",
        "item_template", "npc_vendor",
        "gameobject_template", "gameobject_template_addon", "gameobject",
        "creature", "creature_addon", "creature_template_addon", "creature_formations", "game_event_creature", "waypoint_data",
    )

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.temp.name) / "world.db"
        cls.report = Path(cls.temp.name) / "content-dropped.csv"
        commands = [
            [sys.executable, str(SCRIPTS / "mysql_dump.py"), "--db", str(cls.db), "--schema", str(SCRIPTS / "world_schema.sql")]
            + [f"{name}={AZEROTHCORE_SQL / (name + '.sql')}" for name in cls.TABLES],
            [sys.executable, str(SCRIPTS / "mysql_dump.py"), "--db", str(cls.db), "--schema", str(SCRIPTS / "tdb_schema.sql"),
             "--prefix", "tdb_"] + [f"{name}={TDB_DIR / (name + '.sql')}" for name in tdb_tables()],
            [sys.executable, str(SCRIPTS / "db2.py"), "import", "--csv-root", str(PINNED_CSV_ROOT), "--db", str(cls.db)],
            [sys.executable, str(SCRIPTS / "content_policy.py"), "--db", str(cls.db), "--report", str(cls.report),
             "--csv-root", str(PINNED_CSV_ROOT)],
        ]
        for command in commands:
            result = subprocess.run(command, capture_output=True, text=True, timeout=900, check=False)
            if result.returncode != 0:
                raise AssertionError(result.stdout + result.stderr)
        cls.conn = sqlite3.connect(cls.db)
        with cls.report.open(newline="", encoding="utf-8") as handle:
            cls.dropped = {(r["table"], r["key"], int(r["source_id"])): r["reason"] for r in csv.DictReader(handle)}

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.temp.cleanup()

    def rows(self, sql, *params):
        return self.conn.execute(sql, params).fetchall()

    def vendor_items(self, entry):
        return self.rows(
            "SELECT v.item, c.Name, c.BuyPrice, c.ItemLevel, c.DisplayInfoID FROM content_npc_vendor v"
            " JOIN content_item c ON c.ID = v.item WHERE v.entry = ? ORDER BY v.item", entry)

    def test_corina_steele_sells_items_with_retail_level_and_display(self):
        self.assertEqual(
            self.vendor_items(54),
            [
                (2488, "Gladius", 536, 4, 1542), (2489, "Two-Handed Sword", 343, 3, 1627),
                (2490, "Tomahawk", 540, 4, 453), (2491, "Large Axe", 484, 4, 22112),
                (2492, "Cudgel", 285, 3, 12992), (2493, "Wooden Mallet", 701, 4, 22121),
                (2494, "Stiletto", 402, 4, 22136), (2495, "Walking Stick", 505, 4, 1768),
            ],
        )
        # AzerothCore: Gladius item level 9, display 22078.
        self.assertEqual(self.rows("SELECT ItemLevel, displayid FROM item_template WHERE entry = 2488"), [(9, 22078)])

    def test_goldshire_barkeep_water_uses_retail_price(self):
        self.assertEqual(self.rows("SELECT BuyPrice FROM item_template WHERE entry = 159"), [(25,)])
        self.assertIn((159, "Refreshing Spring Water", 5), [row[:3] for row in self.vendor_items(465)])

    def test_wrath_only_items_are_dropped_and_reported(self):
        self.assertEqual(self.rows("SELECT count(*) FROM content_item WHERE ID = 41"), [(0,)])
        # Retail's 2892 Empty Poison Vial is in content_item for TrinityCore's content, not as
        # AzerothCore's Deadly Poison (no SourceEntry).
        self.assertEqual(self.rows("SELECT Name, SourceEntry FROM content_item WHERE ID = 2892"),
                         [("Empty Poison Vial", None)])
        self.assertEqual(self.dropped[("item_template", "entry=41", 41)], "no Retail row")
        self.assertEqual(
            self.dropped[("item_template", "entry=2892", 2892)], "Retail name 'Empty Poison Vial' != 'Deadly Poison'")
        self.assertEqual(self.rows("SELECT count(*) FROM content_npc_vendor WHERE entry = 1325 AND item = 2892"), [(0,)])
        self.assertEqual(self.dropped[("npc_vendor", "entry=1325", 2892)], "item not mapped")

    def test_reviewed_override_keeps_renamed_okra(self):
        self.assertEqual(self.rows("SELECT Name, SourceEntry FROM content_item WHERE ID = 732"), [("Ripe Okra", 732)])

    def test_quests_are_kept_only_when_retail_has_the_id(self):
        # 783 A Threat Within is in QuestV2 but Cataclysm removed it: Retail Deputy Willem
        # gives TrinityCore's quests only, so nothing starts it.
        self.assertEqual(
            self.rows(
                "SELECT q.ID, t.name FROM content_quest_template q JOIN content_creature_queststarter s ON s.quest = q.ID"
                " JOIN content_creature_template t ON t.entry = s.id WHERE q.ID IN (16, 783, 28789)"
            ),
            [(28789, "Marshal McBride")],
        )
        self.assertEqual(self.dropped[("quest_template", "ID=16", 16)], "quest id not in Retail QuestV2")

    def test_a_threat_within_starts_at_deputy_willem_and_ends_at_marshal_mcbride(self):
        self.assertEqual(
            self.rows(
                "SELECT q.ID, q.QuestLevel, st.name, en.name FROM quest_template q"
                " JOIN creature_queststarter s ON s.quest = q.ID JOIN creature_template st ON st.entry = s.id"
                " JOIN creature_questender e ON e.quest = q.ID JOIN creature_template en ON en.entry = e.id"
                " WHERE q.LogTitle = 'A Threat Within'"
            ),
            [(783, 1, "Deputy Willem", "Marshal McBride")],
        )

    def test_goldshire_weaponsmith_corina_steele_sells_basic_weapons(self):
        self.assertEqual(
            self.rows(
                "SELECT v.item, i.name, i.BuyPrice FROM npc_vendor v JOIN item_template i ON i.entry = v.item"
                " WHERE v.entry = 54 ORDER BY v.item"
            ),
            [
                (2488, "Gladius", 536), (2489, "Two-handed Sword", 343), (2490, "Tomahawk", 540),
                (2491, "Large Axe", 484), (2492, "Cudgel", 285), (2493, "Wooden Mallet", 701),
                (2494, "Stiletto", 402), (2495, "Walking Stick", 505),
            ],
        )

    def test_stormwind_guild_vaults_keep_retail_display_and_spawns(self):
        # TrinityCore entry 205107 "Guild Vault" (type 34 GAMEOBJECT_TYPE_GUILD_BANK), display 7607:
        # Retail GameObjectDisplayInfo 7607 is FileDataID 199769 (guildvault_human_01.m2).
        self.assertEqual(
            self.rows("SELECT t.name, t.type, d.FileDataID FROM content_gameobject_template t"
                      " JOIN game_object_display_info d ON d.ID = t.displayId WHERE t.entry = 205107"),
            [("Guild Vault", 34, 199769)],
        )
        # Retail Stormwind has one entry per vault; AzerothCore's four 187329 spawns are replaced.
        self.assertEqual(
            self.rows("SELECT id, round(position_x, 2), round(position_y, 2) FROM content_gameobject"
                      " WHERE id BETWEEN 205104 AND 205107 AND map = 0 ORDER BY guid"),
            [(205107, -8930.07, 613.16), (205106, -8932.42, 617.34), (205105, -8303.91, 572.98),
             (205104, -8307.61, 569.94)],
        )
        self.assertEqual(self.rows("SELECT count(*) FROM content_gameobject WHERE id = 187329 AND map = 0"), [(0,)])

    def test_retail_bank_tab_prices(self):
        # Retail BankTab: Character 1g..5000g, Guild 100g..5000g, Warband 1000g..2500000g.
        costs = {bank: [c for (c,) in self.rows("SELECT Cost FROM bank_tab WHERE BankType = ? ORDER BY OrderIndex", bank)]
                 for bank in (0, 1, 2)}
        self.assertEqual(costs[0], [10_000, 1_000_000, 5_000_000, 10_000_000, 25_000_000, 50_000_000])
        self.assertEqual(costs[1], [1_000_000, 2_500_000, 5_000_000, 10_000_000, 25_000_000, 50_000_000])
        self.assertEqual(costs[2], [10_000_000, 250_000_000, 1_000_000_000, 5_000_000_000, 25_000_000_000])

    def test_wrath_only_spider_and_murloc_displays_use_retail_models(self):
        # Giant Moss Creeper 6808, Forest Lurker 827, Daggerspine Raider 11254 and Silt Crawler 9571 are not in Retail.
        self.assertEqual(
            self.rows("SELECT CreatureID, CreatureDisplayID FROM content_creature_template_model"
                      " WHERE CreatureID IN (2349, 1195, 2595, 922) ORDER BY CreatureID"),
            [(922, 342), (1195, 909), (2349, 520), (2595, 4763)],
        )

    def test_pygmy_surf_crawler_model_is_a_retail_display(self):
        # AzerothCore 3106's display 1307 is not in Retail; its Retail spawns on map 1 take
        # TrinityCore's model although TrinityCore renamed the entry Surf Crawler.
        self.assertEqual(self.rows("SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID = 3106"),
                         [(1938,)])
        self.assertEqual(self.rows("SELECT count(*) FROM creature_display_info WHERE ID = 1938"), [(1,)])

    def test_every_display_spawned_on_maps_0_and_1_is_a_retail_display(self):
        self.assertEqual(self.rows("""SELECT DISTINCT m.CreatureID, m.CreatureDisplayID FROM content_creature c
            JOIN content_creature_template_model m ON m.CreatureID = c.id1
            WHERE c.map IN (0, 1) AND m.CreatureDisplayID NOT IN (SELECT ID FROM creature_display_info)"""), [])

    def test_retail_display_is_kept_as_imported(self):
        # Hogger's AzerothCore display 384 exists in Retail, so TrinityCore's 108956 is not used.
        self.assertEqual(self.rows("SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID = 448"), [(384,)])

    def test_quoted_creature_subname_is_unescaped(self):
        self.assertEqual(self.rows("SELECT count(*) FROM creature_template WHERE name LIKE '%\\\"%'"), [(0,)])
        self.assertEqual(self.rows("SELECT name FROM creature_template WHERE name LIKE 'Remy%'"), [('Remy "Two Times"',)])


class Db2ReimportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.csv_root = root / "csv"
        self.csv_dir = self.csv_root / db2.DEFAULT_BUILD
        write_header_only_csvs(self.csv_dir)
        with (self.csv_dir / "SpellName.csv").open("a", newline="") as handle:
            csv.writer(handle).writerow([35395, "Crusader Strike"])
        self.db = root / "world.db"
        with sqlite3.connect(self.db) as conn:
            conn.execute("CREATE TABLE chr_races (ID INTEGER PRIMARY KEY, Name TEXT)")
            conn.execute("INSERT INTO chr_races VALUES (1, 'Human')")

    def test_reimport_replaces_db2_rows_and_keeps_other_tables(self):
        for _ in range(2):
            result = run_db2_import(self.csv_root, self.db)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute("SELECT ID, Name_lang FROM spell_name").fetchall(), [(35395, "Crusader Strike")])
            self.assertEqual(conn.execute("SELECT * FROM chr_races").fetchall(), [(1, "Human")])
            self.assertEqual(conn.execute("SELECT count(*) FROM db2_build").fetchone()[0], len(db2.TABLES))

    def test_header_mismatch_fails_and_leaves_previous_import_intact(self):
        self.assertEqual(run_db2_import(self.csv_root, self.db).returncode, 0)
        (self.csv_dir / "SpellName.csv").write_text("ID,Name_lang,Extra\n1,Renamed,0\n")
        result = run_db2_import(self.csv_root, self.db)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SpellName.csv header does not match spell_name", result.stderr)
        with sqlite3.connect(self.db) as conn:
            self.assertEqual(conn.execute("SELECT ID, Name_lang FROM spell_name").fetchall(), [(35395, "Crusader Strike")])


@unittest.skipUnless(PINNED_BUILD_DIR.is_dir(), f"pinned DB2 CSVs not present at {PINNED_BUILD_DIR}")
class PinnedBuildImportTests(unittest.TestCase):
    """Import the real pinned wago.tools exports and check known Retail rows."""

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.temp.name) / "world.db"
        result = run_db2_import(PINNED_CSV_ROOT, cls.db)
        if result.returncode != 0:
            raise AssertionError(result.stdout + result.stderr)
        cls.conn = sqlite3.connect(cls.db)

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.temp.cleanup()

    def one(self, sql, *params):
        return self.conn.execute(sql, params).fetchall()

    def test_every_table_records_the_pinned_build(self):
        self.assertEqual(self.one("SELECT DISTINCT build FROM db2_build"), [("12.1.0.69933",)])
        self.assertEqual(self.one("SELECT count(*) FROM db2_build"), [(len(db2.TABLES),)])

    def test_crusader_strike_is_a_paladin_class_skill_ability(self):
        self.assertEqual(
            self.one(
                "SELECT a.ID, a.ClassMask, s.DisplayName_lang FROM skill_line_ability a"
                " JOIN skill_line s ON s.ID = a.SkillLine WHERE a.Spell = 35395"
            ),
            [(43394, 2, "Paladin")],
        )

    def test_retribution_specialization_belongs_to_paladin(self):
        self.assertEqual(
            self.one("SELECT Name_lang, ClassID, MasterySpellID_0 FROM chr_specialization WHERE ID = 70"),
            [("Retribution", 2, 267316)],
        )

    def test_kick_interrupt_effect_is_68(self):
        self.assertEqual(
            self.one("SELECT EffectIndex, Effect FROM spell_effect WHERE SpellID = 1766 AND DifficultyID = 0"),
            [(0, 68)],
        )

    def test_selfless_healer_stacks_to_four(self):
        self.assertEqual(
            self.one("SELECT CumulativeAura, ProcChance FROM spell_aura_options WHERE SpellID = 114250"),
            [(4, 101)],
        )

    def test_bolt_of_linen_cloth_takes_two_linen_cloth(self):
        self.assertEqual(
            self.one("SELECT Reagent_0, ReagentCount_0, Reagent_1 FROM spell_reagents WHERE SpellID = 2963"),
            [(2589, 2, 0)],
        )

    def test_white_linen_shirt_is_a_classic_tailoring_shirt(self):
        self.assertEqual(
            self.one(
                "SELECT c.Name_lang, c.ParentTradeSkillCategoryID, c.SkillLineID FROM skill_line_ability a"
                " JOIN trade_skill_category c ON c.ID = a.TradeSkillCategoryID WHERE a.Spell = 2393"
            ),
            [("Shirts", 362, 197)],
        )

    def test_blacksmith_hammer_is_totem_category_162(self):
        self.assertEqual(
            self.one("SELECT Name_lang, TotemCategoryType, TotemCategoryMask FROM totem_category WHERE ID = 162"),
            [("Blacksmith Hammer", 24, 4)],
        )

    def test_classic_tailoring_uses_skill_tier_336(self):
        self.assertEqual(self.one("SELECT SkillTierID FROM skill_race_class_info WHERE SkillID = 2540"), [(336,)])


class TdbExcerptImportTests(unittest.TestCase):
    """Import the real TDB1210 excerpts and check Stormwind's Retail tailoring trainer."""

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        db = Path(cls.temp.name) / "world.db"
        pairs = [f"{name}={TDB_DIR / (name + '.sql')}" for name in TRAINER_TDB_TABLES]
        subprocess.run(
            [sys.executable, str(SCRIPTS / "mysql_dump.py"), "--db", str(db),
             "--schema", str(SCRIPTS / "tdb_schema.sql"), "--prefix", "tdb_", *pairs],
            check=True, capture_output=True, timeout=120,
        )
        cls.conn = sqlite3.connect(db)

    @classmethod
    def tearDownClass(cls):
        cls.conn.close()
        cls.temp.cleanup()

    def one(self, sql):
        return self.conn.execute(sql).fetchall()

    def test_georgio_bolero_uses_retail_tailoring_trainer(self):
        self.assertEqual(self.one("SELECT TrainerID FROM tdb_creature_trainer WHERE CreatureID = 1346"), [(163,)])

    def test_tailoring_trainer_teaches_the_profession_and_classic_recipes(self):
        self.assertEqual(
            self.one("SELECT SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, ReqLevel FROM tdb_trainer_spell"
                     " WHERE TrainerId = 163 AND SpellId IN (264617, 2393) ORDER BY SpellId"),
            [(2393, 10, 2540, 1, 0), (264617, 10, 0, 0, 5)],
        )

    def test_retail_trainer_offers_no_apprentice_rank_spell(self):
        self.assertEqual(self.one("SELECT count(*) FROM tdb_trainer_spell WHERE TrainerId = 163 AND SpellId = 3911"), [(0,)])

    def test_classic_tier_caps_at_300(self):
        self.assertEqual(self.one("SELECT Value1, Value2 FROM tdb_skill_tiers WHERE ID = 336"), [(300, 0)])


if __name__ == "__main__":
    unittest.main()


class ContentTuningLevelTests(unittest.TestCase):
    """TrinityCore GetContentTuningData level ranges under Expansion 11 (Midnight)."""

    def test_fixed_range_is_kept(self):
        self.assertEqual(retail_content.content_tuning_levels(5, 30, 0, 0), (5, 30))

    def test_max_level_type_adds_the_midnight_cap(self):
        # ContentTuning 1/864: MaxLevelSquish 0 + GetMaxLevelForExpansion(11).
        self.assertEqual(retail_content.content_tuning_levels(1, 0, 0, 2), (1, 90))

    def test_min_level_type_adds_one_and_prev_expansion_type_the_war_within_cap(self):
        self.assertEqual(retail_content.content_tuning_levels(0, 0, 1, 3), (1, 80))

    def test_levels_clamp_to_one(self):
        self.assertEqual(retail_content.content_tuning_levels(0, 0, 0, 0), (1, 1))

    def test_quest_min_level_adds_target_level_delta_before_clamping(self):
        self.assertEqual(retail_content.quest_min_level(5, 0, 2), 7)
        self.assertEqual(retail_content.quest_min_level(0, 1, -3), 1)
        self.assertEqual(retail_content.quest_min_level(0, 2, 0), 90)

    def test_horde_and_alliance_flags_give_scaling_faction_groups(self):
        self.assertEqual([retail_content.scaling_faction_group(f) for f in (0, 0x08, 0x10, 0x18)], [0, 5, 3, 5])


import db2_casc  # noqa: E402


WAGO_VERIFY = SCRIPTS.parent / "data/reference/wago-verify"
# Rows copied from wago.tools' 12.1.0.69933 exports (data/reference/wago-verify); wago prints
# float32 values rounded to 11 significant digits.
WAGO_ROWS = {
    "QuestMoneyReward": "10,0,200,500,1000,1500,2000,4000,6000,8000,0",
    "ContentTuning": "2802,0,10,0,0,0,0,0,0,80,80,0,0,0,0,0,0,0,1.20000004768",
    "ExpectedStat": "1,-2,78,292.24713134766,4.75353193283,50,600,20,0,116,292.24713134766,0,1",
    "ExpectedStatMod": "405,0.90844345093,0.9357034564,0.9357034564,1,1,0.93159759045,0.92000001669,0.88179254532,"
                       "0.9357034564",
    "ContentTuningXExpected": "13,5,0,0,500",
}


def float32(value):
    return struct.unpack("<f", struct.pack("<f", float(value)))[0]


def same_cells(decoded, exported):
    """Decoded values equal wago's cells at float32 precision."""
    return len(decoded) == len(exported) and all(float32(a) == float32(b) for a, b in zip(decoded, exported))


@unittest.skipUnless(db2_casc.CASC_LOCAL.is_file(), f"casc-local not present at {db2_casc.CASC_LOCAL}")
class LocalDb2ExtractTests(unittest.TestCase):
    """Decode the LOCAL_TABLES straight from the local CASC install and check them against wago.tools."""

    @classmethod
    def setUpClass(cls):
        schema = sqlite3.connect(":memory:")
        schema.executescript(db2.SCHEMA_SQL.read_text())
        cls.tables = {}
        for table, (fdid, layout) in db2.LOCAL_TABLES.items():
            columns = [(row[1], row[2]) for row in schema.execute(f"PRAGMA table_info({db2.table_name(table)})")]
            header, rows = db2_casc.extract_rows(fdid, layout, columns)
            cls.tables[table] = (header, {row[0]: row for row in rows})

    def test_wago_rows_decode_identically(self):
        for table, line in WAGO_ROWS.items():
            exported = line.split(",")
            with self.subTest(table=table):
                decoded = self.tables[table][1][int(exported[0])]
                self.assertTrue(same_cells(decoded, exported), (decoded, exported))

    def test_pallet_array_id_list_and_copy_table(self):
        # QuestMoneyReward: 90 records + 40 copy-table rows, ID from the non-inline ID list,
        # Difficulty_0..9 from one pallet-array field.
        self.assertEqual(len(self.tables["QuestMoneyReward"][1]), 130)

    def test_relationship_map_is_the_last_column(self):
        # ExpectedStat.Lvl and ContentTuningXExpected.ContentTuningID come from the relationship map.
        self.assertEqual(self.tables["ExpectedStat"][0][-1], "Lvl")
        self.assertEqual(self.tables["ContentTuningXExpected"][1][13][-1], 500)

    @unittest.skipUnless(WAGO_VERIFY.is_dir(), f"wago.tools exports not present at {WAGO_VERIFY}")
    def test_whole_tables_match_the_wago_exports(self):
        for table, (header, rows) in self.tables.items():
            with self.subTest(table=table):
                with (WAGO_VERIFY / f"{table}.csv").open(newline="") as handle:
                    exported = list(csv.reader(handle))
                self.assertEqual(exported[0], header)
                self.assertEqual(sorted(int(row[0]) for row in exported[1:]), sorted(rows))
                bad = [row[0] for row in exported[1:] if not same_cells(rows[int(row[0])], row)]
                self.assertEqual(bad, [])


def extract_local(db2_name):
    """{ID: {column: value}} of a LOCAL_TABLES DB2 decoded from the local CASC install."""
    schema = sqlite3.connect(":memory:")
    schema.executescript(db2.SCHEMA_SQL.read_text())
    columns = [(row[1], row[2]) for row in schema.execute(f"PRAGMA table_info({db2.table_name(db2_name)})")]
    fdid, layout = db2.LOCAL_TABLES[db2_name]
    header, rows = db2_casc.extract_rows(fdid, layout, columns)
    return {row[0]: dict(zip(header, row)) for row in rows}


@unittest.skipUnless(db2_casc.CASC_LOCAL.is_file(), f"casc-local not present at {db2_casc.CASC_LOCAL}")
class LocalExpectedStatExtractTests(unittest.TestCase):
    """ExpectedStat (ID list + Lvl relationship), ExpectedStatMod (ID list, copies) and
    ContentTuningXExpected (ID list + ContentTuningID relationship) from local CASC."""

    @classmethod
    def setUpClass(cls):
        cls.stats = extract_local("ExpectedStat")
        cls.mods = extract_local("ExpectedStatMod")
        cls.tuning_mods = extract_local("ContentTuningXExpected")

    def test_every_record_decodes(self):
        self.assertEqual((len(self.stats), len(self.tuning_mods)), (232, 1072))
        self.assertEqual(len(self.mods), 265 + 70)

    def test_levels_come_from_the_relationship_field(self):
        default_levels = sorted(row["Lvl"] for row in self.stats.values() if row["ExpansionID"] == -2)
        self.assertEqual(default_levels, list(range(1, 134)))
        self.assertEqual((self.stats[1]["Lvl"], self.stats[1]["CreatureHealth"]), (1, 78.0))
        self.assertEqual((self.stats[475]["Lvl"], self.stats[475]["CreatureHealth"]), (90, 141482.015625))

    def test_default_creature_health_grows_to_level_90_then_holds(self):
        by_level = {row["Lvl"]: row["CreatureHealth"] for row in self.stats.values() if row["ExpansionID"] == -2}
        self.assertTrue(all(by_level[level] < by_level[level + 1] for level in range(1, 90)))
        self.assertEqual({by_level[level] for level in range(90, 134)}, {141482.015625})

    def test_legion_level_73_has_two_rows(self):
        # DB2Manager::LoadStores keys _expectedStatsByLevel by (Lvl, ExpansionID) in ID order,
        # so the later ID (569) is the one TrinityCore evaluates.
        rows = sorted(i for i, row in self.stats.items() if (row["Lvl"], row["ExpansionID"]) == (73, 6))
        self.assertEqual(rows, [566, 569])

    def test_class_mods_trinitycore_reads_exist(self):
        # DB2Manager::EvaluateExpectedStat: warrior 4, paladin 2, rogue 3, mage 1.
        self.assertTrue({1, 2, 3, 4} <= self.mods.keys())

    def test_content_tuning_mods_reference_expected_stat_mods(self):
        referenced = {row["ExpectedStatModID"] for row in self.tuning_mods.values()}
        self.assertTrue(referenced <= self.mods.keys(), referenced - self.mods.keys())
        self.assertTrue(all(row["ContentTuningID"] > 0 for row in self.tuning_mods.values()))
