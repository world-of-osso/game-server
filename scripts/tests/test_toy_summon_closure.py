"""Real Toy→ItemEffect→SpellEffect dependencies, imported without invented templates."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

# Highborne Soul Mirror95007 summons51337 directly. Golden Banana116484 reaches
# 60383 through116485. These are actual TDB identities, not behavior substitutes.
ENTRIES = (51337, 60383)


class ToySummonClosureImportTests(unittest.TestCase):
    def test_direct_and_recursive_dependencies_preserve_source_and_existing_rows(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target") as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                conn.execute("DELETE FROM content_creature_template_model WHERE CreatureID IN (?,?)", ENTRIES)
                conn.execute("DELETE FROM content_creature_template_addon WHERE entry IN (?,?)", ENTRIES)
                conn.execute("DELETE FROM content_creature_template WHERE entry IN (?,?)", ENTRIES)
                retained = conn.execute("SELECT * FROM content_creature_template WHERE entry=65387").fetchone()
                source = conn.execute("SELECT entry,name,faction,unit_flags,ScriptName FROM tdb_creature_template WHERE entry IN (?,?) ORDER BY entry", ENTRIES).fetchall()
                source_models = conn.execute("SELECT * FROM tdb_creature_template_model WHERE CreatureID IN (?,?) ORDER BY CreatureID,Idx", ENTRIES).fetchall()
            for _ in range(2):
                result = subprocess.run([sys.executable, str(ROOT / "scripts/add_toy_summon_templates.py"), "--db", str(path)], text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute("SELECT entry,name,faction,unit_flags,ScriptName FROM content_creature_template WHERE entry IN (?,?) ORDER BY entry", ENTRIES).fetchall(), source)
                self.assertEqual(conn.execute("SELECT * FROM content_creature_template_model WHERE CreatureID IN (?,?) ORDER BY CreatureID,Idx", ENTRIES).fetchall(), source_models)
                self.assertEqual(conn.execute("SELECT * FROM content_creature_template WHERE entry=65387").fetchone(), retained)
                # Source-missing ice-fishing tent cannot turn into a fabricated actor.
                self.assertIsNone(conn.execute("SELECT entry FROM content_creature_template WHERE entry=265250").fetchone())


if __name__ == "__main__":
    unittest.main()
