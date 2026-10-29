import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from ruby_population import ERKHART_XY, ERKHART_Z, KYRAKKA_XY, KYRAKKA_Z, import_population, retire_superseded_erkhart

ROOT = Path(__file__).resolve().parents[2]


class RubyPopulationTests(unittest.TestCase):
    def test_importers_reject_linked_databases(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'original.sqlite'
            source.touch()
            symlink = Path(directory) / 'symlink.sqlite'
            symlink.symlink_to(source)
            hardlink = Path(directory) / 'hardlink.sqlite'
            os.link(source, hardlink)
            for script in ('ruby_population.py', 'ruby_native_templates.py'):
                for path in (symlink, hardlink):
                    result = subprocess.run([sys.executable, str(ROOT / 'scripts' / script), '--db', str(path)], capture_output=True, text=True)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('independent regular file', result.stderr)
                    self.assertEqual(source.stat().st_size, 0)

    def test_copy_replaces_only_placeholder_and_adds_kyrakka(self):
        with tempfile.TemporaryDirectory() as directory:
            with sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'copy.sqlite') as db:
                source.backup(db)
                before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                with db:
                    import_population(db)
                self.assertEqual(db.execute('SELECT position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone(), (*ERKHART_XY, ERKHART_Z))
                self.assertEqual(db.execute('SELECT position_x,position_y,position_z FROM content_creature WHERE guid=63521001').fetchone(), (*KYRAKKA_XY, KYRAKKA_Z))
                comment = db.execute('SELECT Comment FROM content_creature WHERE guid=63521000').fetchone()[0]
                self.assertIn('LOG_DERIVED_POPULATION', comment)
                self.assertIn('FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP', comment)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], before + 1)
                self.assertEqual(db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190484').fetchone(), (16, 64))
                self.assertEqual(db.execute('SELECT entry,ScriptName FROM content_creature_template WHERE entry IN (190484,190485) ORDER BY entry').fetchall(), [(190484, 'boss_kyrakka_erkhart'), (190485, 'boss_kyrakka_erkhart')])
                self.assertEqual(db.execute('SELECT encounter_id,bit FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=190484').fetchone(), (2623, 2))
                with self.assertRaisesRegex(ValueError, 'GUID already occupied'):
                    import_population(db)

    def test_placed_erkhart_retires_its_log_derived_duplicate(self):
        with tempfile.TemporaryDirectory() as directory:
            with sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'copy.sqlite') as db:
                source.backup(db)
                creatures = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
                observations = db.execute('SELECT count(*) FROM log_derived_population').fetchone()[0]
                with db:
                    self.assertEqual(retire_superseded_erkhart(db), [3276416184])
                self.assertEqual(db.execute('SELECT guid FROM content_creature WHERE map=2521 AND id1=190485').fetchall(), [(63521000,)])
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], creatures - 1)
                self.assertEqual(db.execute('SELECT count(*) FROM log_derived_population').fetchone()[0], observations - 1)
                self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE map=2521 AND id1=190484').fetchone()[0], 1)
                self.assertEqual(retire_superseded_erkhart(db), [])


if __name__ == '__main__':
    unittest.main()
