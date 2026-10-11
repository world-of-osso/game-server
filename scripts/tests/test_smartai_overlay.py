import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path


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


if __name__ == "__main__":
    unittest.main()
