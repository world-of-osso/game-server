"""Golden geometry from the actual TDB dump, through the copy-only producer."""
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

SERVER = Path('/home/osso/Projects/world-of-osso/game-server')
SOURCE = SERVER / 'data/world.db'
DUMP = SERVER / 'data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql'
SCRIPT = Path(__file__).resolve().parents[1] / 'encounter_model_geometry.py'


class EncounterModelGeometryTests(unittest.TestCase):
    def test_authentic_rows_are_admitted_idempotently_without_template_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / 'geometry.sqlite'
            with closing(sqlite3.connect(SOURCE.as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(copy)) as target:
                    source.backup(target)
            with closing(sqlite3.connect(copy)) as target:
                templates_before = target.execute(
                    'SELECT * FROM content_creature_template WHERE entry IN (135322,135406)'
                ).fetchall()
            for _ in range(2):
                result = subprocess.run(
                    [sys.executable, str(SCRIPT), '--db', str(copy), '--source-dump', str(DUMP)],
                    text=True, capture_output=True, check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                with closing(sqlite3.connect(copy)) as target:
                    tables = target.execute(
                        "SELECT count(*) FROM sqlite_master WHERE name='encounter_creature_model_info'"
                    ).fetchone()[0]
                    self.assertEqual(tables, 1, 'source model geometry was not admitted')
                    rows = target.execute(
                        'SELECT DisplayID,BoundingRadius,CombatReach,DisplayID_Other_Gender,VerifiedBuild '
                        'FROM encounter_creature_model_info ORDER BY DisplayID'
                    ).fetchall()
                    self.assertEqual(rows, [
                        (11686, 0.5, 1.0, 0, 66527),
                        (84202, 6.90508, 5.0, 0, 61967),
                        (88651, 0.31, 1.0, 0, 65727),
                    ])
                    self.assertEqual(target.execute(
                        'SELECT * FROM content_creature_template WHERE entry IN (135322,135406)'
                    ).fetchall(), templates_before)

    def test_canonical_database_is_refused_before_any_write(self):
        before = SOURCE.stat()
        result = subprocess.run(
            [sys.executable, str(SCRIPT), '--db', str(SOURCE), '--source-dump', str(DUMP)],
            text=True, capture_output=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('independent', result.stderr)
        self.assertEqual(SOURCE.stat().st_mtime_ns, before.st_mtime_ns)
        self.assertEqual(SOURCE.stat().st_size, before.st_size)


if __name__ == '__main__':
    unittest.main()
