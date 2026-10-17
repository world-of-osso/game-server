"""Private DB2 roots follow their active receipt in each import process."""
import csv
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import db2


class ActiveDb2Tests(unittest.TestCase):
    def test_private_root_does_not_read_canonical_csvs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / 'next-export'
            snapshot.mkdir()
            (snapshot / 'receipt.json').write_text(json.dumps({'identity': {'build': snapshot.name}}))
            (root / 'active').symlink_to(snapshot.name)
            result = subprocess.run(
                [sys.executable, str(SCRIPTS / 'db2.py'), 'check'],
                env={**os.environ, 'DB2_CSV_ROOT': str(root)}, capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(f'Missing CSVs in {snapshot}', result.stderr)

    def test_import_follows_private_active_switch_and_preserves_non_db2_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            database = root / 'world.db'
            with sqlite3.connect(database) as connection:
                connection.executescript(db2.SCHEMA_SQL.read_text())
                connection.execute('CREATE TABLE unrelated (id INTEGER, name TEXT)')
                connection.execute("INSERT INTO unrelated VALUES (7, 'preserved')")
                headers = {table: [row[1] for row in connection.execute(
                    f'PRAGMA table_info({db2.table_name(table)})')] for table in db2.TABLES}
            for build, name in [('next-export', 'First spell'), ('later-export', 'Second spell')]:
                snapshot = root / build
                snapshot.mkdir()
                for table, header in headers.items():
                    with (snapshot / f'{table}.csv').open('w', newline='') as stream:
                        writer = csv.writer(stream)
                        writer.writerow(header)
                        if table == 'SpellName':
                            writer.writerow([133 if column == 'ID' else name for column in header])
                (snapshot / 'receipt.json').write_text(json.dumps({'identity': {'build': build}}))
                (root / 'active').unlink(missing_ok=True)
                (root / 'active').symlink_to(build)
                result = subprocess.run(
                    [sys.executable, str(SCRIPTS / 'db2.py'), 'import', '--db', str(database)],
                    env={**os.environ, 'DB2_CSV_ROOT': str(root)}, capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                with sqlite3.connect(database) as connection:
                    self.assertEqual(connection.execute('SELECT Name_lang FROM spell_name WHERE ID=133').fetchone(), (name,))
                    self.assertEqual(connection.execute('SELECT DISTINCT build FROM db2_build').fetchall(), [(build,)])
                    self.assertEqual(connection.execute('SELECT * FROM unrelated').fetchall(), [(7, 'preserved')])


if __name__ == '__main__':
    unittest.main()
