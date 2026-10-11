import csv
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "import-pvp-progression.py"
SOURCE = SCRIPT.parent / "reference/pvp-12.1.0.69933"


class PvpImport(unittest.TestCase):
    def test_local_catalog_import_is_atomic_and_preserves_unrelated_data(self):
        spec = importlib.util.spec_from_file_location("pvp_import", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "world.db"
            conn = sqlite3.connect(db)
            conn.execute("CREATE TABLE owned(value TEXT)")
            conn.execute("INSERT INTO owned VALUES ('untouched')")
            conn.commit()
            counts = module.import_catalogs(SOURCE, db)
            self.assertEqual(counts["pvp_talent"], 407)
            self.assertEqual(conn.execute("SELECT name, cap FROM pvp_currency WHERE id=1792").fetchone(), ("Honor", 15000))
            self.assertEqual(conn.execute("SELECT world_state_cap FROM pvp_currency WHERE id=1602").fetchone(), (31270,))
            self.assertEqual(conn.execute("SELECT level, achievement FROM pvp_honor_reward WHERE id=1").fetchone(), (5, 12893))
            self.assertEqual(conn.execute("SELECT value FROM owned").fetchone(), ("untouched",))
            module.import_catalogs(SOURCE, db)
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM pvp_talent").fetchone()[0], 407)
            conn.close()

    def test_invalid_source_cannot_partially_replace_catalog(self):
        spec = importlib.util.spec_from_file_location("pvp_import", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "world.db"
            sqlite3.connect(db).close()
            module.import_catalogs(SOURCE, db)
            broken = Path(tmp) / "source"
            broken.mkdir()
            for source in SOURCE.glob("*.csv"):
                (broken / source.name).write_bytes(source.read_bytes())
            (broken / "PvpTalent.csv").write_text("ID,SpellID\n24,not-an-integer\n")
            with self.assertRaises((KeyError, ValueError)):
                module.import_catalogs(broken, db)
            with sqlite3.connect(db) as conn:
                self.assertEqual(conn.execute("SELECT spell_id FROM pvp_talent WHERE id=24").fetchone()[0], 236077)
                self.assertEqual(conn.execute("SELECT COUNT(*) FROM pvp_talent").fetchone()[0], 407)


if __name__ == "__main__":
    unittest.main()
