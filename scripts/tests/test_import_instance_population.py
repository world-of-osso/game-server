"""Real TDB/world-copy regression. Set DUNGEON_POPULATION_SOURCE to run."""
import os
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
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

    def test_always_visible_authored_font_rows_are_base_phase_eligible(self):
        import import_instance_population as importer
        with closing(sqlite3.connect(":memory:")) as conn:
            conn.execute("CREATE TABLE spawn (guid INTEGER, PhaseId INTEGER, PhaseGroup INTEGER, terrainSwapMap INTEGER, phaseUseFlags INTEGER)")
            # TDB1210 Font of Power rows: maps 2515/2521, always-visible flag 1.
            conn.executemany("INSERT INTO spawn VALUES (?,0,0,-1,1)", [(9000002,), (9000008,)])
            actual = conn.execute(f"SELECT guid FROM spawn WHERE {importer.BASE_PHASE} ORDER BY guid").fetchall()
            self.assertEqual([(9000002,), (9000008,)], actual)

    def test_linked_destinations_are_refused_without_writing(self):
        import import_instance_population as importer
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.sqlite"
            source.write_bytes(b"untouched")
            symbolic = Path(directory) / "symbolic.sqlite"
            symbolic.symlink_to(source)
            hard = Path(directory) / "hard.sqlite"
            os.link(source, hard)
            for path in (symbolic, hard):
                with self.assertRaisesRegex(ValueError, "copy"):
                    importer.import_database(path)
            self.assertEqual(source.read_bytes(), b"untouched")

    @unittest.skipUnless(os.environ.get("DUNGEON_POPULATION_SOURCE"), "requires authentic world.db")
    def test_all_instance_maps_match_eligible_tdb_and_keep_world_content(self):
        import import_instance_population as importer
        source = Path(os.environ["DUNGEON_POPULATION_SOURCE"])
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "instances.sqlite"
            with closing(sqlite3.connect(f"file:{source}?mode=ro", uri=True)) as original:
                with closing(sqlite3.connect(destination)) as copy:
                    original.backup(copy)
            with closing(sqlite3.connect(destination)) as conn:
                outside = "map NOT IN (SELECT map FROM tdb_instance_template)"
                before = conn.execute(f"SELECT * FROM content_creature WHERE {outside} ORDER BY guid").fetchall()
                before_objects = conn.execute(f"SELECT * FROM content_gameobject WHERE {outside} ORDER BY guid").fetchall()
                before_loot = conn.execute("SELECT entry,lootid,mingold,maxgold FROM content_creature_template WHERE lootid>=20000000 ORDER BY entry").fetchall()
            importer.import_database(destination)
            with closing(sqlite3.connect(destination)) as conn:
                self.assertEqual(before, conn.execute(f"SELECT * FROM content_creature WHERE {outside} ORDER BY guid").fetchall())
                self.assertEqual(before_objects, conn.execute(f"SELECT * FROM content_gameobject WHERE {outside} ORDER BY guid").fetchall())
                self.assertEqual(before_loot, conn.execute("SELECT entry,lootid,mingold,maxgold FROM content_creature_template WHERE lootid>=20000000 ORDER BY entry").fetchall())
                entries = "SELECT DISTINCT id1 FROM content_creature WHERE map IN (SELECT map FROM tdb_instance_template)"
                self.assertEqual(conn.execute(f"SELECT entry,name FROM tdb_creature_template WHERE entry IN ({entries}) ORDER BY entry").fetchall(), conn.execute(f"SELECT entry,name FROM content_creature_template WHERE entry IN ({entries}) ORDER BY entry").fetchall())
                self.assertEqual(conn.execute(f"SELECT * FROM tdb_creature_template_model WHERE CreatureID IN ({entries}) ORDER BY CreatureID,Idx").fetchall(), conn.execute(f"SELECT * FROM content_creature_template_model WHERE CreatureID IN ({entries}) ORDER BY CreatureID,Idx").fetchall())
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
            with closing(sqlite3.connect(destination)) as conn:
                self.assertEqual(421, conn.execute("SELECT count(*) FROM content_creature WHERE map=36").fetchone()[0])
                before_failed_import = conn.execute("SELECT * FROM content_creature WHERE map=36 ORDER BY guid").fetchall()
                conn.execute("DROP TABLE tdb_creature_template_model")
            with self.assertRaises(sqlite3.OperationalError):
                importer.import_database(destination)
            with closing(sqlite3.connect(destination)) as conn:
                self.assertEqual(before_failed_import, conn.execute("SELECT * FROM content_creature WHERE map=36 ORDER BY guid").fetchall())


if __name__ == "__main__":
    unittest.main()
