import os
import re
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from ruby_population import ERKHART_XY, ERKHART_Z, KYRAKKA_XY, KYRAKKA_Z, POLICY, import_population

ROOT = Path(__file__).resolve().parents[2]


def placeholder_copy(directory):
    """A canonical copy with Erkhart back on its journal placeholder and no Kyrakka row."""
    db = sqlite3.connect(Path(directory) / 'copy.sqlite')
    with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
        source.backup(db)
    comment = db.execute('SELECT Comment FROM content_creature WHERE guid=63521000').fetchone()[0]
    if comment.startswith(POLICY):
        xyz, original = re.search(r'; replaced xyz=\(([^)]*)\); (.*)$', comment).groups()
        with db:
            db.execute('UPDATE content_creature SET position_x=?,position_y=?,position_z=?,Comment=? WHERE guid=63521000',
                       (*map(float, xyz.split(', ')), original))
            db.execute('DELETE FROM content_creature WHERE guid=63521001')
    return db


def ruby_rows(db):
    return [
        db.execute('SELECT count(*) FROM content_creature').fetchone(),
        db.execute('SELECT * FROM content_creature WHERE map=2521 ORDER BY guid').fetchall(),
        db.execute('SELECT * FROM content_creature_template WHERE entry IN (189886,190484,190485) ORDER BY entry').fetchall(),
        db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 ORDER BY creature_entry').fetchall(),
    ]


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
        with tempfile.TemporaryDirectory() as directory, closing(placeholder_copy(directory)) as db:
            before = db.execute('SELECT count(*) FROM content_creature').fetchone()[0]
            with db:
                import_population(db)
            self.assertEqual(db.execute('SELECT position_x,position_y,position_z FROM content_creature WHERE guid=63521000').fetchone(), (*ERKHART_XY, ERKHART_Z))
            self.assertEqual(db.execute('SELECT position_x,position_y,position_z FROM content_creature WHERE guid=63521001').fetchone(), (*KYRAKKA_XY, KYRAKKA_Z))
            self.assertEqual(db.execute('SELECT spawntimesecs FROM content_creature WHERE guid IN (63521000,63521001) ORDER BY guid').fetchall(), [(7200,), (7200,)])
            comment = db.execute('SELECT Comment FROM content_creature WHERE guid=63521000').fetchone()[0]
            self.assertIn('LOG_DERIVED_POPULATION', comment)
            self.assertIn('FOLLOWER_PLACEMENT_XY_FROM_JOURNAL_ENCOUNTER_UIMAP', comment)
            self.assertEqual(db.execute('SELECT count(*) FROM content_creature WHERE id1=190484 AND map=2521').fetchone()[0], 1)
            self.assertLessEqual(db.execute('SELECT count(*) FROM content_creature').fetchone()[0], before + 1)
            self.assertEqual(db.execute('SELECT faction,unit_flags FROM content_creature_template WHERE entry=190484').fetchone(), (16, 64))
            self.assertEqual(db.execute('SELECT entry,ScriptName FROM content_creature_template WHERE entry IN (190484,190485) ORDER BY entry').fetchall(), [(190484, 'boss_kyrakka_erkhart'), (190485, 'boss_kyrakka_erkhart')])
            self.assertEqual(db.execute('SELECT encounter_id,bit FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=190484').fetchone(), (2623, 2))

    def test_database_already_holding_the_placements_is_unchanged(self):
        with tempfile.TemporaryDirectory() as directory, closing(placeholder_copy(directory)) as db:
            with db:
                import_population(db)
            applied = ruby_rows(db)
            with db:
                import_population(db)
            self.assertEqual(ruby_rows(db), applied)

    def test_half_applied_placements_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory, closing(placeholder_copy(directory)) as db:
            with db:
                import_population(db)
                db.execute('DELETE FROM content_creature WHERE guid=63521001')
            with self.assertRaisesRegex(ValueError, 'partially applied'):
                import_population(db)

if __name__ == '__main__':
    unittest.main()
