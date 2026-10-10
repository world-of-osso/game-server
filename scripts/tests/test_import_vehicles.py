import csv
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "import-vehicles.py"


class VehicleImportTests(unittest.TestCase):
    def test_import_preserves_authored_values_and_unrelated_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vehicle = {"ID": 312, **{f"SeatID_{i}": 2764 if i == 1 else 0 for i in range(8)}}
            seat = {"ID": 2764, "Flags": -2113929216, "AttachmentID": 14,
                    "AttachmentOffset_0": 1.25, "AttachmentOffset_1": -2.5,
                    "AttachmentOffset_2": 3.75}
            for name, row in (("Vehicle", vehicle), ("VehicleSeat", seat)):
                with (root / f"{name}.csv").open("w", newline="") as handle:
                    writer = csv.DictWriter(handle, fieldnames=row)
                    writer.writeheader()
                    writer.writerow(row)
            database = root / "world.db"
            with sqlite3.connect(database) as conn:
                conn.execute("CREATE TABLE preserved (value INTEGER)")
                conn.execute("INSERT INTO preserved VALUES (42)")
            for _ in range(2):
                result = subprocess.run([sys.executable, str(SCRIPT), str(root), str(database)],
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(database) as conn:
                self.assertEqual(conn.execute("SELECT * FROM vehicle").fetchall(),
                                 [(312, 0, 2764, 0, 0, 0, 0, 0, 0)])
                self.assertEqual(conn.execute("SELECT * FROM vehicle_seat").fetchall(),
                                 [(2764, -2113929216, 14, 1.25, -2.5, 3.75)])
                self.assertEqual(conn.execute("SELECT * FROM preserved").fetchall(), [(42,)])


if __name__ == "__main__":
    unittest.main()
