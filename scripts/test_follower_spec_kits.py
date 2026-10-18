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
    def command(self, output, receipt, diff, world_db=None, source=None):
        return [sys.executable, str(ROOT / 'scripts/follower_spec_kits.py'),
                '--world-db', str(world_db or ROOT / 'data/world.db'), '--db2-root', str(DB2),
                '--source-json', str(source or SOURCE), '--output-db', str(output),
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

    def test_rebuilds_existing_kit_only_in_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / 'first.db'
            result = subprocess.run(self.command(first, root / 'first.json', root / 'first.sql'), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            source = json.loads(SOURCE.read_text())
            source['world_db'] = str(first)
            source_path = root / 'source.json'
            source_path.write_text(json.dumps(source))
            second = root / 'second.db'
            result = subprocess.run(self.command(second, root / 'second.json', root / 'second.sql', first, source_path), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(first) as before, sqlite3.connect(second) as after:
                self.assertEqual(before.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall(), after.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall())
                self.assertEqual(before.execute('SELECT * FROM follower_profiles ORDER BY entry').fetchall(), after.execute('SELECT * FROM follower_profiles ORDER BY entry').fetchall())
            self.assertEqual((root / 'second.sql').read_text(), '')
            self.assertEqual(json.loads((root / 'second.json').read_text())['changed_tables'], [])

    def test_refuses_source_database_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run(self.command(ROOT / 'data/world.db', root / 'receipt.json', root / 'kit.sql'), capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('refusing', result.stderr.lower())


if __name__ == '__main__':
    unittest.main()
