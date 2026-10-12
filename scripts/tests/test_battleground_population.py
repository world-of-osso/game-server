"""Process-level promotion against a read-only authentic TDB/DB2 database."""
from contextlib import closing
import hashlib
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
SOURCE = os.environ.get("WORLD_POPULATION_SOURCE")
MAPS = (489, 566, 2107, 761, 726, 727, 998, 559)
ALLOWED = {
    "content_creature", "content_creature_template", "content_creature_template_model",
    "content_creature_template_addon", "content_creature_equip_template",
    "content_creature_addon", "content_creature_formations", "content_game_event_creature",
    "content_waypoint_data", "content_gameobject", "content_gameobject_template",
    "content_gameobject_template_addon",
}


def fingerprints(db, tables):
    result = {}
    for table in tables:
        digest = hashlib.sha256()
        columns = len(db.execute(f'PRAGMA table_info("{table}")').fetchall())
        order = ",".join(str(i) for i in range(1, columns + 1))
        for row in db.execute(f'SELECT * FROM "{table}" ORDER BY {order}'):
            digest.update((repr(row) + "\n").encode())
        result[table] = digest.hexdigest()
    return result


@unittest.skipUnless(SOURCE, "requires authentic read-only world.db")
class BattlegroundPopulationTest(unittest.TestCase):
    def copy_database(self, directory):
        path = Path(directory) / "population.sqlite"
        subprocess.run(["cp", "--reflink=auto", SOURCE, str(path)], check=True)
        return path

    def promote(self, path, maps):
        return subprocess.run(
            [sys.executable, str(SCRIPTS / "promote_battlegrounds.py"), "--db", str(path),
             "--maps", ",".join(map(str, maps))], capture_output=True, text=True)

    def test_authentic_spawns_preserve_other_tables_shared_templates_and_repeat(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.copy_database(directory)
            targets = ",".join(map(str, MAPS))
            with closing(sqlite3.connect(path)) as db:
                tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")]
                unrelated = fingerprints(db, set(tables) - ALLOWED)
                other_spawns = {t: db.execute(f"SELECT * FROM {t} WHERE map NOT IN ({targets}) ORDER BY guid").fetchall()
                                for t in ("content_creature", "content_gameobject")}
                shared = db.execute(f"SELECT DISTINCT id1 FROM content_creature WHERE map NOT IN ({targets})").fetchall()
                shared_ids = ",".join(str(row[0]) for row in shared)
                shared_templates = db.execute(f"SELECT * FROM content_creature_template WHERE entry IN ({shared_ids}) ORDER BY entry").fetchall()
                expected = db.execute(f"SELECT guid+20000000,id,map,position_x,position_y,position_z FROM tdb_creature WHERE map IN ({targets}) AND terrainSwapMap=-1 AND (phaseUseFlags=1 OR (phaseUseFlags=0 AND PhaseId IN (0,169) AND PhaseGroup=0)) AND instr(','||replace(spawnDifficulties,' ','')||',',',0,')>0 AND id IN (SELECT entry FROM tdb_creature_template)").fetchall()
                self.assertGreater(len(expected), 100)
            result = self.promote(path, MAPS)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(unrelated, fingerprints(db, set(tables) - ALLOWED))
                for table, rows in other_spawns.items():
                    self.assertEqual(rows, db.execute(f"SELECT * FROM {table} WHERE map NOT IN ({targets}) ORDER BY guid").fetchall())
                self.assertEqual(shared_templates, db.execute(f"SELECT * FROM content_creature_template WHERE entry IN ({shared_ids}) ORDER BY entry").fetchall())
                actual = db.execute(f"SELECT guid,id1,map,position_x,position_y,position_z FROM content_creature WHERE map IN ({targets}) AND guid IN (SELECT guid+20000000 FROM tdb_creature WHERE map IN ({targets}))").fetchall()
                self.assertEqual(sorted(expected), sorted(actual))
                self.assertEqual([("ok",)], db.execute("PRAGMA integrity_check").fetchall())
                promoted = fingerprints(db, ALLOWED)
            result = self.promote(path, MAPS)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(promoted, fingerprints(db, ALLOWED))
                self.assertEqual(unrelated, fingerprints(db, set(tables) - ALLOWED))

    def test_mixed_open_world_request_is_rejected_without_any_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = self.copy_database(directory)
            before = hashlib.sha256(path.read_bytes()).hexdigest()
            result = self.promote(path, (727, 0))
            self.assertNotEqual(0, result.returncode)
            self.assertIn("not a battleground or arena", result.stderr)
            self.assertEqual(before, hashlib.sha256(path.read_bytes()).hexdigest())


if __name__ == "__main__":
    unittest.main()
