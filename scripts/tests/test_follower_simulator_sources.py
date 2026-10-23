"""Read-only native source contracts; edited fixtures are private in-memory copies."""
import csv
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from follower_simulator_sources import SourceGap, read_source_graph

WORLD = Path("/home/osso/Projects/world-of-osso/game-server/data/world.db")
NAV = Path("/home/osso/Projects/world-of-osso/game-server/data/ground/navmesh")
JOURNAL = Path("/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.5.70448/JournalEncounterCreature.csv")


class NativeFollowerSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.native = sqlite3.connect(":memory:", uri=True)
        cls.native.execute("ATTACH DATABASE ? AS source", (WORLD.resolve().as_uri() + "?mode=ro",))
        filters = {
            "follower_lfg_dungeons": "map_id IN (2521,2527)",
            "map_difficulty": "MapID IN (2521,2527)",
            "tdb_world_safe_locs": "MapID IN (2521,2527)",
            "dungeon_encounter": "MapID IN (2521,2527)",
            "raid_encounter_creature": "map_id IN (2521,2527)",
            "content_creature": "map IN (2521,2527)",
            "content_creature_formations": "memberGUID IN (SELECT guid FROM source.content_creature WHERE map IN (2521,2527))",
        }
        for table, where in filters.items():
            cls.native.execute(f"CREATE TABLE {table} AS SELECT * FROM source.{table} WHERE {where}")
        cls.native.commit()
        with JOURNAL.open() as stream:
            cls.journal = {int(row["ID"]): row for row in csv.DictReader(stream)}

    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.native.backup(self.db)

    def tearDown(self):
        self.db.close()

    @classmethod
    def tearDownClass(cls):
        cls.native.close()

    def read_ruby(self, nav=None):
        return read_source_graph(self.db, 2521, nav or NAV / "2521", self.journal)

    def test_ruby_retains_every_normal_identity_pose_and_explicit_boss_order(self):
        graph, requirements, receipt = self.read_ruby()
        self.assertEqual(requirements["difficulty_id"], 205)
        self.assertEqual(requirements["boss_order"], [188252, 189232, 190485])
        self.assertEqual(receipt["entrance_id"], 100030)
        self.assertEqual(receipt["content_tuning_id"], 2845)
        actual = {row["guid"] for pack in graph["packs"] for row in pack["spawns"]}
        expected = {row[0] for row in self.db.execute("SELECT guid FROM content_creature WHERE map=2521 AND (spawnMask&1)!=0")}
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 307)
        self.assertEqual(graph["route"], [])
        self.assertFalse(receipt["pull_route_qualified"])

    def test_halls_refuses_missing_final_encounter_not_a_three_boss_clear(self):
        with self.assertRaisesRegex(SourceGap, "MISSING_ENCOUNTER_BOSS_SOURCE_2527: encounter2618"):
            read_source_graph(self.db, 2527, NAV / "2527", self.journal)

    def test_missing_entrance_is_a_named_source_refusal(self):
        self.db.execute("DELETE FROM tdb_world_safe_locs WHERE MapID=2521")
        with self.assertRaisesRegex(SourceGap, "MISSING_OR_AMBIGUOUS_ENTRANCE_2521"):
            self.read_ruby()

    def test_missing_boss_spawn_is_a_named_source_refusal(self):
        self.db.execute("DELETE FROM content_creature WHERE id1=188252")
        with self.assertRaisesRegex(SourceGap, "MISSING_OR_AMBIGUOUS_BOSS_SPAWN_2521: entry188252"):
            self.read_ruby()

    def test_missing_nav_never_becomes_a_walk_connector(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(SourceGap, "MISSING_NAV_BAKE_2521"):
                self.read_ruby(Path(directory))

    def test_missing_follower_difficulty_never_substitutes_normal(self):
        self.db.execute("DELETE FROM map_difficulty WHERE MapID=2521 AND DifficultyID=205")
        with self.assertRaisesRegex(SourceGap, "MISSING_FOLLOWER_DIFFICULTY_2521"):
            self.read_ruby()


if __name__ == "__main__":
    unittest.main()
