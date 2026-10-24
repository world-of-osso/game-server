"""Authentic TDB Howl actors selected by the normal class-summon importer."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
import add_class_summon_templates
import retail_content


class HowlSummonClosureTests(unittest.TestCase):
    def test_normal_class_closure_reaches_native_wyvern_and_bear_templates(self):
        source = Path(os.environ['DPS_PARITY_WORLD_DB']).resolve()
        with closing(sqlite3.connect(source.as_uri() + '?mode=ro', uri=True)) as db:
            authored = dict(db.execute('SELECT entry,name FROM tdb_creature_template WHERE entry IN(234170,234018)'))
            self.assertEqual(authored, {234018: 'Bear', 234170: 'Wyvern'})
            reached = {row[0] for row in db.execute(retail_content.CLASS_SUMMON_ENTRIES)}
            self.assertTrue({234018, 234170}.issubset(reached),
                            'named Howl policy links must feed normal native trigger closure')

    def test_normal_import_copies_authentic_actor_identity_without_world_spawns(self):
        source = Path(os.environ['DPS_PARITY_WORLD_DB']).resolve()
        with tempfile.TemporaryDirectory(prefix='howl-closure-') as directory:
            output = Path(directory) / 'howl.private.db'
            with closing(sqlite3.connect(source.as_uri() + '?mode=ro', uri=True)) as native:
                with closing(sqlite3.connect(output)) as db:
                    native.backup(db)
                    before_spawns = list(db.execute('SELECT * FROM content_creature ORDER BY guid'))
                    added = add_class_summon_templates.add_class_summon_templates(db)
                    rows = dict(db.execute('SELECT entry,name FROM content_creature_template WHERE entry IN(234170,234018)'))
                    self.assertEqual(rows, {234018: 'Bear', 234170: 'Wyvern'})
                    self.assertTrue({234018, 234170}.issubset(added))
                    self.assertEqual(list(db.execute('SELECT * FROM content_creature ORDER BY guid')), before_spawns)
                    model_rows = list(db.execute('SELECT CreatureID,CreatureDisplayID FROM content_creature_template_model WHERE CreatureID IN(234170,234018)'))
                    self.assertTrue(model_rows, 'normal importer includes authored model rows')


if __name__ == '__main__':
    unittest.main()
