"""Native summon-effect template publication, with real DB2/TDB rows on a private copy."""
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest


class GuardianSourceRootsTests(unittest.TestCase):
    def test_explicit_native_summon_roots_publish_exact_tdb_models_without_world_spawns(self):
        root = Path(__file__).resolve().parents[1]
        source_path = Path(os.environ["DPS_PARITY_WORLD_DB"])
        with tempfile.TemporaryDirectory(prefix="guardian-source-roots-") as directory:
            prepared_path = Path(directory) / "prepared.sqlite"
            with sqlite3.connect(f"file:{source_path}?mode=ro", uri=True) as source:
                with sqlite3.connect(prepared_path) as prepared:
                    source.backup(prepared)
            with sqlite3.connect(prepared_path) as prepared:
                before_spawns = prepared.execute("SELECT COUNT(*) FROM content_creature").fetchone()[0]
                before_templates = prepared.execute("SELECT COUNT(*) FROM content_creature_template").fetchone()[0]
                source_entries = prepared.execute(
                    "SELECT DISTINCT EffectMiscValue_0 FROM spell_effect "
                    "WHERE SpellID IN (104317,193331,193332) AND DifficultyID=0 AND Effect IN (28,56) "
                    "ORDER BY EffectMiscValue_0"
                ).fetchall()
                self.assertEqual(source_entries, [(55659,), (98035,)])
            command = [sys.executable, str(root / "scripts/add_guardian_templates.py"),
                       "--db", str(prepared_path), "--spell", "104317",
                       "--spell", "193331", "--spell", "193332"]
            for attempt in range(2):
                result = subprocess.run(command, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with sqlite3.connect(prepared_path) as prepared:
                    self.assertEqual(prepared.execute(
                        "SELECT entry,name FROM content_creature_template WHERE entry IN (55659,98035) ORDER BY entry"
                    ).fetchall(), [(55659, "Wild Imp"), (98035, "Dreadstalker")])
                    for entry in (55659, 98035):
                        actual = prepared.execute(
                            "SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx", (entry,)
                        ).fetchall()
                        expected = prepared.execute(
                            "SELECT * FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx", (entry,)
                        ).fetchall()
                        self.assertTrue(expected)
                        self.assertEqual(actual, expected, f"all TDB models/builds preserved on attempt{attempt}")
                    self.assertEqual(prepared.execute("SELECT COUNT(*) FROM content_creature").fetchone()[0], before_spawns)
                    self.assertEqual(prepared.execute("SELECT COUNT(*) FROM content_creature_template").fetchone()[0], before_templates + 2)


if __name__ == "__main__":
    unittest.main()
