import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from import_spellclicks import import_spellclicks


class SpellclickImportTests(unittest.TestCase):
    def test_tdb_dump_import_then_content_projection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'world.sql'
            database = root / 'copy.sqlite'
            source.write_text('''CREATE TABLE `npc_spellclick_spells` (
  `npc_entry` int unsigned NOT NULL,
  `spell_id` int unsigned NOT NULL,
  `cast_flags` tinyint unsigned NOT NULL,
  `user_type` smallint unsigned NOT NULL,
  PRIMARY KEY (`npc_entry`,`spell_id`)
) ENGINE=InnoDB;
INSERT INTO `npc_spellclick_spells` VALUES (244941,1239001,1,0),(245716,1239300,1,0);
''')
            sqlite3.connect(database).close()
            script = Path(__file__).resolve().parents[1] / 'import-battlegrounds.py'
            result = subprocess.run([sys.executable, str(script), '--dump', str(source), '--db', str(database), '--table', 'npc_spellclick_spells'], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(database) as db:
                self.assertEqual(import_spellclicks(db), 2)
                self.assertEqual(db.execute('SELECT * FROM content_npc_spellclick_spells ORDER BY npc_entry').fetchall(), [
                    (244941,1239001,1,0),(245716,1239300,1,0),
                ])

    def test_projects_all_source_rows_and_preserves_unrelated_content(self):
        db = sqlite3.connect(':memory:')
        self.addCleanup(db.close)
        db.executescript('''
            CREATE TABLE tdb_npc_spellclick_spells (
                npc_entry INTEGER, spell_id INTEGER, cast_flags INTEGER, user_type INTEGER,
                PRIMARY KEY(npc_entry,spell_id));
            INSERT INTO tdb_npc_spellclick_spells VALUES
                (244941,1239001,1,0),(245716,1239300,1,0),(60283,124491,1,0);
            CREATE TABLE content_npc_spellclick_spells (
                npc_entry INTEGER, spell_id INTEGER, cast_flags INTEGER, user_type INTEGER,
                PRIMARY KEY(npc_entry,spell_id));
            INSERT INTO content_npc_spellclick_spells VALUES
                (244941,1239001,0,3),(999999,1,1,0);
        ''')
        self.assertEqual(import_spellclicks(db), 3)
        self.assertEqual(import_spellclicks(db), 3)
        self.assertEqual(db.execute('SELECT * FROM content_npc_spellclick_spells ORDER BY npc_entry').fetchall(), [
            (60283,124491,1,0),(244941,1239001,1,0),(245716,1239300,1,0),(999999,1,1,0),
        ])


if __name__ == '__main__':
    unittest.main()
