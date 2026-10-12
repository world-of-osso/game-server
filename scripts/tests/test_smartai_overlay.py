import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).parents[1]))


class SmartAiOverlayTests(unittest.TestCase):
    def test_generic_overlay_preserves_content_and_is_idempotent(self):
        path = Path(__file__).parents[1] / "import_smartai_overlay.py"
        spec = importlib.util.spec_from_file_location("smartai_overlay", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            db = Path(directory) / "private.db"
            with sqlite3.connect(db) as conn:
                conn.executescript("""
                    CREATE TABLE content_creature_template(entry INTEGER PRIMARY KEY, AIName TEXT, ScriptName TEXT, faction INTEGER);
                    CREATE TABLE tdb_creature_template(entry INTEGER PRIMARY KEY, AIName TEXT, ScriptName TEXT);
                    CREATE TABLE tdb_smart_scripts(entryorguid INTEGER, source_type INTEGER);
                    INSERT INTO content_creature_template VALUES (1978,'','npc_deathstalker_erland',68),(999,'','legacy',35),(1000,'','legacy_without_rows',14),(1001,'SmartAI','',14);
                    INSERT INTO tdb_creature_template VALUES (1978,'SmartAI',''),(999,'','native_script'),(1000,'SmartAI',''),(1001,'SmartAI','');
                    INSERT INTO tdb_smart_scripts VALUES (1978,0),(1000,9),(1001,0);
                """)
            self.assertEqual(module.import_overlay(db), [1978])
            self.assertEqual(module.import_overlay(db), [])
            with sqlite3.connect(db) as conn:
                self.assertEqual(conn.execute("SELECT * FROM content_creature_template ORDER BY entry").fetchall(), [
                    (999,'','legacy',35),(1000,'','legacy_without_rows',14),(1001,'SmartAI','',14),(1978,'SmartAI','',68)])

    def test_authentic_summon_groups_import_and_bad_source_rolls_back(self):
        path = Path(__file__).parents[1] / "import_smartai_overlay.py"
        spec = importlib.util.spec_from_file_location("smartai_overlay", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            db = Path(directory) / "private.db"
            sql = Path(directory) / "groups.sql"
            sql.write_text("INSERT INTO `creature_summon_groups` VALUES (3439,0,0,3282,1075.12,-2983.98,92.9072,5.81195,4,60000,'Mercenary'),(3439,0,1,3451,1014.92,-2945.61,94.5029,2.11416,8,0,'Pilot');\n")
            with sqlite3.connect(db) as conn:
                conn.executescript("CREATE TABLE content_creature_template(entry INTEGER,AIName TEXT,ScriptName TEXT); CREATE TABLE tdb_creature_template(entry INTEGER,AIName TEXT); CREATE TABLE tdb_smart_scripts(entryorguid INTEGER,source_type INTEGER);")
            module.import_overlay(db, sql)
            module.import_overlay(db, sql)
            with sqlite3.connect(db) as conn:
                before = conn.execute("SELECT * FROM tdb_creature_summon_groups ORDER BY groupId").fetchall()
                self.assertEqual(before, [(3439,0,0,3282,1075.12,-2983.98,92.9072,5.81195,4,60000,'Mercenary'),(3439,0,1,3451,1014.92,-2945.61,94.5029,2.11416,8,0,'Pilot')])
            sql.write_text("INSERT INTO `creature_summon_groups` VALUES (broken);\n")
            with self.assertRaises(ValueError):
                module.import_overlay(db, sql)
            with sqlite3.connect(db) as conn:
                self.assertEqual(conn.execute("SELECT * FROM tdb_creature_summon_groups ORDER BY groupId").fetchall(), before)


if __name__ == "__main__":
    unittest.main()
