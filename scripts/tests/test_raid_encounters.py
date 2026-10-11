import os
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from raid_encounters import prepare


class RaidEncountersTest(unittest.TestCase):
    def test_authentic_single_boss_join_and_ambiguous_displays_are_not_bound(self):
        root = Path(os.environ['RAID_EVIDENCE'])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'raid.sql'
            report = Path(directory) / 'report.json'
            bindings, gaps = prepare(Path(os.environ['RAID_WORLD_DB']),
                                     root / 'db2/1240336.db2', root / 'db2/1301155.db2',
                                     output, report)
            self.assertIn((409, 12118, 663, 0, 1519, 3733, 13031), bindings)
            self.assertFalse(any(row[0] == 409 and row[2] in (665, 669, 671) for row in bindings))
            self.assertTrue(any(row['encounter_id'] == 665 for row in gaps))
            with closing(sqlite3.connect(':memory:')) as conn:
                conn.executescript(output.read_text())
                self.assertEqual(conn.execute('SELECT creature_entry,encounter_id,bit FROM raid_encounter_creature WHERE map_id=409 AND creature_entry=12118').fetchone(), (12118, 663, 0))
                conn.executescript(output.read_text())
                self.assertEqual(conn.execute('SELECT count(*) FROM raid_encounter_creature').fetchone()[0], len(bindings))


if __name__ == '__main__':
    unittest.main()
