"""Exercise the real importer without touching the development world database."""

import csv
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import content_policy  # noqa: E402
import db2  # noqa: E402

AZEROTHCORE_SQL = Path(os.environ.get("AZEROTHCORE_SQL", "/home/osso/Repos/azerothcore/data/sql/base/db_world"))

PINNED_CSV_ROOT = Path(os.environ.get("DB2_CSV_ROOT", db2.DEFAULT_CSV_ROOT))
PINNED_BUILD_DIR = PINNED_CSV_ROOT / db2.DEFAULT_BUILD


def write_header_only_csvs(csv_dir):
    """Write one header-only CSV per DB2 table, headers taken from db2_schema.sql."""
    csv_dir.mkdir(parents=True)
    schema = sqlite3.connect(":memory:")
    schema.executescript(db2.SCHEMA_SQL.read_text())
    for name in db2.TABLES:
        columns = [row[1] for row in schema.execute(f"PRAGMA table_info({db2.table_name(name)})")]
        with (csv_dir / f"{name}.csv").open("w", newline="") as handle:
            csv.writer(handle).writerow(columns)


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
        for name in ("import-azerothcore.sh", "world_schema.sql", "db2.py", "db2_schema.sql", "mysql_dump.py", "content_policy.py"):
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
        (self.csv / content_policy.WRATH_BUILD).mkdir()
        (self.csv / content_policy.WRATH_BUILD / "SpellName.csv").write_text("ID,Name_lang\n")
        self.db = self.root / "data/world.db"

    def run_import(self):
        return subprocess.run(
            [str(self.root / "scripts/import-azerothcore.sh"), str(self.sql)],
            env={**os.environ, "DB2_CSV_ROOT": str(self.csv)},
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

    def test_content_tables_join_quest_vendor_and_trainer_rows(self):
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
        (self.sql / "trainer_spell.sql").write_text(
            "INSERT INTO `trainer_spell` VALUES\n(1,6673,10,0,0,0,0,0,1,0),(1,100,100,0,0,0,0,0,4,0);\n"
        )
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
            self.assertEqual(
                conn.execute("SELECT SpellId, ReqLevel FROM trainer_spell ORDER BY ReqLevel").fetchall(),
                [(6673, 1), (100, 4)],
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

    def test_unparseable_dump_fails_without_partial_tables(self):
        (self.sql / "npc_vendor.sql").write_text("INSERT INTO `npc_vendor` VALUES\n(54,0,2488,0,0,0,0),(54,0,0x1F,0,0,0,0);\n")
        result = self.run_import()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unparseable value", result.stderr)


@unittest.skipUnless(AZEROTHCORE_SQL.is_dir(), f"AzerothCore dumps not present at {AZEROTHCORE_SQL}")
@unittest.skipUnless(PINNED_BUILD_DIR.is_dir(), f"pinned DB2 CSVs not present at {PINNED_BUILD_DIR}")
class AzerothCoreContentTests(unittest.TestCase):
    """Load real AzerothCore dumps + pinned Retail DB2, derive content_* tables, check Elwynn Forest rows."""

    TABLES = (
        "creature_template", "quest_template", "creature_queststarter", "creature_questender",
        "item_template", "npc_vendor", "trainer", "trainer_spell", "creature_default_trainer",
    )

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.db = Path(cls.temp.name) / "world.db"
        cls.report = Path(cls.temp.name) / "content-dropped.csv"
        commands = [
            [sys.executable, str(SCRIPTS / "mysql_dump.py"), "--db", str(cls.db), "--schema", str(SCRIPTS / "world_schema.sql")]
            + [f"{name}={AZEROTHCORE_SQL / (name + '.sql')}" for name in cls.TABLES],
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

    def test_llane_beshere_teaches_retail_warrior_spells(self):
        self.assertEqual(
            self.rows(
                "SELECT s.SpellId, n.Name_lang, s.ReqLevel, s.MoneyCost FROM creature_default_trainer c"
                " JOIN content_trainer_spell s ON s.TrainerId = c.TrainerId JOIN spell_name n ON n.ID = s.SpellId"
                " WHERE c.CreatureId = 911 ORDER BY s.ReqLevel, s.SpellId"
            ),
            [
                (1464, "Slam", 1, 0), (88163, "Attack", 1, 0), (100, "Charge", 2, 0), (23922, "Shield Slam", 3, 0),
                (1715, "Hamstring", 4, 0), (34428, "Victory Rush", 5, 0), (2565, "Shield Block", 6, 0),
                (6552, "Pummel", 7, 0), (355, "Taunt", 8, 0), (1680, "Whirlwind", 9, 0),
                (6673, "Battle Shout", 10, 0), (57755, "Heroic Throw", 10, 0), (163201, "Execute", 10, 0),
                (18499, "Berserker Rage", 12, 0),
            ],
        )
        self.assertEqual(
            self.dropped[("trainer_spell", "TrainerId=2", 772)],
            "class trainer: Retail class skill line replaces AzerothCore ranks",
        )

    def test_wrath_only_items_are_dropped_and_reported(self):
        self.assertEqual(self.rows("SELECT count(*) FROM content_item WHERE ID IN (41, 2892)"), [(0,)])
        self.assertEqual(self.dropped[("item_template", "entry=41", 41)], "no Retail row")
        self.assertEqual(
            self.dropped[("item_template", "entry=2892", 2892)], "Retail name 'Empty Poison Vial' != 'Deadly Poison'")
        self.assertEqual(self.rows("SELECT count(*) FROM content_npc_vendor WHERE entry = 1325 AND item = 2892"), [(0,)])
        self.assertEqual(self.dropped[("npc_vendor", "entry=1325", 2892)], "item not mapped")

    def test_reviewed_override_keeps_renamed_okra(self):
        self.assertEqual(self.rows("SELECT Name, SourceEntry FROM content_item WHERE ID = 732"), [("Ripe Okra", 732)])

    def test_quests_are_kept_only_when_retail_has_the_id(self):
        self.assertEqual(
            self.rows(
                "SELECT q.ID, t.name FROM content_quest_template q JOIN content_creature_queststarter s ON s.quest = q.ID"
                " JOIN creature_template t ON t.entry = s.id WHERE q.ID IN (16, 783)"
            ),
            [(783, "Deputy Willem")],
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

    def test_northshire_warrior_trainer_llane_beshere_teaches_early_warrior_spells(self):
        self.assertEqual(
            self.rows(
                "SELECT ts.SpellId, ts.MoneyCost, ts.ReqLevel FROM creature_default_trainer c"
                " JOIN trainer t ON t.Id = c.TrainerId JOIN trainer_spell ts ON ts.TrainerId = t.Id"
                " WHERE c.CreatureId = 911 ORDER BY ts.ReqLevel, ts.SpellId"
            ),
            [(6673, 10, 1), (100, 100, 4), (772, 100, 4), (3127, 100, 6), (6343, 100, 6), (34428, 100, 6)],
        )

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


if __name__ == "__main__":
    unittest.main()
