"""Legacy flat NPC loot captures coexist with Forever calibration/content JSON."""

import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path
from contextlib import closing

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import wowhead_loot


class WowheadLootTest(unittest.TestCase):
    def test_content_regeneration_reads_npc_loot_not_capture_reports(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            closing(sqlite3.connect(":memory:")) as db,
        ):
            root = Path(directory)
            legacy = root / "legacy-20261004"
            legacy.mkdir()
            (legacy / "823.json").write_text(
                json.dumps(
                    {
                        "drops": [
                            {
                                "id": 25,
                                "quality": 1,
                                "classs": 2,
                                "stack": [1, 2],
                                "modes": {"0": {"count": 3, "outof": 10}},
                            }
                        ]
                    }
                )
            )
            forever = root / "forever-zephras-20261004"
            forever.mkdir()
            (forever / "assignments.json").write_text('{"uiMap":2665,"map":2991}')
            (forever / "zephras-content.json").write_text(
                '{"map":2991,"reconstructed":true}'
            )
            wowhead_loot.load_wowhead_drops(db, root)
            self.assertEqual(
                db.execute("SELECT * FROM wowhead_creature_drop").fetchall(),
                [(823, 25, 0, 3, 10, 1, 2, 1, 2, "legacy-20261004")],
            )


if __name__ == "__main__":
    unittest.main()
