"""The Darkness moves from its Journal placement to the logged WCL fight position."""
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'darkness_placement.py'


class DarknessPlacement(unittest.TestCase):
    def test_moves_the_single_journal_placement_to_the_logged_room_floor(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'copy.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE content_creature(guid INTEGER PRIMARY KEY, id1 INTEGER, map INTEGER,'
                           'position_x REAL, position_y REAL, position_z REAL, orientation REAL)')
                db.executemany('INSERT INTO content_creature VALUES(?,?,?,?,?,?,?)', [
                    (63651001, 208747, 2651, 188.47888114999995, -547.817170475, 59.8, 0.0),
                    (63651000, 208743, 2651, 406.77750715, -155.2714289, 56.68354978731017, 0.0),
                ])
            subprocess.run([sys.executable, '-I', str(SCRIPT), '--db', str(path)], check=True,
                           capture_output=True)
            with closing(sqlite3.connect(path)) as db:
                rows = db.execute('SELECT guid,position_x,position_y,position_z,orientation '
                                  'FROM content_creature ORDER BY guid').fetchall()
        self.assertEqual(rows[0], (63651000, 406.77750715, -155.2714289, 56.68354978731017, 0.0))
        guid, x, y, z, orientation = rows[1]
        self.assertEqual(guid, 63651001)
        self.assertAlmostEqual(x, 391.93)
        self.assertAlmostEqual(y, 27.99)
        self.assertAlmostEqual(z, 87.26754)
        self.assertAlmostEqual(orientation, 5.106370614359172)

    def test_refuses_canonical_world_db_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'world.db'
            path.touch()
            result = subprocess.run([sys.executable, '-I', str(SCRIPT), '--db', str(path)],
                                    capture_output=True)
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
