"""Decode pinned readable Forever creature rows, without fabricating Zephras displays."""

import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SOURCE = Path(os.environ["FOREVER_DB2_SOURCE"])


class CreatureDecodeTest(unittest.TestCase):
    def test_known_names_displays_and_relationships(self):
        import zephras_creatures

        catalog, report = zephras_creatures.load_catalog(SOURCE)
        squirrel = catalog[2671]
        self.assertEqual(squirrel["name"], "Mechanical Squirrel")
        self.assertEqual(squirrel["type"], 1)
        self.assertEqual(squirrel["classification"], 0)
        self.assertEqual(squirrel["models"][0]["display_id"], 7937)
        self.assertEqual(catalog[23909]["models"][0]["display_id"], 21900)
        self.assertEqual(report["creature_rows"], 179)
        self.assertEqual(report["xdisplay_rows"], 185)
        self.assertEqual(report["encrypted_sections"], {"841631": 2, "1864302": 2})
        self.assertNotIn(251362, catalog)
        self.assertNotIn(251487, catalog)

    def test_corruption_fails_before_partial_catalog_is_returned(self):
        import shutil
        import tempfile

        import zephras_creatures

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for fdid in (841631, 1864302):
                shutil.copyfile(
                    SOURCE / f"{fdid}.db2",
                    root / f"{fdid}.db2",
                )
            path = root / "841631.db2"
            path.write_bytes(path.read_bytes() + b"corrupt")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                zephras_creatures.load_catalog(root)


if __name__ == "__main__":
    unittest.main()
