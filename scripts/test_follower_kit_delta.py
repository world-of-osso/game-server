"""Scoped kit publication must preserve unrelated specs and ignore physical rowids."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest


PRODUCER = Path(__file__).with_name('follower_kit_delta.py')
COLUMNS = 'spec_id,spell_id,min_level,max_level,role_slot,priority,conditions,policy'
SCHEMA = f'CREATE TABLE follower_spec_kit ({COLUMNS}, PRIMARY KEY(spec_id,spell_id,min_level))'


def write_database(path, rows):
    with sqlite3.connect(path) as db:
        db.execute(SCHEMA)
        db.executemany('INSERT INTO follower_spec_kit VALUES (?,?,?,?,?,?,?,?)', rows)
        db.execute('CREATE TABLE preserved (value TEXT)')
        db.execute("INSERT INTO preserved VALUES ('native source metadata')")


def read_rows(path):
    with sqlite3.connect(path) as db:
        return db.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall()


class FollowerKitDeltaTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / 'canonical.db'
        self.prepared = self.root / 'prepared.db'
        self.output = self.root / 'patched.db'
        self.sql = self.root / 'delta.sql'
        self.receipt = self.root / 'receipt.json'
        self.original = [(spec, spec + 10, 1, 90, 'attack', 100, '{}', 'native')
                         for spec in (63, 66, 253, 262, 1467)]
        self.desired = [(*row[:5], 84 if row[0] == 262 else 99, *row[6:])
                        for row in self.original]
        write_database(self.source, self.original)
        write_database(self.prepared, self.desired)

    def produce(self):
        return subprocess.run([
            sys.executable, str(PRODUCER), '--world-db', str(self.source),
            '--prepared-db', str(self.prepared), '--output-db', str(self.output),
            '--sql', str(self.sql), '--receipt', str(self.receipt),
            '--specs', '63', '253', '262', '1467',
        ], capture_output=True, text=True, check=False)

    def test_delta_replays_by_native_keys_and_preserves_other_specs_and_tables(self):
        before = self.source.read_bytes()
        result = self.produce()
        self.assertEqual(result.returncode, 0, result.stderr)
        expected = [old if old[0] == 66 else new
                    for old, new in zip(self.original, self.desired)]
        self.assertEqual(read_rows(self.output), expected)
        with sqlite3.connect(self.output) as db:
            self.assertEqual(db.execute('SELECT value FROM preserved').fetchone()[0],
                             'native source metadata')
        replay = self.root / 'reordered.db'
        write_database(replay, list(reversed(self.original)))
        with sqlite3.connect(replay) as db:
            db.executescript(self.sql.read_text())
            db.executescript(self.sql.read_text())
        self.assertEqual(read_rows(replay), expected)
        self.assertEqual(self.source.read_bytes(), before)
        receipt = json.loads(self.receipt.read_text())
        self.assertEqual(receipt['specs'], [63, 253, 262, 1467])
        self.assertEqual(receipt['changed_specs'], [63, 253, 262, 1467])
        self.assertEqual(receipt['sql_sha256'], hashlib.sha256(self.sql.read_bytes()).hexdigest())

    def test_missing_interval_is_rejected_before_any_output_is_published(self):
        with sqlite3.connect(self.prepared) as db:
            db.execute('DELETE FROM follower_spec_kit WHERE spec_id=262')
        result = self.produce()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('kit interval keys differ', result.stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sql.exists())
        self.assertFalse(self.receipt.exists())

    def test_existing_output_is_not_overwritten(self):
        self.output.write_bytes(b'keep existing artifact')
        result = self.produce()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('refusing existing', result.stderr)
        self.assertEqual(self.output.read_bytes(), b'keep existing artifact')


if __name__ == '__main__':
    unittest.main()
