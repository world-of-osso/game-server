import sqlite3
import tempfile
import unittest
from pathlib import Path

from ruby_population import ERKHART_XY, ERKHART_Z, KYRAKKA_XY, KYRAKKA_Z, import_population

ROOT = Path(__file__).resolve().parents[2]


class RubyPopulationTests(unittest.TestCase):
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


if __name__ == '__main__':
    unittest.main()
