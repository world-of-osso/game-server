"""Real TDB/world-copy regression. Set DUNGEON_POPULATION_SOURCE to run."""
import os
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


class InstancePopulationTest(unittest.TestCase):
    def test_destination_named_world_db_is_refused(self):
        import import_instance_population as importer
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.db"
            path.touch()
            with self.assertRaisesRegex(ValueError, "world.db"):
                importer.import_database(path)

    @unittest.skipUnless(os.environ.get("DUNGEON_POPULATION_SOURCE"), "requires authentic world.db")
    def test_all_instance_maps_match_eligible_tdb_and_keep_world_content(self):
        import import_instance_population as importer
        source = Path(os.environ["DUNGEON_POPULATION_SOURCE"])
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "instances.sqlite"
            with sqlite3.connect(f"file:{source}?mode=ro", uri=True) as original:
                with sqlite3.connect(destination) as copy:
                    original.backup(copy)
            with sqlite3.connect(destination) as conn:
                before = conn.execute("SELECT * FROM content_creature WHERE map=2991 ORDER BY guid").fetchall()
            importer.import_database(destination)
            with sqlite3.connect(destination) as conn:
                self.assertEqual(before, conn.execute("SELECT * FROM content_creature WHERE map=2991 ORDER BY guid").fetchall())
                for row in importer.population_counts(conn):
                    with self.subTest(map=row["map"]):
                        self.assertEqual(row["eligible_creatures"], row["after_creatures"])
                        self.assertEqual(row["eligible_gameobjects"], row["after_gameobjects"])
                self.assertEqual(421, conn.execute("SELECT count(*) FROM content_creature WHERE map=36").fetchone()[0])
                self.assertEqual(576, conn.execute("SELECT count(*) FROM content_creature WHERE map=643").fetchone()[0])
                guid, entry, display = conn.execute("SELECT guid,id1,modelid FROM content_creature WHERE map=36 ORDER BY guid LIMIT 1").fetchone()
                self.assertEqual(conn.execute("SELECT id,modelid FROM tdb_creature WHERE guid=?", (guid-20000000,)).fetchone(), (entry, display))
                # Idempotence includes addon/formation primary keys, not just spawn counts.
            importer.import_database(destination)
            with sqlite3.connect(destination) as conn:
                self.assertEqual(421, conn.execute("SELECT count(*) FROM content_creature WHERE map=36").fetchone()[0])


if __name__ == "__main__":
    unittest.main()
