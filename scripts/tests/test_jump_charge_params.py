"""Actual TDB jump parameters through the independent-copy producer CLI."""
from contextlib import closing
import hashlib
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

import mysql_dump

ROOT = Path(__file__).resolve().parents[2]
DUMP = ROOT / 'data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql'
TABLE = 'jump_charge_params'
TARGET = 'tdb_jump_charge_params'
COLUMNS = ['id', 'speed', 'treatSpeedAsMoveTimeSeconds', 'unlimitedSpeed',
           'minHeight', 'maxHeight', 'spellVisualId', 'progressCurveId',
           'parabolicCurveId', 'triggerSpellId']


def read_actual_source():
    statements = []
    rows = []
    active = False
    with DUMP.open(encoding='utf-8') as source:
        for statement in source:
            if statement.startswith(f'CREATE TABLE `{TABLE}`'):
                active = True
            if active:
                statements.append(statement)
                if statement.startswith(')'):
                    active = False
            if statement.startswith(f'INSERT INTO `{TABLE}`'):
                statements.append(statement)
                rows.extend(list(row) for _, row in mysql_dump.dump_rows(statement, TABLE, DUMP))
    return ''.join(statements), sorted(rows, key=lambda row: row[0])


def preserved(db):
    queries = [
        'SELECT * FROM content_creature_template ORDER BY entry',
        'SELECT * FROM content_creature_template_model ORDER BY CreatureID,Idx',
        'SELECT * FROM content_creature_template_difficulty ORDER BY entry,DifficultyID',
        'SELECT * FROM content_creature ORDER BY guid',
        'SELECT * FROM raid_encounter_creature ORDER BY creature_entry',
        "SELECT type,name,tbl_name,sql FROM sqlite_master WHERE tbl_name!='tdb_jump_charge_params' ORDER BY type,name",
    ]
    return [hashlib.sha256(repr(db.execute(query).fetchall()).encode()).hexdigest()
            for query in queries]


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


class JumpChargeParamsTests(unittest.TestCase):
    def test_real_copy_cli_preserves_every_source_row_null_and_unrelated_data(self):
        excerpt, expected = read_actual_source()
        self.assertTrue(expected, 'actual source table is a prerequisite')
        by_id = {row[0]: row for row in expected}
        self.assertEqual(by_id[713], [713, 1, 1, 0, None, None, None, None, None, None])
        self.assertEqual(by_id[714], [714, 1, 1, 0, None, None, None, None, None, None])
        fixtures = ROOT / 'target/encounter-producer-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            path = Path(directory) / 'jump-params.sqlite'
            source_uri = (ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro'
            with closing(sqlite3.connect(source_uri, uri=True)) as source:
                source.execute('VACUUM INTO ?', (str(path),))
            with closing(sqlite3.connect(path)) as db:
                before = preserved(db)
            argv = [sys.executable, str(ROOT / 'scripts/encounter_serverside_spells.py'),
                    '--encounter', 'telash-jumps', '--db', str(path), '--source-dump', str(DUMP)]
            for _ in range(2):
                result = subprocess.run(argv, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(path)) as db:
                    actual_columns = [column[1] for column in db.execute(f'PRAGMA table_info({TARGET})')]
                    self.assertEqual(actual_columns, COLUMNS)
                    actual = [list(row) for row in db.execute(f'SELECT * FROM {TARGET} ORDER BY id')]
                    self.assertEqual(actual, expected, 'all source rows and NULL values survive unchanged')
                    self.assertEqual(preserved(db), before)
            invalid = Path(directory) / 'duplicate-source.sql'
            inserts = ''.join(line for line in excerpt.splitlines(keepends=True)
                              if line.startswith(f'INSERT INTO `{TABLE}`'))
            invalid.write_text(excerpt + inserts)
            before_failure = digest(path)
            result = subprocess.run(argv[:-1] + [str(invalid)], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('duplicate source jump_charge_params identity', result.stderr)
            self.assertEqual(digest(path), before_failure, 'rejected source must not partially mutate the copy')
            with closing(sqlite3.connect(':memory:')) as fresh:
                fresh.executescript((ROOT / 'scripts/tdb_schema.sql').read_text())
                self.assertEqual([column[1] for column in fresh.execute(f'PRAGMA table_info({TARGET})')], COLUMNS)
                excerpt_path = Path(directory) / 'jump-charge-params.sql'
                excerpt_path.write_text(excerpt)
                self.assertEqual(mysql_dump.load_table(fresh, TABLE, excerpt_path, prefix='tdb_'), len(expected))
                self.assertEqual([list(row) for row in fresh.execute(f'SELECT * FROM {TARGET} ORDER BY id')], expected)


if __name__ == '__main__':
    unittest.main()
