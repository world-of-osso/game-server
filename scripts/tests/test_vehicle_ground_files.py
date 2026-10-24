"""Deploy closure for mount bind poses, including unspawned passenger vehicles."""

from pathlib import Path
import sqlite3
import struct
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import ground_files


class VehicleGroundFilesTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.cache = Path(self.temp.name) / "cache"
        (self.cache / "models").mkdir(parents=True)
        self.world = Path(self.temp.name) / "world.db"
        self.db = sqlite3.connect(self.world)
        self.addCleanup(self.db.close)
        self.db.executescript("""
            CREATE TABLE map (ID INTEGER, WdtFileDataID INTEGER, InstanceType INTEGER);
            CREATE TABLE content_creature (map INTEGER, id1 INTEGER);
            CREATE TABLE content_creature_template (entry INTEGER, VehicleId INTEGER);
            CREATE TABLE content_creature_template_model (CreatureID INTEGER, CreatureDisplayID INTEGER);
            CREATE TABLE content_gameobject (map INTEGER);
            CREATE TABLE mount (ID INTEGER, SourceSpellID INTEGER);
            CREATE TABLE mount_x_display (MountID INTEGER, CreatureDisplayInfoID INTEGER);
            CREATE TABLE spell_effect (SpellID INTEGER, EffectMiscValue_0 INTEGER,
                                       EffectAura INTEGER, DifficultyID INTEGER);
            CREATE TABLE creature_template (entry INTEGER, VehicleId INTEGER);
            CREATE TABLE creature_display_info (ID INTEGER, ModelID INTEGER, CreatureModelScale REAL);
            CREATE TABLE creature_model_data (ID INTEGER, FileDataID INTEGER);
            CREATE TABLE vehicle (ID INTEGER, SeatID_0 INTEGER, SeatID_1 INTEGER,
                SeatID_2 INTEGER, SeatID_3 INTEGER, SeatID_4 INTEGER, SeatID_5 INTEGER,
                SeatID_6 INTEGER, SeatID_7 INTEGER);
            CREATE TABLE vehicle_seat (ID INTEGER, Flags INTEGER);
            INSERT INTO mount VALUES (1, 61425), (2, 555), (3, 556), (4, 557);
            INSERT INTO mount_x_display VALUES (1, 27237), (1, 27238), (2, 99), (3, 98), (4, 97);
            INSERT INTO spell_effect VALUES (61425, 32633, 78, 0), (555, 10, 78, 0),
                (556, 11, 78, 0), (557, 12, 78, 1);
            INSERT INTO creature_template VALUES (32633, 312), (10, 100), (11, 101), (12, 312);
            INSERT INTO creature_display_info VALUES (27237, 2824, 1), (27238, 2824, 1),
                (99, 99, 1), (98, 98, 1), (97, 97, 1);
            INSERT INTO creature_model_data VALUES (2824, 234720), (99, 99), (98, 98), (97, 97);
            INSERT INTO vehicle VALUES (312, 2764, 0, 0, 0, 0, 0, 0, 0),
                (100, 100, 0, 0, 0, 0, 0, 0, 0), (101, 101, 0, 0, 0, 0, 0, 0, 0);
            INSERT INTO vehicle_seat VALUES (2764, -570392565), (100, 33556480), (101, 0);
        """)
        self.db.commit()

    def list_files(self, *args):
        return subprocess.run(
            [sys.executable, "-I", ROOT / "scripts/ground_files.py", "--world-db",
             self.world, "--cache", self.cache, *args],
            capture_output=True, text=True, check=True,
        ).stdout.splitlines()

    def test_unspawned_passenger_model_ships_once_without_driver_only_models(self):
        (self.cache / "models/234720.m2").write_bytes(b"mount bind pose")
        self.assertEqual(self.list_files(), ["models/234720.m2"])

    def test_isle_demolisher_driver_model_ships_alongside_passenger_mounts(self):
        self.db.executescript("""
            INSERT INTO map VALUES (628, 821811, 3);
            INSERT INTO content_creature VALUES (628, 34775);
            INSERT INTO content_creature_template VALUES (34775, 509);
            INSERT INTO content_creature_template_model VALUES (34775, 27658);
            INSERT INTO creature_display_info VALUES (27658, 2797, 0.75);
            INSERT INTO creature_model_data VALUES (2797, 123367);
        """)
        self.db.commit()
        (self.cache / "terrain").mkdir()
        # An empty MAID fixture isolates model closure from terrain dependencies.
        (self.cache / "terrain/821811.wdt").write_bytes(b"DIAM" + struct.pack("<I", 32) + bytes(32))
        (self.cache / "models/234720.m2").write_bytes(b"mount bind pose")
        (self.cache / "models/123367.m2").write_bytes(b"Demolisher bind pose")
        self.assertCountEqual(
            self.list_files(),
            ["terrain/821811.wdt", "models/123367.m2", "models/234720.m2"],
        )
        (self.cache / "models/123367.m2").unlink()
        self.assertEqual(self.list_files("--missing"), ["vehicle 509\tmodels/123367.m2"])

    def test_spawnless_housing_ground_and_passenger_bind_pose_ship_through_deploy_cli(self):
        maps = [(2735, 5660542, 8), (2736, 5660543, 8), (2783, 5890928, 7)]
        self.db.executemany("INSERT INTO map VALUES (?, ?, ?)", maps)
        self.db.commit()
        (self.cache / "terrain").mkdir()
        (self.cache / "models/234720.m2").write_bytes(b"mount bind pose")
        expected = {"models/234720.m2"}
        for map_id, wdt, _kind in maps:
            root, obj0 = 8000000 + map_id, 9000000 + map_id
            maid = struct.pack("<II", root, obj0) + bytes(24)
            (self.cache / f"terrain/{wdt}.wdt").write_bytes(
                b"DIAM" + struct.pack("<I", len(maid)) + maid
            )
            for fdid in (root, obj0):
                (self.cache / f"terrain/{fdid}.adt").write_bytes(
                    b"REVM" + struct.pack("<II", 4, 18)
                )
            expected.update((f"terrain/{wdt}.wdt", f"terrain/{root}.adt", f"terrain/{obj0}.adt"))
        self.assertEqual(set(self.list_files()), expected)
        self.assertFalse(any(self.list_files("--missing")))

    def test_discovery_lists_large_payloads_without_reading_their_contents(self):
        def chunk(tag, payload):
            return tag[::-1] + struct.pack("<I", len(payload)) + payload

        def read_chars():
            counters = dict(
                line.split(": ", 1)
                for line in Path("/proc/self/io").read_text().splitlines()
            )
            return int(counters["rchar"])

        (self.cache / "terrain").mkdir()
        maid = struct.pack("<II", 11, 12) + bytes(24)
        (self.cache / "terrain/10.wdt").write_bytes(chunk(b"MAID", maid))
        modf = bytearray(64)
        struct.pack_into("<I", modf, 0, 20)
        struct.pack_into("<H", modf, 56, 8)
        (self.cache / "terrain/12.adt").write_bytes(chunk(b"MODF", modf))
        (self.cache / "models/20.wmo").write_bytes(
            chunk(b"MOHD", struct.pack("<II", 0, 2))
            + chunk(b"GFID", struct.pack("<II", 21, 22))
        )
        for relative in ("terrain/11.adt", "models/21.wmo", "models/234720.m2"):
            with (self.cache / relative).open("wb") as payload:
                payload.truncate(16 * 1024 * 1024)

        files = ground_files.GroundFiles(self.cache)
        files.start_map(7)
        before = read_chars()
        files.add_map(10)
        ground_files.add_vehicle_models(files, self.world)
        bytes_read = read_chars() - before
        self.assertEqual(
            list(files.present),
            ["terrain/10.wdt", "terrain/11.adt", "terrain/12.adt",
             "models/20.wmo", "models/21.wmo", "models/234720.m2"],
        )
        self.assertEqual(files.missing, [(7, "models/22.wmo")])
        self.assertLess(bytes_read, 1024 * 1024, "discovery read unused 48MiB payloads")

    def test_missing_passenger_model_is_reported_with_vehicle_id(self):
        self.assertEqual(self.list_files("--missing"), ["vehicle 312\tmodels/234720.m2"])


if __name__ == "__main__":
    unittest.main()
