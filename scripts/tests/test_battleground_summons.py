"""Source-to-runtime Silvershard preparation on disposable databases only."""
from contextlib import closing
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / 'scripts/add_battleground_summon_templates.py'


class BattlegroundSummonTests(unittest.TestCase):
    def prepare(self, path):
        shutil.copyfile(ROOT / 'data/world.db', path)
        with closing(sqlite3.connect(path)) as conn, conn:
            conn.execute('CREATE TABLE IF NOT EXISTS tdb_npc_spellclick_spells (npc_entry INTEGER, spell_id INTEGER, cast_flags INTEGER, user_type INTEGER, PRIMARY KEY(npc_entry,spell_id))')
            conn.execute('INSERT OR REPLACE INTO tdb_npc_spellclick_spells VALUES (60283,124491,1,0)')

    def run_import(self, path):
        return subprocess.run([sys.executable, str(SCRIPT), '--db', str(path)], capture_output=True, text=True)

    def test_silvershard_templates_models_paths_and_spellclick_are_authentic_and_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'prepared.sqlite'
            self.prepare(path)
            for _ in range(2):
                result = self.run_import(path)
                self.assertEqual(result.returncode, 0, result.stderr)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute('SELECT entry,name,speed_walk,npcflag FROM content_creature_template WHERE entry IN (60140,60283) ORDER BY entry').fetchall(),
                                 [(60140,'Mine Cart',0.8,0),(60283,'Track Switch',0.4,16777216)])
                for entry in (60140,60283):
                    self.assertEqual(conn.execute('SELECT * FROM content_creature_template_model WHERE CreatureID=?',(entry,)).fetchall(),
                                     conn.execute('SELECT * FROM tdb_creature_template_model WHERE CreatureID=?',(entry,)).fetchall())
                source = conn.execute('SELECT n.PathId,n.NodeId,n.PositionX,n.PositionY,n.PositionZ,p.MoveType FROM tdb_waypoint_path_node n JOIN tdb_waypoint_path p ON p.PathId=n.PathId WHERE n.PathId BETWEEN 6014000 AND 6014011 ORDER BY n.PathId,n.NodeId').fetchall()
                self.assertTrue(source)
                self.assertEqual(conn.execute('SELECT id,point,position_x,position_y,position_z,move_type FROM content_waypoint_data WHERE id BETWEEN 6014000 AND 6014011 ORDER BY id,point').fetchall(),source)
                self.assertEqual(conn.execute('SELECT * FROM content_npc_spellclick_spells WHERE npc_entry=60283').fetchall(),[(60283,124491,1,0)])

    def test_missing_model_aborts_transaction_without_partial_templates(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'prepared.sqlite'
            self.prepare(path)
            with closing(sqlite3.connect(path)) as conn, conn:
                conn.execute('DELETE FROM content_creature_template WHERE entry IN (60140,60283)')
                conn.execute('DELETE FROM tdb_creature_template_model WHERE CreatureID=60140')
            result = self.run_import(path)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('60140',result.stderr)
            with closing(sqlite3.connect(path)) as conn:
                self.assertEqual(conn.execute('SELECT count(*) FROM content_creature_template WHERE entry IN (60140,60283)').fetchone()[0],0)


if __name__ == '__main__':
    unittest.main()
