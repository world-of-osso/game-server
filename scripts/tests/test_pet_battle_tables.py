import csv
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'import-pet-battle-tables.py'


class BattleTableImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.database = self.root / 'world.db'
        self.rows = {
            'BattlePetAbility': {'ID': 110, 'Name_lang': 'Bite', 'Description_lang': 'First, bite.\nThen "attack".', 'IconFileDataID': 1, 'PetTypeEnum': 7, 'Cooldown': 0, 'BattlePetVisualID': 1, 'Flags': 0},
            'BattlePetAbilityTurn': {'ID': 1, 'BattlePetAbilityID': 110, 'OrderIndex': 1, 'TurnTypeEnum': 0, 'EventTypeEnum': 0, 'BattlePetVisualID': 0},
            'BattlePetAbilityEffect': dict(ID=1, BattlePetAbilityTurnID=1, OrderIndex=1, BattlePetEffectPropertiesID=24, AuraBattlePetAbilityID=0, BattlePetVisualID=0, **{f'Param_{i}': i for i in range(6)}),
            'BattlePetEffectProperties': dict(ID=24, BattlePetVisualID=0, **{f'ParamLabel_{i}': 'Points' if i == 0 else '' for i in range(6)}, **{f'ParamTypeEnum_{i}': 0 for i in range(6)}),
            'BattlePetState': dict(ID=18, LuaName='Stat_Power', Flags=0, BattlePetVisualID=0),
            'BattlePetSpeciesState': dict(ID=1, BattlePetStateID=18, Value=-100, BattlePetSpeciesID=39),
            'BattlePetBreedState': dict(ID=1, BattlePetStateID=18, Value=1700, BattlePetBreedID=3),
            'BattlePetBreedQuality': dict(ID=4, MaxQualityRoll=100, StateMultiplier=0.649999976, QualityEnum=3),
            'BattlePetSpeciesXAbility': dict(ID=1, BattlePetAbilityID=110, RequiredLevel=1, SlotEnum=-1, BattlePetSpeciesID=39),
            'BattlePetAbilityState': dict(ID=1, BattlePetStateID=18, Value=0, BattlePetAbilityID=110),
        }
        self.write_exports()
        with sqlite3.connect(self.database) as db:
            db.execute('CREATE TABLE toy(ID INTEGER PRIMARY KEY)')
            db.execute('INSERT INTO toy VALUES(99)')
        spec = importlib.util.spec_from_file_location('pet_battle_import', SCRIPT)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)

    def write_exports(self):
        for table, row in self.rows.items():
            with (self.root / f'{table}.csv').open('w', newline='') as f:
                writer = csv.DictWriter(f, fieldnames=list(row))
                writer.writeheader()
                writer.writerow(row)

    def test_multiline_text_numeric_affinity_and_unrelated_data(self):
        counts = self.module.import_tables(self.root, self.database)
        self.assertEqual(counts['battle_pet_ability'], 1)
        with sqlite3.connect(self.database) as db:
            self.assertEqual(db.execute('SELECT Description_lang FROM battle_pet_ability').fetchone()[0], 'First, bite.\nThen "attack".')
            self.assertEqual(db.execute('SELECT Value, typeof(Value) FROM battle_pet_species_state').fetchone(), (-100, 'integer'))
            self.assertEqual(db.execute('SELECT StateMultiplier, typeof(StateMultiplier) FROM battle_pet_breed_quality').fetchone(), (0.649999976, 'real'))
            self.assertEqual(db.execute('SELECT SlotEnum FROM battle_pet_species_x_ability').fetchone()[0], -1)
            self.assertEqual(db.execute('SELECT ID FROM toy').fetchone()[0], 99)
        self.module.import_tables(self.root, self.database)
        with sqlite3.connect(self.database) as db:
            self.assertEqual(db.execute('SELECT count(*) FROM battle_pet_ability').fetchone()[0], 1)

    def test_invalid_later_export_preserves_previous_import(self):
        self.module.import_tables(self.root, self.database)
        self.rows['BattlePetAbility']['Name_lang'] = 'Changed'
        self.rows['BattlePetAbilityState']['Value'] = 'not a number'
        self.write_exports()
        with self.assertRaisesRegex(ValueError, 'BattlePetAbilityState.*Value'):
            self.module.import_tables(self.root, self.database)
        with sqlite3.connect(self.database) as db:
            self.assertEqual(db.execute('SELECT Name_lang FROM battle_pet_ability').fetchone()[0], 'Bite')

    def test_missing_export_fails_without_creating_partial_tables(self):
        (self.root / 'BattlePetAbilityState.csv').unlink()
        with self.assertRaises(FileNotFoundError):
            self.module.import_tables(self.root, self.database)
        with sqlite3.connect(self.database) as db:
            self.assertIsNone(db.execute("SELECT name FROM sqlite_master WHERE name='battle_pet_ability'").fetchone())


if __name__ == '__main__':
    unittest.main()
