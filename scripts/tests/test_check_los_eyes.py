import csv
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "check_los_eyes.py"


class EyeCoverageTest(unittest.TestCase):
    def test_rejects_missing_spawn_override_and_pose_then_accepts_complete_table(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            db_path = root / "world.db"
            with sqlite3.connect(db_path) as db:
                db.executescript("""
                    CREATE TABLE map (ID INTEGER, WdtFileDataID INTEGER, InstanceType INTEGER);
                    CREATE TABLE content_creature (id1 INTEGER, map INTEGER, modelid INTEGER);
                    CREATE TABLE content_gameobject (map INTEGER);
                    CREATE TABLE content_creature_template_model (CreatureID INTEGER, CreatureDisplayID INTEGER);
                    INSERT INTO map VALUES (2519, 1, 1);
                    INSERT INTO content_creature VALUES (1, 2519, 22);
                    INSERT INTO content_creature_template_model VALUES (1, 11);
                """)
            table = root / "eyes.tsv"
            gaps = root / "gaps.tsv"
            gaps.write_text("display_id\tcause\n")
            def write_rows(rows):
                with table.open("w", newline="") as stream:
                    writer = csv.writer(stream, delimiter="\t")
                    writer.writerow(["display_id", "stand_state"])
                    writer.writerows(rows)
            def check():
                return subprocess.run([sys.executable, SCRIPT, "--world-db", db_path, "--table", table, "--gaps", gaps],
                                      text=True, capture_output=True)
            write_rows((11, state) for state in range(9))
            result = check()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("map 2519: display 11 missing eye states [9]", result.stdout)
            self.assertIn("map 2519: display 22 missing eye states", result.stdout)
            gaps.write_text("display_id\tcause\n11\tmissing animation\n22\tno model mapping\n")
            self.assertEqual(check().returncode, 0)
            gaps.write_text("display_id\tcause\n")
            write_rows((display, state) for display in (11, 22) for state in range(10))
            self.assertEqual(check().returncode, 0)


if __name__ == "__main__":
    unittest.main()
