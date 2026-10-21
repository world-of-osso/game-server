"""Native class-summon import behavior against an online backup of authentic data."""
from contextlib import closing
import hashlib
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import retail_content


def spawn_fingerprint(conn):
    digest = hashlib.sha256()
    for row in conn.execute("SELECT * FROM content_creature ORDER BY guid"):
        digest.update(repr(row).encode())
        digest.update(b"\n")
    return digest.hexdigest()


class ClassSummonImportTests(unittest.TestCase):
    def test_efflo_native_summon_import_is_authentic_idempotent_and_spawn_free(self):
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "class-summons.sqlite"
            source = (ROOT / "data/world.db").resolve()
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                entry = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                    WHERE SpellID=145205 AND DifficultyID=0 AND Effect=28""").fetchone()[0]
                self.assertEqual(entry, 47649)
                source_template = conn.execute("""SELECT entry,name,faction,unit_class,
                    AIName,ScriptName FROM tdb_creature_template WHERE entry=?""", (entry,)).fetchone()
                self.assertIsNotNone(source_template)
                source_models = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,
                    DisplayScale,Probability,VerifiedBuild FROM tdb_creature_template_model
                    WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                self.assertTrue(source_models)
                source_addons = conn.execute(f"""SELECT a.entry,{retail_content.ADDON_COLUMNS}
                    FROM tdb_creature_template_addon a WHERE a.entry=?""", (entry,)).fetchall()
                for table, key in (("content_creature_template", "entry"),
                                   ("content_creature_template_model", "CreatureID"),
                                   ("content_creature_template_addon", "entry")):
                    conn.execute(f"DELETE FROM {table} WHERE {key}=?", (entry,))
                before_spawns = spawn_fingerprint(conn)
            for _ in range(2):
                subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                                "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                actual = conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                    FROM content_creature_template WHERE entry=?""", (entry,)).fetchone()
                self.assertEqual(actual, source_template)
                actual_models = conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                self.assertEqual(actual_models, source_models)
                actual_addons = conn.execute("SELECT * FROM content_creature_template_addon WHERE entry=?",
                                             (entry,)).fetchall()
                self.assertEqual(actual_addons, source_addons)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)


if __name__ == "__main__":
    unittest.main()
