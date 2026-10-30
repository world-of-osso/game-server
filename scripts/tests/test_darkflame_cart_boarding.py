"""The inferred cart point uses node0 and retains explicit provenance."""
from pathlib import Path
import sqlite3
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import darkflame_cart_boarding as cart


class DarkflameCartBoarding(unittest.TestCase):
    def test_point_uses_path_node_and_records_inference(self):
        db = sqlite3.connect(':memory:')
        db.executescript('''
            CREATE TABLE taxi_path_node (PathID INTEGER, NodeIndex INTEGER, Loc_0 REAL, Loc_1 REAL, Loc_2 REAL);
            CREATE TABLE content_creature_template (entry INTEGER, npcflag INTEGER);
            CREATE TABLE content_creature (guid INTEGER, id1 INTEGER, map INTEGER, zoneId INTEGER,
                areaId INTEGER, spawnMask INTEGER, phaseMask INTEGER, position_x REAL, position_y REAL,
                position_z REAL, orientation REAL, spawntimesecs INTEGER, curhealth INTEGER,
                VerifiedBuild INTEGER, Comment TEXT, spawnDifficulties TEXT);
        ''')
        db.executemany('INSERT INTO taxi_path_node VALUES (?,?,?,?,?)', [
            (9458, i, 209.368057 + i, -608.100708, 62.039959) for i in range(32)])

        def import_source(connection, entry, script):
            self.assertEqual((entry, script), (209439, ''))
            connection.execute('INSERT INTO content_creature_template VALUES (?,0)', (entry,))

        with patch.object(cart, 'import_base_template', import_source):
            self.assertEqual(cart.add_cart(db), (209.368057, -608.100708, 62.039959))
            with self.assertRaisesRegex(ValueError, 'already exists'):
                cart.add_cart(db)
        self.assertEqual(db.execute('SELECT npcflag FROM content_creature_template WHERE entry=209439').fetchone(),
                         (0x01000000,))
        guid, verified, comment, difficulties = db.execute(
            'SELECT guid,VerifiedBuild,Comment,spawnDifficulties FROM content_creature').fetchone()
        self.assertEqual((guid, verified, difficulties), (63651999, 0, '1'))
        self.assertIn('DARKFLAME_CART_BOARDING_LOG_CORRELATED', comment)
        self.assertIn('exact boarding trigger and spellclick association unsourced', comment)
        db.close()


if __name__ == '__main__':
    unittest.main()
