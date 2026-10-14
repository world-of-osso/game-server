"""Toy.db2 membership must survive real runtime-content preparation."""
from contextlib import closing
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import content_policy
import retail_content
from content_common import Report


class ToyItemImportTests(unittest.TestCase):
    def test_resolved_toys_are_source_exact_idempotent_and_rebuildable(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.sqlite"
            shutil.copyfile(ROOT / "data/world.db", path)
            with closing(sqlite3.connect(path)) as conn, conn:
                original = conn.execute("SELECT * FROM content_item WHERE ID=32782").fetchone()
                for _ in range(2):
                    result = subprocess.run(
                        ["python3", str(ROOT / "scripts/add_toy_summon_templates.py"), "--db", str(path)],
                        capture_output=True, text=True,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                expected = conn.execute("""SELECT s.ID,s.Display_lang,s.RequiredSkill,
                    s.RequiredSkillRank,s.RequiredLevel,s.AllowableClass,s.AllowableRaces_0,
                    s.AllowableRaces_1 FROM toy t JOIN item_sparse s ON s.ID=t.ItemID
                    JOIN item i ON i.ID=s.ID ORDER BY s.ID""").fetchall()
                actual = conn.execute("""SELECT c.ID,c.Name,c.RequiredSkill,c.RequiredSkillRank,
                    c.RequiredLevel,c.AllowableClass,c.AllowableRaces_0,c.AllowableRaces_1
                    FROM content_item c JOIN toy t ON t.ItemID=c.ID ORDER BY c.ID""").fetchall()
                self.assertEqual(actual, expected)
                self.assertEqual(conn.execute("SELECT * FROM content_item WHERE ID=32782").fetchone(), original)
                self.assertIsNone(conn.execute("SELECT ID FROM content_item WHERE ID=72231").fetchone())
                source_ids = retail_content.retail_item_ids(conn)
                self.assertTrue({64646, 109183, 113542}.issubset(source_ids))
                conn.execute("CREATE TEMP TABLE item_map(source_id INTEGER,retail_id INTEGER)")
                content_policy.build_items(conn, {}, {}, source_ids, Report())
                for item in [64646, 109183, 113542]:
                    self.assertIsNotNone(conn.execute("SELECT ID FROM content_item WHERE ID=?", (item,)).fetchone())


if __name__ == "__main__":
    unittest.main()
