"""Deploy closure for mount bind poses, including unspawned passenger vehicles."""

from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


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
            CREATE TABLE map (ID INTEGER, WdtFileDataID INTEGER);
            CREATE TABLE content_creature (map INTEGER);
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

    def test_missing_passenger_model_is_reported_with_vehicle_id(self):
        self.assertEqual(self.list_files("--missing"), ["vehicle 312\tmodels/234720.m2"])


if __name__ == "__main__":
    unittest.main()
