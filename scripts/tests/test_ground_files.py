"""The ground files deploy.sh ships, listed from the real world.db and client cache."""

from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORLD_DB = ROOT / "data/world.db"
CACHE = ROOT.parent / "game-engine/data"


@unittest.skipUnless(WORLD_DB.is_file() and CACHE.is_dir(), "needs data/world.db and ../game-engine/data")
class GroundFilesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        run = subprocess.run(
            [sys.executable, ROOT / "scripts/ground_files.py", "--world-db", WORLD_DB, "--cache", CACHE],
            capture_output=True, text=True, check=True,
        )
        cls.files = run.stdout.splitlines()
        cls.summary = run.stderr

    def test_northshire_tile_ships_with_the_azeroth_wdt(self):
        # Azeroth WDT 775971; MAID (32, 48): root 778027, obj0 778028 (the spy's ridge).
        for path in ["terrain/775971.wdt", "terrain/778027.adt", "terrain/778028.adt"]:
            self.assertIn(path, self.files)

    def test_stockade_ships_its_global_wmo_and_every_group(self):
        # stormwindjail.wdt 791060 places WMO 108631, groups 108632..108643 (GFID).
        expected = ["terrain/791060.wdt"] + [f"models/{fdid}.wmo" for fdid in range(108631, 108644)]
        for path in expected:
            self.assertIn(path, self.files)

    def test_every_listed_file_is_in_the_cache_once(self):
        self.assertEqual(len(self.files), len(set(self.files)))
        self.assertEqual([path for path in self.files if not (CACHE / path).is_file()], [])
        self.assertIn(f"{len(self.files)} cached files", self.summary)


if __name__ == "__main__":
    unittest.main()
