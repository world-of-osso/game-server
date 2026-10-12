"""Authentic copy/process acceptance for zone services and quest promotion."""
from contextlib import closing
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
SOURCE = os.environ.get("WORLD_POPULATION_SOURCE")


@unittest.skipUnless(SOURCE, "requires authentic read-only world.db")
class ZonePopulationTest(unittest.TestCase):
    def test_missing_zone_quests_services_and_spawns_are_source_backed_and_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "population.sqlite"
            report = Path(directory) / "report.json"
            with closing(sqlite3.connect(f"file:{SOURCE}?mode=ro", uri=True)) as source:
                with closing(sqlite3.connect(path)) as copy:
                    source.backup(copy)
            command = [sys.executable, str(SCRIPTS / "content_policy.py"), "--mode", "world-promote",
                       "--db", str(path), "--maps", "530,571,646,1116,1643,2552", "--report", str(report)]
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                for quest, giver in [(36626, 86065), (48677, 127296), (82747, 224544)]:
                    source_title = db.execute("SELECT LogTitle FROM tdb_quest_template WHERE ID=?", (quest,)).fetchone()
                    self.assertEqual(source_title, db.execute("SELECT LogTitle FROM content_quest_template WHERE ID=?", (quest,)).fetchone())
                    self.assertEqual([(giver, quest)], db.execute("SELECT id,quest FROM content_creature_queststarter WHERE id=? AND quest=?", (giver, quest)).fetchall())
                    self.assertTrue(db.execute("SELECT 1 FROM content_creature WHERE id1=?", (giver,)).fetchone())
                self.assertEqual(0, db.execute("SELECT count(*) FROM content_creature c JOIN creature a ON a.guid=c.guid AND a.map=c.map AND a.id1=c.id1 WHERE c.map IN(530,571)").fetchone()[0])
                self.assertEqual(db.execute("SELECT * FROM tdb_creature_template_model WHERE CreatureID=139612").fetchall(), db.execute("SELECT * FROM content_creature_template_model WHERE CreatureID=139612").fetchall())
                source_rows = db.execute("SELECT guid+20000000,id,position_x,position_y,position_z FROM tdb_creature WHERE map=646 AND phaseUseFlags=0 AND PhaseId IN(0,169) AND PhaseGroup=0 AND terrainSwapMap=-1 AND instr(','||replace(spawnDifficulties,' ','')||',',',0,')>0").fetchall()
                self.assertGreater(len(source_rows), 5000)
                self.assertEqual(sorted(source_rows), sorted(db.execute("SELECT guid,id1,position_x,position_y,position_z FROM content_creature WHERE map=646").fetchall()))
                # A real Drustvar vendor whose item list was absent from the old map-only promotion.
                entries = "SELECT DISTINCT id1 FROM content_creature WHERE map=1643"
                expected = db.execute(f"SELECT entry,slot,item FROM tdb_npc_vendor WHERE entry IN({entries}) AND type=1 AND item IN(SELECT ID FROM content_item)").fetchall()
                self.assertGreater(len(expected), 0)
                for row in expected:
                    self.assertIn(row, db.execute("SELECT entry,slot,item FROM content_npc_vendor WHERE entry=?", (row[0],)).fetchall())
                tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'content_%'")]
                snapshot = {t: sorted(db.execute(f'SELECT * FROM "{t}"').fetchall(), key=repr) for t in tables}
            result = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(snapshot, {t: sorted(db.execute(f'SELECT * FROM "{t}"').fetchall(), key=repr) for t in tables})


if __name__ == "__main__":
    unittest.main()
