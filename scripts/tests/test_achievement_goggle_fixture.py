"""Goggle process fixture follows active provenance and preserves exact source items."""
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'crates/server/tests/fixtures/prepare_achievement_goggle_db.py'


class GoggleFixtureTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / 'source.sqlite'
        self.output = self.root / 'goggle.sqlite'
        self.csv_root = self.root / 'db2'
        self.csv_root.mkdir()
        canonical = Path(os.environ.get('GAME_SERVER_DB_PATH', ROOT / 'data/world.db'))
        with sqlite3.connect(self.source, uri=True) as connection:
            connection.execute('ATTACH DATABASE ? AS retail', (canonical.resolve().as_uri() + '?mode=ro',))
            connection.execute('CREATE TABLE item AS SELECT * FROM retail.item WHERE ID BETWEEN 77533 AND 77539')
            connection.execute('CREATE TABLE item_sparse AS SELECT * FROM retail.item_sparse WHERE ID BETWEEN 77533 AND 77539')
            connection.execute('CREATE TABLE item_modified_appearance AS SELECT * FROM retail.item_modified_appearance WHERE ItemID BETWEEN 77533 AND 77539')
            connection.execute('CREATE TABLE item_appearance AS SELECT * FROM retail.item_appearance WHERE ID IN (SELECT ItemAppearanceID FROM item_modified_appearance)')
            connection.execute('CREATE TABLE content_item AS SELECT * FROM retail.content_item WHERE 0')
            connection.execute('CREATE TABLE db2_build(table_name TEXT PRIMARY KEY, build TEXT)')
            connection.execute('CREATE TABLE unrelated(ID INTEGER, name TEXT)')
            connection.execute("INSERT INTO unrelated VALUES(7, 'preserved')")
            self.expected = connection.execute(
                'SELECT s.ID,s.Display_lang,s.OverallQualityID,s.ItemLevel,i.ClassID,i.SubclassID,NULL '
                'FROM item_sparse s JOIN item i ON i.ID=s.ID ORDER BY s.ID'
            ).fetchall()
        self.assertEqual([row[0] for row in self.expected], list(range(77533, 77540)))

    def select_source(self, active, item=None, sparse=None):
        directory = self.csv_root / active
        directory.mkdir(exist_ok=True)
        (directory / 'receipt.json').write_text(json.dumps({'identity': {'product': 'wow', 'build': active}}))
        link = self.csv_root / 'active'
        link.unlink(missing_ok=True)
        link.symlink_to(active)
        with sqlite3.connect(self.source) as connection:
            connection.execute('DELETE FROM db2_build')
            connection.executemany('INSERT INTO db2_build VALUES(?,?)',
                                   [('item', item or active), ('item_sparse', sparse or active)])

    def run_fixture(self):
        return subprocess.run([sys.executable, str(SCRIPT), str(self.source), str(self.output)],
                              env={**os.environ, 'DB2_CSV_ROOT': str(self.csv_root)},
                              capture_output=True, text=True)

    def test_active_and_next_build_project_all_seven_real_items_without_touching_source(self):
        for build in ['12.1.5.70448', '12.1.6.71000']:
            with self.subTest(build=build):
                self.select_source(build)
                self.output.unlink(missing_ok=True)
                before = hashlib.sha256(self.source.read_bytes()).hexdigest()
                result = self.run_fixture()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(hashlib.sha256(self.source.read_bytes()).hexdigest(), before)
                with sqlite3.connect(self.output) as connection:
                    actual = connection.execute('SELECT ID,Name,Quality,ItemLevel,ClassID,SubclassID,SourceEntry FROM content_item ORDER BY ID').fetchall()
                    self.assertEqual(actual, self.expected)
                    self.assertEqual(connection.execute('SELECT * FROM unrelated').fetchall(), [(7, 'preserved')])

    def test_stale_and_mixed_source_receipts_are_rejected_before_output(self):
        for item, sparse in [('12.1.5.70391', '12.1.5.70391'), ('12.1.5.70448', '12.1.5.70391')]:
            with self.subTest(item=item, sparse=sparse):
                self.select_source('12.1.5.70448', item, sparse)
                self.output.unlink(missing_ok=True)
                result = self.run_fixture()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('goggle source build mismatch', result.stderr)
                self.assertFalse(self.output.exists())

    def test_missing_real_goggle_item_still_refuses_projection(self):
        self.select_source('12.1.5.70448')
        with sqlite3.connect(self.source) as connection:
            connection.execute('DELETE FROM item_sparse WHERE ID=77539')
        result = self.run_fixture()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('missing sourced goggle item 77539', result.stderr)


if __name__ == '__main__':
    unittest.main()
