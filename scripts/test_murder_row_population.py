import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest

from murder_row_population import BINDINGS, ENTRANCE, LITHIEL_PRE_PULL, prepare


class MurderRowPopulationTest(unittest.TestCase):
    def test_prepares_real_source_rows_on_copy_and_is_idempotent(self):
        source = Path(os.environ['MURDER_ROW_SOURCE_DB'])
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'source-copy.sqlite'
            with sqlite3.connect(f'file:{source}?mode=ro', uri=True) as original:
                with sqlite3.connect(target) as copy:
                    original.backup(copy)
            prepare(target)
            prepare(target)
            with sqlite3.connect(target) as db:
                self.assertEqual(db.execute('SELECT * FROM tdb_world_safe_locs WHERE ID=81002813').fetchone(), ENTRANCE)
                bindings = db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2813 AND creature_entry IN (234647,237415) ORDER BY creature_entry').fetchall()
                self.assertEqual(bindings, BINDINGS)
                spawn = db.execute('SELECT position_x,position_y,position_z,orientation,spawnDifficulties FROM content_creature WHERE map=2813 AND id1=234647').fetchall()
                self.assertEqual(spawn, [(8628.92, -5039.0, 126.20164, 0.823, '1')])
                self.assertEqual(db.execute('SELECT name FROM content_creature_template WHERE entry=234647').fetchone(), ('Xathuux the Annihilator',))
                self.assertEqual(db.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=234647').fetchone(), (140268,))
                lithiel = db.execute('SELECT guid,position_x,position_y,position_z,orientation FROM content_creature WHERE map=2813 AND id1=237415').fetchall()
                self.assertEqual(len(lithiel), 1)
                self.assertEqual(lithiel[0][1:], LITHIEL_PRE_PULL)
                provenance = json.loads(db.execute('SELECT observation_json FROM log_derived_population WHERE guid=?', (lithiel[0][0],)).fetchone()[0])
                self.assertEqual(provenance['position_method'], 'last_pre_encounter_start')
                self.assertEqual(provenance['observation_time'], '10/7/2026 23:06:53.269-5')
                self.assertEqual(db.execute('PRAGMA integrity_check').fetchone(), ('ok',))


if __name__ == '__main__':
    unittest.main()
