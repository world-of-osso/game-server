"""Registered-map extraction and build-selected local CASC products."""

import sqlite3
from contextlib import closing
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ground_files


class GroundSelectionTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.db = Path(self.temp.name) / "world.db"
        with closing(sqlite3.connect(self.db)) as db:
            db.executescript("""
                CREATE TABLE map (ID INTEGER PRIMARY KEY, WdtFileDataID INTEGER);
                CREATE TABLE content_creature (map INTEGER);
                CREATE TABLE content_gameobject (map INTEGER);
                CREATE TABLE db2_row_source (table_name TEXT, row_id INTEGER, build TEXT);
                INSERT INTO map VALUES (0,775971),(2991,7198644),(42,123);
                INSERT INTO content_creature VALUES (0),(0);
                INSERT INTO db2_row_source VALUES ('map',2991,'1.60.1.70205');
            """)

    def test_explicit_map_without_spawns_uses_its_source_product(self):
        self.assertEqual(
            ground_files.selected_map_sources(self.db, [2991]),
            [(2991, 7198644, "wow_classic_beta")],
        )

    def test_default_deploy_selection_includes_gameobject_only_zone(self):
        with closing(sqlite3.connect(self.db)) as db:
            db.execute("INSERT INTO content_gameobject VALUES (42)")
            db.commit()
        self.assertEqual(ground_files.selected_map_sources(self.db),
                         [(0, 775971, "wow"), (42, 123, "wow")])

    def test_registered_maps_and_default_spawn_selection(self):
        self.assertEqual(
            ground_files.selected_map_sources(self.db), [(0, 775971, "wow")]
        )
        self.assertEqual(
            ground_files.selected_map_sources(self.db, registered=True),
            [(0, 775971, "wow"), (42, 123, "wow"), (2991, 7198644, "wow_classic_beta")],
        )

    def test_extraction_follows_maid_modf_gfid_and_normalizes_unlisted_files(self):
        cache = Path(self.temp.name) / "cache"
        source = Path(self.temp.name) / "source"
        source.mkdir()

        def chunk(tag, payload):
            return tag[::-1] + struct.pack("<I", len(payload)) + payload

        version = chunk(b"MVER", struct.pack("<I", 18))
        modf = bytearray(64)
        struct.pack_into("<I", modf, 0, 333477)
        struct.pack_into("<H", modf, 56, 8)
        assets = {
            7198644: version
            + chunk(b"MPHD", bytes(4))
            + chunk(b"MAID", struct.pack("<II", 7199999, 7200000) + bytes(24)),
            7199999: version,
            7200000: version + chunk(b"MODF", modf),
            333477: version
            + chunk(b"MOHD", struct.pack("<II", 0, 1))
            + chunk(b"GFID", struct.pack("<I", 447893)),
            447893: version + chunk(b"MOGP", bytes(68)),
        }
        for fdid, data in assets.items():
            (source / f"{fdid}.dat").write_bytes(data)
        executable = Path(self.temp.name) / "casc-local"
        executable.write_text(
            "#!/usr/bin/env python3\n"
            "import os,sys,shutil\nfrom pathlib import Path\n"
            "assert os.environ['WOW_PRODUCT'] == 'wow_classic_beta'\n"
            "args=sys.argv[1:]; split=args.index('-o'); out=Path(args[split+1])\n"
            "for fdid in args[:split]: shutil.copy(Path('source')/(fdid+'.dat'),out/(fdid+'.dat'))\n"
        )
        executable.chmod(0o755)
        result = subprocess.run(
            [
                sys.executable,
                str(Path(ground_files.__file__).with_name("extract_ground.py")),
                "--world-db",
                str(self.db),
                "--cache",
                str(cache),
                "--engine",
                self.temp.name,
                "--casc-local",
                str(executable),
                "--failed",
                str(cache.parent / "failed.tsv"),
                "--map",
                "2991",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        files = ground_files.GroundFiles(cache)
        files.start_map(2991)
        files.add_map(7198644)
        self.assertEqual(files.missing, [])
        self.assertEqual(len(files.present), 5)
        self.assertEqual(list(cache.rglob("*.dat")), [])

    def test_unregistered_map_fails_explicitly(self):
        with self.assertRaisesRegex(ValueError, "999.*registered"):
            ground_files.selected_map_sources(self.db, [999])


if __name__ == "__main__":
    unittest.main()
