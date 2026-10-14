import importlib.util
import json
import os
from pathlib import Path
import sqlite3
from contextlib import closing
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('guide_catalog', Path(__file__).resolve().parents[1] / 'adventure_guide_catalog.py')

class CatalogPrepareTests(unittest.TestCase):
    def test_preparer_preserves_authentic_rows_and_never_writes_world(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        source = Path(os.environ['ADVENTURE_GUIDE_SOURCE'])
        world = Path(os.environ['ADVENTURE_GUIDE_WORLD_COPY'])
        before = world.stat().st_mtime_ns
        decoder = module.load_decoder(Path(os.environ['ADVENTURE_GUIDE_DECODER']))
        tables = module.decode_sources(decoder, source / 'db2', source / 'definitions')
        receipt = json.loads((source / 'db2/receipt.json').read_text())
        build = receipt['identity']['build']
        sql, report = module.prepare_sql(tables, world, build)
        self.assertEqual(world.stat().st_mtime_ns, before)
        with tempfile.TemporaryDirectory() as temporary:
            with closing(sqlite3.connect(Path(temporary) / 'catalog.sqlite')) as connection:
                connection.executescript(sql)
                row = json.loads(connection.execute('SELECT payload FROM perks_activity WHERE ID=3').fetchone()[0])
                self.assertEqual((row['ActivityName_lang'], row['CriteriaTreeID'], row['ThresholdContributionAmount']), ('Complete 30 Quests', 135693, 200))
                self.assertEqual(connection.execute('SELECT COUNT(*) FROM perks_activity').fetchone()[0], receipt['tables']['PerksActivity']['rows'])
                self.assertEqual(connection.execute('SELECT COUNT(*) FROM adventure_journal').fetchone()[0], receipt['tables']['AdventureJournal']['rows'])
                self.assertEqual(connection.execute('SELECT build FROM adventure_guide_source WHERE name=?', ('PerksActivity',)).fetchone()[0], build)
        for name in ['PerksActivityThreshold', 'PerksActivityThresholdGroup']:
            self.assertEqual(report['sources'][name]['unreadable'], receipt['tables'][name]['unreadable_records'])
        self.assertIn('no verified', report['month_assignment'])

if __name__ == '__main__':
    unittest.main()
