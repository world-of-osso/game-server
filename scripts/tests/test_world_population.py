"""Scoped process/copy regression against authentic TDB1210 reference rows."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing

SCRIPTS = Path(__file__).resolve().parents[1]
SOURCE = os.environ.get("WORLD_POPULATION_SOURCE")


@unittest.skipUnless(SOURCE, "requires authentic read-only world.db")
class WorldPopulationTest(unittest.TestCase):
    def run_import(self, path, report, maps="2444"):
        return subprocess.run(
            [sys.executable, str(SCRIPTS / "content_policy.py"), "--mode", "world-promote",
             "--db", str(path), "--maps", maps, "--report", str(report)],
            capture_output=True, text=True,
        )

    def test_dragon_isles_source_population_preserves_other_maps_and_overlays(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "population.sqlite"
            report = Path(directory) / "report.json"
            with closing(sqlite3.connect(f"file:{SOURCE}?mode=ro", uri=True)) as source:
                with closing(sqlite3.connect(path)) as copy:
                    source.backup(copy)
            with closing(sqlite3.connect(path)) as db:
                # A preexisting unrelated authored pool must survive the shared builder.
                db.execute("INSERT INTO content_pool_template VALUES (987654321,1,'private overlay')")
                db.execute("INSERT INTO content_pool_members VALUES (2,987654321,987654321,0,'private overlay')")
                # An unrelated authored placement on the same promoted map is not
                # owned by the TDB GUID namespace and must survive with its addon.
                row = list(db.execute("SELECT * FROM content_creature WHERE map=0 LIMIT 1").fetchone())
                columns = [r[1] for r in db.execute("PRAGMA table_info(content_creature)")]
                row[columns.index("guid")] = 987654320
                row[columns.index("map")] = 2444
                db.execute(f"INSERT INTO content_creature VALUES ({','.join('?' for _ in row)})", row)
                overlay = tuple(row)
                object_row = list(db.execute("SELECT * FROM content_gameobject WHERE map=0 LIMIT 1").fetchone())
                object_columns = [r[1] for r in db.execute("PRAGMA table_info(content_gameobject)")]
                object_row[object_columns.index("guid")] = 987654319
                object_row[object_columns.index("map")] = 2444
                db.execute(f"INSERT INTO content_gameobject VALUES ({','.join('?' for _ in object_row)})", object_row)
                addon = list(db.execute("SELECT * FROM content_creature_addon LIMIT 1").fetchone())
                addon[0] = 987654320
                db.execute(f"INSERT INTO content_creature_addon VALUES ({','.join('?' for _ in addon)})", addon)
                db.commit()
                before = {t: db.execute(f"SELECT * FROM {t} WHERE map<>2444 ORDER BY guid").fetchall()
                          for t in ("content_creature", "content_gameobject")}
                protected_entries = "SELECT id1 FROM content_creature WHERE map<>2444"
                templates = db.execute(f"SELECT * FROM content_creature_template WHERE entry IN ({protected_entries}) ORDER BY entry").fetchall()
                expected = db.execute("SELECT guid+20000000,id,map,zoneId,areaId,position_x,position_y,position_z,orientation FROM tdb_creature WHERE map=2444 AND phaseUseFlags=0 AND PhaseId IN (0,169) AND PhaseGroup=0 AND terrainSwapMap=-1 AND instr(','||replace(spawnDifficulties,' ','')||',',',0,')>0 AND id IN (SELECT entry FROM tdb_creature_template)").fetchall()
                self.assertGreater(len(expected), 2500)
            result = self.run_import(path, report)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                actual = db.execute("SELECT guid,id1,map,zoneId,areaId,position_x,position_y,position_z,orientation FROM content_creature WHERE map=2444 AND guid<>987654320").fetchall()
                self.assertEqual(sorted(expected), sorted(actual))
                self.assertEqual(overlay, db.execute("SELECT * FROM content_creature WHERE guid=987654320").fetchone())
                self.assertEqual(tuple(addon), db.execute("SELECT * FROM content_creature_addon WHERE guid=987654320").fetchone())
                self.assertEqual(tuple(object_row), db.execute("SELECT * FROM content_gameobject WHERE guid=987654319").fetchone())
                for table, rows in before.items():
                    self.assertEqual(rows, db.execute(f"SELECT * FROM {table} WHERE map<>2444 ORDER BY guid").fetchall())
                self.assertEqual(templates, db.execute(f"SELECT * FROM content_creature_template WHERE entry IN ({protected_entries}) ORDER BY entry").fetchall())
                self.assertEqual([(987654321,1,'private overlay')], db.execute("SELECT * FROM content_pool_template WHERE entry=987654321").fetchall())
                self.assertEqual([(2,987654321,987654321,0,'private overlay')], db.execute("SELECT * FROM content_pool_members WHERE spawnId=987654321").fetchall())
                tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'content_%'")]
                snapshot = {t: sorted(db.execute(f'SELECT * FROM "{t}"').fetchall(), key=repr) for t in tables}
            result = self.run_import(path, report)
            self.assertEqual(0, result.returncode, result.stderr)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(snapshot, {t: sorted(db.execute(f'SELECT * FROM "{t}"').fetchall(), key=repr) for t in tables})
            self.assertEqual([2444], json.loads(report.read_text())["target_maps"])

    def test_instance_target_and_linked_or_canonical_destination_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "population.sqlite"
            report = Path(directory) / "report.json"
            with closing(sqlite3.connect(f"file:{SOURCE}?mode=ro", uri=True)) as source:
                with closing(sqlite3.connect(path)) as copy:
                    source.backup(copy)
            with closing(sqlite3.connect(path)) as db:
                original_count = db.execute("SELECT count(*) FROM content_creature WHERE map=2444").fetchone()[0]
            for destination, maps in [(path, "36"), (Path(directory) / "world.db", "2444")]:
                result = self.run_import(destination, report, maps)
                self.assertNotEqual(0, result.returncode)
            link = Path(directory) / "linked.sqlite"
            link.symlink_to(path)
            self.assertNotEqual(0, self.run_import(link, report).returncode)
            hard = Path(directory) / "hard.sqlite"
            os.link(path, hard)
            self.assertNotEqual(0, self.run_import(hard, report).returncode)
            with closing(sqlite3.connect(path)) as db:
                self.assertEqual(original_count, db.execute("SELECT count(*) FROM content_creature WHERE map=2444").fetchone()[0])


if __name__ == "__main__":
    unittest.main()
