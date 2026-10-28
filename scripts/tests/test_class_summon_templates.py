"""Native class-summon import behavior against an online backup of authentic data."""
from contextlib import closing
import hashlib
import os
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
    def test_enhancement_lightning_wolf_imports_native_template_without_spawns(self):
        source = Path(os.environ.get("PARITY_263_TEST_DB", ROOT / "data/world.db")).resolve()
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "wolf.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                self.assertEqual(conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                    WHERE SpellID=198506 AND DifficultyID=0 AND Effect=28""").fetchone(), (100820,))
                self.assertIn(100820, {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)})
                for table, key in (("content_creature_template", "entry"),
                                   ("content_creature_template_model", "CreatureID"),
                                   ("content_creature_template_addon", "entry")):
                    conn.execute(f"DELETE FROM {table} WHERE {key}=100820")
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                self.assertEqual(conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                    FROM content_creature_template WHERE entry=100820""").fetchone(),
                    conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                    FROM tdb_creature_template WHERE entry=100820""").fetchone())
                native = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
                    FROM tdb_creature_template_model WHERE CreatureID=100820 ORDER BY Idx""").fetchall()
                self.assertTrue(native)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=100820 ORDER BY Idx""").fetchall(), native)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_diabolist_ritual_promotes_authentic_demon_templates(self):
        source = Path(os.environ.get("PARITY_266_TEST_DB", ROOT / "data/world.db")).resolve()
        expected = ((428571, 228575), (428565, 228576), (434400, 228574))
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "diabolist.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                for spell, entry in expected:
                    native = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                        WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone()
                    self.assertEqual(native, (entry,))
                    for table, key in (("content_creature_template", "entry"),
                                       ("content_creature_template_model", "CreatureID"),
                                       ("content_creature_template_addon", "entry")):
                        conn.execute(f"DELETE FROM {table} WHERE {key}=?", (entry,))
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                for _, entry in expected:
                    with self.subTest(entry=entry):
                        self.assertEqual(conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                            FROM content_creature_template WHERE entry=?""", (entry,)).fetchone(),
                            conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                            FROM tdb_creature_template WHERE entry=?""", (entry,)).fetchone())
                        actual = conn.execute("""SELECT * FROM content_creature_template_model
                            WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                        native = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
                            FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                        self.assertTrue(native)
                        self.assertEqual(actual, native)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_demonology_guardian_talents_import_authentic_templates(self):
        source = Path(os.environ.get("PARITY_266_TEST_DB", ROOT / "data/world.db")).resolve()
        expected = ((1251781, 135816), (279910, 143622), (455465, 226268), (455476, 226269),
                    (1276182, 255147), (1276283, 258242), (1282501, 260076), (1282502, 260077))
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "demonology.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                for spell, entry in expected:
                    self.assertEqual(conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                        WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone(), (entry,))
                    for table, key in (("content_creature_template", "entry"),
                                       ("content_creature_template_model", "CreatureID"),
                                       ("content_creature_template_addon", "entry")):
                        conn.execute(f"DELETE FROM {table} WHERE {key}=?", (entry,))
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                for _, entry in expected:
                    with self.subTest(entry=entry):
                        self.assertEqual(
                            conn.execute("SELECT name FROM content_creature_template WHERE entry=?", (entry,)).fetchone(),
                            conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone())
                        native = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
                            FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall()
                        self.assertTrue(native)
                        self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                            WHERE CreatureID=? ORDER BY Idx""", (entry,)).fetchall(), native)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_call_of_the_ancestors_promotes_native_template(self):
        source = ROOT / "data/world.db"
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "ancestor.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                self.assertEqual(conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                    WHERE SpellID=445624 AND DifficultyID=0 AND Effect=28""").fetchone(), (221177,))
                for table, key in (("content_creature_template", "entry"),
                                   ("content_creature_template_model", "CreatureID"),
                                   ("content_creature_template_addon", "entry")):
                    conn.execute(f"DELETE FROM {table} WHERE {key}=221177")
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                self.assertEqual(conn.execute("SELECT name FROM content_creature_template WHERE entry=221177").fetchone(),
                                 ("Ancestor",))
                native = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
                    FROM tdb_creature_template_model WHERE CreatureID=221177 ORDER BY Idx""").fetchall()
                self.assertTrue(native)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=221177 ORDER BY Idx""").fetchall(), native)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_primal_elementals_are_promoted_from_real_native_summons(self):
        source = Path(os.environ.get("PARITY_262_TEST_DB", ROOT / "data/world.db")).resolve()
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "primal.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            expected = ((118291, 61029, "Primal Fire Elemental"),
                        (157319, 77942, "Primal Storm Elemental"))
            with closing(sqlite3.connect(prepared)) as conn, conn:
                for spell, entry, name in expected:
                    self.assertEqual(conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                        WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone(), (entry,))
                    self.assertEqual(conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?",
                                                  (entry,)).fetchone(), (name,))
                    for table, key in (("content_creature_template", "entry"),
                                       ("content_creature_template_model", "CreatureID"),
                                       ("content_creature_template_addon", "entry")):
                        conn.execute(f"DELETE FROM {table} WHERE {key}=?", (entry,))
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                for _, entry, name in expected:
                    self.assertEqual(conn.execute("SELECT name FROM content_creature_template WHERE entry=?",
                                                  (entry,)).fetchone(), (name,))

    def test_huntmaster_call_reaches_both_authentic_companions(self):
        source = (ROOT / "data/world.db").resolve()
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            for spell, entry, name in ((459733, 228224, "Fenryr"), (459739, 228226, "Hati")):
                native_entry = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                    WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone()[0]
                self.assertEqual(native_entry, entry)
                self.assertEqual(conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone(), (name,))
                self.assertIn(entry, selected, f"Huntmaster Call must reach native {name}")

    def test_unholy_summons_reach_authentic_templates(self):
        source = (ROOT / "data/world.db").resolve()
        expected = {1277098: 258578, 275430: 237409, 1242294: 163366, 1292072: 264321}
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            for spell, entry in expected.items():
                with self.subTest(spell=spell):
                    native = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                        WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone()
                    self.assertEqual(native, (entry,))
                    self.assertIsNotNone(conn.execute(
                        "SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone())
                    self.assertIn(entry, selected)

    def test_raise_abomination_imports_native_template_without_spawns(self):
        source = (ROOT / "data/world.db").resolve()
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "abomination.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                self.assertEqual(conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                    WHERE SpellID=288853 AND DifficultyID=0 AND Effect=28""").fetchone(), (149555,))
                for table, key in (("content_creature_template", "entry"),
                                   ("content_creature_template_model", "CreatureID"),
                                   ("content_creature_template_addon", "entry")):
                    conn.execute(f"DELETE FROM {table} WHERE {key}=?", (149555,))
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                template = conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                    FROM tdb_creature_template WHERE entry=149555""").fetchone()
                self.assertIsNotNone(template)
                self.assertEqual(conn.execute("""SELECT entry,name,faction,unit_class,AIName,ScriptName
                    FROM content_creature_template WHERE entry=149555""").fetchone(), template)
                models = conn.execute("""SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
                    FROM tdb_creature_template_model WHERE CreatureID=149555 ORDER BY Idx""").fetchall()
                self.assertTrue(models)
                self.assertEqual(conn.execute("""SELECT * FROM content_creature_template_model
                    WHERE CreatureID=149555 ORDER BY Idx""").fetchall(), models)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_rider_talents_reach_all_four_authentic_horsemen(self):
        source = (ROOT / "data/world.db").resolve()
        expected = ((444248, 221632), (444251, 221633),
                    (444252, 221634), (444254, 221635))
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            for spell, entry in expected:
                with self.subTest(spell=spell):
                    native = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                        WHERE SpellID=? AND DifficultyID=0 AND Effect=28""", (spell,)).fetchone()
                    self.assertEqual(native, (entry,))
                    self.assertIsNotNone(conn.execute(
                        "SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone())
                    self.assertIn(entry, selected)
        with tempfile.TemporaryDirectory() as directory:
            prepared = Path(directory) / "riders.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as src:
                with closing(sqlite3.connect(prepared)) as dst:
                    src.backup(dst)
            with closing(sqlite3.connect(prepared)) as conn, conn:
                for _, entry in expected:
                    for table, key in (("content_creature_template", "entry"),
                                       ("content_creature_template_model", "CreatureID"),
                                       ("content_creature_template_addon", "entry")):
                        conn.execute(f"DELETE FROM {table} WHERE {key}=?", (entry,))
                before_spawns = spawn_fingerprint(conn)
            subprocess.run([sys.executable, str(ROOT / "scripts/add_class_summon_templates.py"),
                            "--db", str(prepared)], check=True, capture_output=True, text=True)
            with closing(sqlite3.connect(f"file:{prepared}?mode=ro", uri=True)) as conn:
                for _, entry in expected:
                    self.assertEqual(conn.execute("SELECT name FROM content_creature_template WHERE entry=?", (entry,)).fetchone(),
                                     conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone())
                    models = conn.execute("SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild FROM tdb_creature_template_model WHERE CreatureID=? ORDER BY Idx", (entry,)).fetchall()
                    self.assertTrue(models)
                    self.assertEqual(conn.execute("SELECT * FROM content_creature_template_model WHERE CreatureID=? ORDER BY Idx", (entry,)).fetchall(), models)
                self.assertEqual(spawn_fingerprint(conn), before_spawns)

    def test_dire_command_reaches_authentic_native_beast(self):
        source = (ROOT / "data/world.db").resolve()
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            entry = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                WHERE SpellID=219199 AND DifficultyID=0 AND Effect=28""").fetchone()[0]
            self.assertEqual(entry, 110063)
            native = conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone()
            self.assertEqual(native, ("Beast",))
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertIn(entry, selected, "selected Dire Command must retain its actual native summon dependency")

    def test_invocation_closure_reaches_authentic_arcane_phoenix(self):
        source = (ROOT / "data/world.db").resolve()
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            entry = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                WHERE SpellID=448659 AND DifficultyID=0 AND Effect=28""").fetchone()[0]
            native = conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone()
            self.assertEqual(native, ("Arcane Phoenix",))
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertIn(entry, selected)

    def test_yogg_threshold_reaches_authentic_thing_from_beyond(self):
        source = (ROOT / "data/world.db").resolve()
        with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as conn:
            entry = conn.execute("""SELECT EffectMiscValue_0 FROM spell_effect
                WHERE SpellID=373277 AND DifficultyID=0 AND Effect=28""").fetchone()[0]
            self.assertEqual(entry, 189988)
            native = conn.execute("SELECT name FROM tdb_creature_template WHERE entry=?", (entry,)).fetchone()
            self.assertEqual(native, ("Thing From Beyond",))
            selected = {row[0] for row in conn.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertIn(entry, selected, "native Yogg counter must retain its real scripted summon dependency")

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
