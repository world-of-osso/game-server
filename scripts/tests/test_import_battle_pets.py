"""Local-CASC catalog import, transactional replacement and shipped DB behavior."""
import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "import_battle_pets", Path(__file__).parents[1] / "import-battle-pets.py"
)
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class ImportBattlePetsTests(unittest.TestCase):
    def test_import_replaces_catalog_and_preserves_other_tables(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "pets.json"
            database = Path(directory) / "world.db"
            row = dict(species_id=39, creature_id=2671, name="Mechanical Squirrel",
                       display_id=7937, summon_spell_id=4055)
            source.write_text(json.dumps([row]))
            with sqlite3.connect(database) as conn:
                conn.execute("CREATE TABLE toy (ID INTEGER)")
                conn.execute("INSERT INTO toy VALUES (1)")
            self.assertEqual(IMPORTER.import_catalog(source, database), 1)
            with sqlite3.connect(database) as conn:
                self.assertEqual(conn.execute("SELECT * FROM battle_pet_species").fetchall(),
                                 [(39, 2671, "Mechanical Squirrel", 7937, 4055)])
                self.assertEqual(conn.execute("SELECT * FROM toy").fetchall(), [(1,)])
            source.write_text(json.dumps([row, row]))
            with self.assertRaises(sqlite3.IntegrityError):
                IMPORTER.import_catalog(source, database)
            with sqlite3.connect(database) as conn:
                self.assertEqual(conn.execute("SELECT COUNT(*) FROM battle_pet_species").fetchone(), (1,))
            source.write_text("[]")
            self.assertEqual(IMPORTER.import_catalog(source, database), 0)
            with sqlite3.connect(database) as conn:
                self.assertEqual(conn.execute("SELECT COUNT(*) FROM battle_pet_species").fetchone(), (0,))


if __name__ == "__main__":
    unittest.main()
