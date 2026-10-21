"""Real TDB1210/70448 producer tests; canonical source is read-only."""
import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("follower_population", SCRIPTS / "follower_population.py")
population = importlib.util.module_from_spec(spec)
spec.loader.exec_module(population)
SOURCE = (SCRIPTS.parent / "data/world.db").resolve()
DB2 = SOURCE.parent.parent / "db2/12.1.5.70448"


class FollowerPopulationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.copy = Path(self.temporary.name) / "population.sqlite"
        with closing(sqlite3.connect(SOURCE.as_uri() + "?mode=ro", uri=True)) as source, closing(sqlite3.connect(self.copy)) as target, target:
            for table in ("content_creature", "tdb_world_safe_locs", "follower_lfg_dungeons"):
                schema = source.execute("SELECT sql FROM sqlite_master WHERE name=?", (table,)).fetchone()[0]
                target.execute(schema)
            target.execute("CREATE TABLE content_creature_template(entry INTEGER PRIMARY KEY)")
            target.executemany("INSERT INTO content_creature_template VALUES(?)", source.execute("SELECT entry FROM content_creature_template"))
            rows = source.execute("SELECT * FROM follower_lfg_dungeons WHERE difficulty_id=205").fetchall()
            target.executemany(f"INSERT INTO follower_lfg_dungeons VALUES({','.join('?' for _ in rows[0])})", rows)

    def test_actual_source_projection_is_complete_exact_and_idempotent(self):
        report = population.apply_population(SOURCE, self.copy, DB2)
        self.assertEqual(len(report["maps"]), 27)
        self.assertEqual([row["map_id"] for row in report["maps"] if row["source_complete"]], [2451, 2515, 2521, 2526, 2648])
        with closing(sqlite3.connect(SOURCE.as_uri() + "?mode=ro", uri=True)) as source, closing(sqlite3.connect(self.copy)) as target, target:
            for row in report["maps"]:
                map_id = row["map_id"]
                expected = source.execute(
                    f"SELECT guid+{population.RETAIL_GUID_OFFSET},id,map,position_x,position_y,position_z,orientation,spawnDifficulties "
                    f"FROM tdb_creature WHERE map=? AND {population.NORMAL} AND {population.BASE_PHASE} ORDER BY guid", (map_id,),
                ).fetchall()
                actual = target.execute("SELECT guid,id1,map,position_x,position_y,position_z,orientation,spawnDifficulties FROM content_creature WHERE map=? ORDER BY guid", (map_id,)).fetchall()
                self.assertEqual(actual, expected, row["name"])
                expected_entrances = source.execute("SELECT * FROM tdb_world_safe_locs WHERE MapID=? AND Comment LIKE '%Entrance%' ORDER BY ID", (map_id,)).fetchall()
                self.assertEqual(target.execute("SELECT * FROM tdb_world_safe_locs WHERE MapID=? ORDER BY ID", (map_id,)).fetchall(), expected_entrances)
                self.assertEqual(bool(row["gaps"]), not row["source_complete"])
        self.assertEqual(population.apply_population(SOURCE, self.copy, DB2), report)

    def test_missing_template_rolls_back_all_population_and_entrances(self):
        with closing(sqlite3.connect(self.copy)) as target, target:
            target.execute("DELETE FROM content_creature_template")
        with self.assertRaisesRegex(ValueError, "prepare runtime template"):
            population.apply_population(SOURCE, self.copy, DB2)
        with closing(sqlite3.connect(self.copy)) as target:
            self.assertEqual(target.execute("SELECT count(*) FROM content_creature").fetchone()[0], 0)
            self.assertEqual(target.execute("SELECT count(*) FROM tdb_world_safe_locs").fetchone()[0], 0)

    def test_late_roster_mismatch_rolls_back_earlier_maps(self):
        with closing(sqlite3.connect(self.copy)) as target, target:
            target.execute("DELETE FROM follower_lfg_dungeons WHERE map_id=2993")
        with self.assertRaisesRegex(ValueError, "prepared follower roster mismatch"):
            population.apply_population(SOURCE, self.copy, DB2)
        with closing(sqlite3.connect(self.copy)) as target:
            self.assertEqual(target.execute("SELECT count(*) FROM content_creature").fetchone()[0], 0)
            self.assertEqual(target.execute("SELECT count(*) FROM tdb_world_safe_locs").fetchone()[0], 0)

    def test_source_cannot_be_a_destination(self):
        with self.assertRaisesRegex(ValueError, "world.db"):
            population.apply_population(SOURCE, SOURCE, DB2)
        alias = Path(self.temporary.name) / "alias.sqlite"
        alias.symlink_to(SOURCE)
        with self.assertRaises(ValueError):
            population.apply_population(SOURCE, alias, DB2)

    def test_authentication_hash_mismatch_never_writes_destination(self):
        broken = Path(self.temporary.name) / "db2"
        broken.mkdir()
        (broken / "receipt.json").write_bytes((DB2 / "receipt.json").read_bytes())
        (broken / "Map.csv").write_text("ID,InstanceType\n")
        with self.assertRaisesRegex(ValueError, "Map authentication/hash mismatch"):
            population.apply_population(SOURCE, self.copy, broken)
        with closing(sqlite3.connect(self.copy)) as target:
            self.assertEqual(target.execute("SELECT count(*) FROM content_creature").fetchone()[0], 0)


if __name__ == "__main__":
    unittest.main()
