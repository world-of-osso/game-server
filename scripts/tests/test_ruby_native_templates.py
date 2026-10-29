"""Verify native Ruby templates on an isolated database copy."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

from ruby_native_templates import import_templates

ROOT = Path(__file__).resolve().parents[2]


class RubyNativeTemplatesTests(unittest.TestCase):
    def test_source_templates_and_models_are_imported_without_spawns(self):
        with tempfile.TemporaryDirectory() as directory:
            with sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'copy.sqlite') as db:
                source.backup(db)
                before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                with db:
                    import_templates(db)
                for entry, name, display in [(190484, 'Kyrakka', 107137), (189886, 'Blazebound Firestorm', 102505)]:
                    self.assertEqual(db.execute('SELECT name FROM content_creature_template WHERE entry=?', (entry,)).fetchone(), (name,))
                    self.assertEqual(db.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=?', (entry,)).fetchall(), [(display,)])
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], before)


if __name__ == '__main__':
    unittest.main()
