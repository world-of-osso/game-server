"""Copy-only follower-kit producer behavioral tests against native 70448 inputs."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
DB2 = Path('/home/osso/Projects/world-of-osso/game-engine/data/db2/active')
SOURCE = Path(os.environ['FOLLOWER_KIT_SOURCE_OUTPUT'])


class FollowerKitProducerTests(unittest.TestCase):
    def command(self, output, receipt, diff):
        return [sys.executable, str(ROOT / 'scripts/follower_spec_kits.py'),
                '--world-db', str(ROOT / 'data/world.db'), '--db2-root', str(DB2),
                '--source-json', str(SOURCE), '--output-db', str(output),
                '--receipt', str(receipt), '--sqldiff', str(diff)]

    def test_copy_contains_level_gated_garrick_and_crenna_kits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output, receipt, diff = [root / name for name in ['world.db', 'receipt.json', 'kit.sql']]
            result = subprocess.run(self.command(output, receipt, diff), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(output) as db:
                for spec, spells in [(66, [633, 53600, 26573, 85673]), (105, [774, 18562, 102342, 33763])]:
                    rows = db.execute('SELECT spell_id FROM follower_spec_kit WHERE spec_id=? AND min_level<=32 AND max_level>=32', (spec,)).fetchall()
                    self.assertTrue(set(spells).issubset({row[0] for row in rows}), (spec, rows))
                self.assertFalse(db.execute('SELECT 1 FROM follower_spec_kit WHERE min_level>max_level').fetchone())
                self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0], 'ok')
            report = json.loads(receipt.read_text())
            self.assertEqual(report['build'], '12.1.5.70448')
            self.assertEqual(len(report['chosen_loadouts']), 11 * 90)
            self.assertEqual(report['changed_tables'], ['follower_spec_kit'])
            self.assertIn('CREATE TABLE follower_spec_kit', diff.read_text())
            with sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True) as db:
                self.assertFalse(db.execute("SELECT 1 FROM sqlite_master WHERE name='follower_spec_kit'").fetchone())

    def test_refuses_source_database_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run(self.command(ROOT / 'data/world.db', root / 'receipt.json', root / 'kit.sql'), capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('refusing', result.stderr.lower())


if __name__ == '__main__':
    unittest.main()
