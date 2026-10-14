"""Copy-only preparation, including product-isolated race identity."""
import csv
import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path


class SiteCatalogTests(unittest.TestCase):
    def test_native_creature_types_are_not_dropped_from_the_selected_source(self):
        path = Path(__file__).resolve().parents[1] / "prepare_site_catalog.py"
        spec = importlib.util.spec_from_file_location("site_catalog", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            csv_path = Path(directory) / "races.csv"
            csv_path.write_text("ID,Name_lang,FactionID,CreatureType\n1,Human,1,7\n12,Fel Orc,1701,2\n95,Skyborne,3629,9\n")
            self.assertEqual(module.read_races(csv_path, False), [(1,"Human",1,7),(12,"Fel Orc",1701,2)])
            self.assertEqual(module.read_races(csv_path, True), [(95,"Skyborne",3629,9)])

    def test_authored_races_preserve_source_and_choose_forever_for_skyborne(self):
        path = Path(__file__).resolve().parents[1] / "prepare_site_catalog.py"
        spec = importlib.util.spec_from_file_location("site_catalog", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.db"
            with sqlite3.connect(source) as conn:
                conn.execute("CREATE TABLE retained(value TEXT)")
                conn.execute("INSERT INTO retained VALUES('unchanged')")
            original = source.read_bytes()
            catalogs = [
                ("retail", [(1, "Human", 1, 7), (95, "TBD NPC Race 1", 1, 2)]),
                (
                    "forever",
                    [
                        (1, "FOREVER HUMAN", 1, 2),
                        (95, "High Order Skyborne", 3629, 9),
                        (96, "Windshaper Skyborne", 3628, 9),
                    ],
                ),
            ]
            for name, rows in catalogs:
                with (root / f"{name}.csv").open("w", newline="") as file:
                    writer = csv.writer(file)
                    writer.writerow(["ID", "Name_lang", "FactionID", "CreatureType"])
                    writer.writerows(rows)
            output = root / "site.db"
            module.prepare(
                source, root / "retail.csv", root / "forever.csv", output
            )
            self.assertEqual(source.read_bytes(), original)
            with sqlite3.connect(output) as conn:
                self.assertEqual(
                    conn.execute(
                        "SELECT Name_lang,FactionID,CreatureType FROM chr_races WHERE ID=1"
                    ).fetchone(),
                    ("Human", 1, 7),
                )
                self.assertEqual(
                    conn.execute(
                        "SELECT Name_lang,FactionID,CreatureType FROM chr_races WHERE ID=95"
                    ).fetchone(),
                    ("High Order Skyborne", 3629, 9),
                )
                self.assertEqual(
                    conn.execute("SELECT value FROM retained").fetchone(),
                    ("unchanged",),
                )
            with self.assertRaises(FileExistsError):
                module.prepare(
                    source, root / "retail.csv", root / "forever.csv", source
                )
            self.assertEqual(source.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
