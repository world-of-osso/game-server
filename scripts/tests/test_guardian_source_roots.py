"""Native summon-effect template publication, with real DB2/TDB rows on a private copy."""
import importlib.util
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("guardian_templates", ROOT / "scripts/add_guardian_templates.py")
guardian_templates = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guardian_templates)


class GuardianSourceRootFixtureTests(unittest.TestCase):
    def test_native_effects_select_only_summoned_creatures_and_deduplicate_roots(self):
        with sqlite3.connect(":memory:") as db:
            db.execute("CREATE TABLE spell_effect (SpellID INT,DifficultyID INT,Effect INT,EffectMiscValue_0 INT)")
            db.executemany("INSERT INTO spell_effect VALUES (?,?,?,?)", [
                (104317, 0, 28, 55659), (193331, 0, 28, 98035), (193332, 0, 28, 98035),
                (104317, 1, 28, 17252), (104317, 0, 3, 17252),
            ])
            self.assertEqual(guardian_templates.summon_entries(db, [104317, 193331, 193332]), [55659, 98035])
            with self.assertRaisesRegex(ValueError, "spell 686 has no native creature summon effect"):
                guardian_templates.summon_entries(db, [104317, 686])


@unittest.skipUnless(os.environ.get("DPS_PARITY_WORLD_DB"), "DPS_PARITY_WORLD_DB native DB fixture is unset")
class GuardianSourceRootsTests(unittest.TestCase):
    def test_explicit_native_summon_roots_publish_exact_tdb_models_without_world_spawns(self):
        root = ROOT
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
            script = Path(directory) / "scripts/add_guardian_templates.py"
            script.parent.mkdir()
            shutil.copyfile(root / "scripts/add_guardian_templates.py", script)
            environment = dict(os.environ, PYTHONPATH=str(root / "scripts"))
            command = [sys.executable, str(script),
                       "--db", str(prepared_path), "--spell", "104317",
                       "--spell", "193331", "--spell", "193332"]
            for attempt in range(2):
                result = subprocess.run(command, capture_output=True, text=True, check=False, env=environment)
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
