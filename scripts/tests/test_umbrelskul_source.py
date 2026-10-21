"""Umbrelskul local Lua binding preserves all actual world inputs."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from encounter_summons import import_umbrelskul

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'


class UmbrelskulSourceTests(unittest.TestCase):
    def test_only_local_script_binding_changes_and_rejects_source_drift(self):
        fixtures = ROOT / 'target/encounter-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'umbrelskul.sqlite') as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                before = dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=186738').fetchone()))
                tables = [('content_creature', 'SELECT * FROM content_creature WHERE id1=186738'),
                          ('difficulties', 'SELECT * FROM content_creature_template_difficulty WHERE entry=186738 ORDER BY DifficultyID'),
                          ('models', 'SELECT * FROM content_creature_template_model WHERE CreatureID=186738'),
                          ('addons', 'SELECT * FROM content_creature_template_addon WHERE entry=186738'),
                          ('bindings', 'SELECT * FROM raid_encounter_creature ORDER BY map_id,creature_entry')]
                retained = {name: db.execute(query).fetchall() for name, query in tables}
                population = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                self.assertEqual(import_umbrelskul(db), [186738])
                expected = dict(before, ScriptName='boss_umbrelskul')
                self.assertEqual(dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=186738').fetchone())), expected)
                self.assertEqual({name: db.execute(query).fetchall() for name, query in tables}, retained)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], population)
                import_umbrelskul(db)
                self.assertEqual(dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=186738').fetchone())), expected)
                db.execute("UPDATE tdb_creature_template SET ScriptName='different_source' WHERE entry=186738")
                with self.assertRaisesRegex(ValueError, 'source identity changed'):
                    import_umbrelskul(db)


if __name__ == '__main__':
    unittest.main()
