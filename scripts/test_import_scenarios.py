"""Pinned local scenario catalog contract; never opens canonical world.db for writes."""
from pathlib import Path
import sqlite3
import unittest

from import_scenarios import export_tables, import_tables

SOURCE = Path('/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933')
TDB = Path('/home/osso/Projects/world-of-osso/game-server/data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql')


class ScenarioImportTests(unittest.TestCase):
    def test_local_gnomeregan_join_preserves_main_and_bonus_steps(self):
        tables = export_tables(SOURCE)
        with sqlite3.connect(':memory:') as db:
            import_tables(db, tables, TDB)
            self.assertEqual(db.execute('SELECT Name_lang, Type FROM scenario WHERE ID=649').fetchone(), ('Gnomeregan', 3))
            self.assertEqual(db.execute('SELECT scenario_A, scenario_H FROM scenario_map WHERE map=90 AND difficulty=1').fetchone(), (649, 649))
            main = db.execute('SELECT ID, CriteriatreeID, OrderIndex FROM scenario_step WHERE ScenarioID=649 AND (Flags & 1)=0 ORDER BY OrderIndex').fetchall()
            self.assertEqual(main, [(1176, 36986, 0)])
            self.assertEqual(db.execute('SELECT COUNT(*) FROM scenario_step WHERE ScenarioID=649 AND (Flags & 1)=1').fetchone()[0], 4)
            self.assertEqual(db.execute('SELECT Type, Asset FROM criteria WHERE ID=(SELECT CriteriaID FROM criteria_tree WHERE ID=36987)').fetchone(), (0, 7800))
            import_tables(db, tables, TDB)
            self.assertEqual(db.execute('SELECT COUNT(*) FROM scenario WHERE ID=649').fetchone()[0], 1)


if __name__ == '__main__':
    unittest.main()
