"""Exercise the real TDB1210-to-runtime import on a disposable database copy."""
from contextlib import closing
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from retail_content import TOY_SCENERY_SUMMON_ENTRIES

# These authored identities are the import input; all row expectations come from TDB.
# Podium's independent real-UseToy test anchors its required240220 identity.
ENTRIES = TOY_SCENERY_SUMMON_ENTRIES
BANNER_ITEMS = (163986, 163987, 187834, 187957, 187958, 203716, 208057, 211424,
                218128, 232301, 232302, 232303, 232305, 232306, 232307)
ENTRY_PARAMS = ', '.join('?' for _ in ENTRIES)
ITEM_PARAMS = ', '.join('?' for _ in BANNER_ITEMS)


class ToySceneryImportTests(unittest.TestCase):
    def test_source_models_templates_and_idempotent_import(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                before = set(conn.execute("SELECT entry FROM content_creature_template"))
                source_models = conn.execute(f"""SELECT CreatureID, Idx, CreatureDisplayID,
                    DisplayScale, Probability, VerifiedBuild FROM tdb_creature_template_model
                    WHERE CreatureID IN ({ENTRY_PARAMS}) ORDER BY CreatureID, Idx""", ENTRIES).fetchall()
            for _ in range(2):  # Rerunning the actual CLI must not duplicate rows.
                subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                "--db", str(path)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(path)) as conn, conn:
                after = set(conn.execute("SELECT entry FROM content_creature_template"))
                self.assertTrue(({(entry,) for entry in ENTRIES} - before) <= (after - before))
                source_entries = set(conn.execute("SELECT entry FROM tdb_creature_template"))
                self.assertTrue((after - before) <= source_entries)
                self.assertTrue({(51337,), (60383,)} <= after)
                actual_models = conn.execute(f"""SELECT * FROM content_creature_template_model
                    WHERE CreatureID IN ({ENTRY_PARAMS}) ORDER BY CreatureID, Idx""", ENTRIES).fetchall()
                self.assertEqual(actual_models, source_models)
                actual = conn.execute(f"""SELECT entry, name, faction, npcflag, AIName, ScriptName
                    FROM content_creature_template WHERE entry IN ({ENTRY_PARAMS}) ORDER BY entry""", ENTRIES).fetchall()
                source = conn.execute(f"""SELECT entry, name, faction, npcflag, AIName, ScriptName
                    FROM tdb_creature_template WHERE entry IN ({ENTRY_PARAMS}) ORDER BY entry""", ENTRIES).fetchall()
                self.assertEqual(actual, source)
                self.assertEqual(len(actual), len(ENTRIES))
                actual_items = conn.execute(f"""SELECT ID, Name, Quality, ItemLevel,
                    RequiredLevel, Stackable, AllowableClass, AllowableRaces_0, AllowableRaces_1,
                    Flags_0, Flags_1, Flags_2, Flags_3, Flags_4 FROM content_item
                    WHERE ID IN ({ITEM_PARAMS}) ORDER BY ID""", BANNER_ITEMS).fetchall()
                source_items = conn.execute(f"""SELECT ID, Display_lang, OverallQualityID, ItemLevel,
                    RequiredLevel, Stackable, AllowableClass, AllowableRaces_0, AllowableRaces_1,
                    Flags_0, Flags_1, Flags_2, Flags_3, Flags_4 FROM item_sparse
                    WHERE ID IN ({ITEM_PARAMS}) ORDER BY ID""", BANNER_ITEMS).fetchall()
                self.assertEqual(actual_items, source_items)
                self.assertEqual(len(actual_items), len(BANNER_ITEMS))

    def test_missing_source_item_rolls_back_templates_models_and_items(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                conn.execute(f"DELETE FROM content_creature_template WHERE entry IN ({ENTRY_PARAMS})", ENTRIES)
                conn.execute(f"DELETE FROM content_creature_template_model WHERE CreatureID IN ({ENTRY_PARAMS})", ENTRIES)
                conn.execute(f"DELETE FROM content_item WHERE ID IN ({ITEM_PARAMS})", BANNER_ITEMS)
                conn.execute("DELETE FROM item WHERE ID = 232306")
            result = subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                     "--db", str(path)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("scenery toys missing authentic Retail items: [232306]", result.stderr)
            with closing(sqlite3.connect(path)) as conn, conn:
                for table, column, values, params in [
                    ("content_creature_template", "entry", ENTRIES, ENTRY_PARAMS),
                    ("content_creature_template_model", "CreatureID", ENTRIES, ENTRY_PARAMS),
                    ("content_item", "ID", BANNER_ITEMS, ITEM_PARAMS),
                ]:
                    self.assertEqual(conn.execute(f"SELECT count(*) FROM {table} WHERE {column} IN ({params})",
                                                  values).fetchone()[0], 0)

    def test_missing_source_models_rolls_back_new_templates(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                conn.execute(f"DELETE FROM content_creature_template WHERE entry IN ({ENTRY_PARAMS})", ENTRIES)
                conn.execute(f"DELETE FROM content_creature_template_model WHERE CreatureID IN ({ENTRY_PARAMS})", ENTRIES)
                conn.execute("DELETE FROM tdb_creature_template_model WHERE CreatureID = 65387")
            result = subprocess.run(["python3", str(ROOT / "scripts/add_toy_summon_templates.py"),
                                     "--db", str(path)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("missing authentic TDB templates/models: [65387]", result.stderr)
            with closing(sqlite3.connect(path)) as conn, conn:
                self.assertEqual(conn.execute(f"""SELECT count(*) FROM content_creature_template
                    WHERE entry IN ({ENTRY_PARAMS})""", ENTRIES).fetchone()[0], 0)
                self.assertEqual(conn.execute(f"""SELECT count(*) FROM content_creature_template_model
                    WHERE CreatureID IN ({ENTRY_PARAMS})""", ENTRIES).fetchone()[0], 0)


if __name__ == "__main__":
    unittest.main()
