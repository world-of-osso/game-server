"""Native policy duck import, on a private normal class-summon fixture copy."""
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / 'crates/server/tests/fixtures/prepare_class_summon_db.py'
spec = importlib.util.spec_from_file_location('prepare_class_summon_db', HELPER)
prepare_class = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare_class)


class NativeFollowerDuckImportTests(unittest.TestCase):
    def test_policy_duck_uses_native_template_model_and_no_world_spawn(self):
        cache = ROOT / 'target/class-fixtures'
        cache.mkdir(parents=True, exist_ok=True)
        source = (ROOT / 'data/world.db').resolve()
        with tempfile.TemporaryDirectory(dir=cache) as directory:
            target = Path(directory) / 'policy-duck.copy.db'
            prepare_class.prepare(source, target)
            with sqlite3.connect(source.as_uri() + '?mode=ro', uri=True) as native, sqlite3.connect(target) as copied:
                row = copied.execute('SELECT family,type,type_flags,HealthModifier,DamageModifier,BaseAttackTime,rank FROM content_creature_template WHERE entry=174884').fetchone()
                self.assertIsNotNone(row, 'FOLLOWER_HUNTER_PET_SIMC_DEFAULT needs native174884 in normal class-template closure')
                expected = native.execute('SELECT t.family,t.type,d.TypeFlags,d.HealthModifier,d.DamageModifier,t.BaseAttackTime,t.Classification FROM tdb_creature_template t JOIN tdb_creature_template_difficulty d ON d.Entry=t.entry AND d.DifficultyID=0 WHERE t.entry=174884').fetchone()
                self.assertEqual(row, expected)
                self.assertEqual(row[:3], (125, 1, 1))
                models = copied.execute('SELECT CreatureDisplayID,DisplayScale,Probability FROM content_creature_template_model WHERE CreatureID=174884 ORDER BY Idx').fetchall()
                self.assertEqual(models, native.execute('SELECT CreatureDisplayID,DisplayScale,Probability FROM tdb_creature_template_model WHERE CreatureID=174884 ORDER BY Idx').fetchall())
                self.assertTrue(models)
                self.assertEqual(copied.execute('SELECT COUNT(*) FROM content_creature').fetchone(), native.execute('SELECT COUNT(*) FROM content_creature').fetchone())
                self.assertEqual(copied.execute('SELECT COUNT(*) FROM content_creature WHERE id1=174884').fetchone(), native.execute('SELECT COUNT(*) FROM content_creature WHERE id1=174884').fetchone())


if __name__ == '__main__':
    unittest.main()
