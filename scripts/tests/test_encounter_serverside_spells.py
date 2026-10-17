import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import encounter_serverside_spells

FIXTURE = Path(__file__).parent / 'fixtures/ragefire-serverside-spells.json'


def write_dump(path, missing=None, fixture=FIXTURE):
    source = json.loads(fixture.read_text())
    statements = []
    for table, columns in source['schema'].items():
        statements.append(f'CREATE TABLE `{table}` (\n')
        statements.extend(f'  `{column}` text,\n' for column in columns)
        statements.append(');\n')
        rows = source['rows'][table]
        for row in rows:
            spell_id = row.get('Id', row.get('SpellID'))
            if missing == (table, spell_id):
                continue
            values = []
            for column in columns:
                value = row[column]
                values.append("'" + value.replace("'", "\\'") + "'" if isinstance(value, str) else str(value))
            statements.append(f"INSERT INTO `{table}` VALUES ({','.join(values)});\n")
    path.write_text(''.join(statements))


class EncounterServersideSpellsTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.db = self.root / 'private.sqlite'
        self.dump = self.root / 'source.sql'
        with sqlite3.connect(self.db) as connection:
            connection.execute('CREATE TABLE unrelated (id INTEGER PRIMARY KEY, value TEXT)')
            connection.execute('INSERT INTO unrelated VALUES (1, "retained")')
        write_dump(self.dump)

    def test_preserves_actual_source_spell_effects_and_unrelated_data_idempotently(self):
        encounter_serverside_spells.import_spells(self.db, self.dump)
        encounter_serverside_spells.import_spells(self.db, self.dump)
        with sqlite3.connect(self.db) as connection:
            self.assertEqual(connection.execute("SELECT name FROM sqlite_master WHERE name LIKE 'tdb_serverside%' ORDER BY name").fetchall(),
                             [('tdb_serverside_spell',), ('tdb_serverside_spell_effect',)])
            self.assertEqual(connection.execute('SELECT count(*) FROM tdb_serverside_spell').fetchone(), (6,))
            self.assertEqual(connection.execute('SELECT count(*) FROM tdb_serverside_spell_effect').fetchone(), (6,))
            self.assertEqual(connection.execute('SELECT Id,DurationIndex,RangeIndex,SpellName FROM tdb_serverside_spell ORDER BY Id').fetchall(),
                             [(119297, 28, 5, 'Inferno Charge'), (119298, 0, 5, 'Inferno Charge'),
                              (119424, 0, 1, 'Magnaw Teleport North'), (119425, 0, 1, 'Magnaw Teleport East'),
                              (119426, 0, 1, 'Magnaw Teleport South'), (119428, 0, 1, 'Magnaw Teleport West')])
            self.assertEqual(connection.execute('SELECT SpellID,Effect,EffectMiscValue1,EffectMiscValue2,ImplicitTarget1,ImplicitTarget2 FROM tdb_serverside_spell_effect ORDER BY SpellID').fetchall(),
                             [(119297, 28, 61409, 491, 53, 0), (119298, 96, 0, 0, 46, 0), (119424, 252, 0, 0, 1, 17),
                              (119425, 252, 0, 0, 1, 17), (119426, 252, 0, 0, 1, 17), (119428, 252, 0, 0, 1, 17)])
            self.assertEqual(connection.execute('SELECT EffectTriggerSpell FROM tdb_serverside_spell_effect WHERE SpellID=119298').fetchone(), (119299,))
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone(), ('retained',))
            connection.row_factory = sqlite3.Row
            expected = json.loads(FIXTURE.read_text())['rows']
            for table, source_rows in expected.items():
                id_column = 'Id' if table == 'serverside_spell' else 'SpellID'
                actual = [dict(row) for row in connection.execute(f'SELECT * FROM tdb_{table} ORDER BY {id_column}')]
                self.assertEqual(actual, source_rows, 'Every actual source field must survive unchanged')

    def test_gargolmar_request_heal_preserves_all_authored_fields(self):
        fixture = FIXTURE.with_name('gargolmar-serverside-spells.json')
        write_dump(self.dump, fixture=fixture)
        for _ in range(2):
            encounter_serverside_spells.import_spells(self.db, self.dump, encounter='gargolmar')
        with sqlite3.connect(self.db) as connection:
            connection.row_factory = sqlite3.Row
            source = json.loads(fixture.read_text())['rows']
            for table, expected in source.items():
                self.assertEqual([dict(row) for row in connection.execute(f'SELECT * FROM tdb_{table}')], expected)
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone()[0], 'retained')
        write_dump(self.dump, missing=('serverside_spell_effect', 30642), fixture=fixture)
        with self.assertRaisesRegex(ValueError, '30642'):
            encounter_serverside_spells.import_spells(self.db, self.dump, encounter='gargolmar')

    def test_omor_orbital_spells_preserve_source_fields_and_refuse_missing_effect(self):
        fixture = FIXTURE.with_name('omor-serverside-spells.json')
        write_dump(self.dump, fixture=fixture)
        for _ in range(2):
            encounter_serverside_spells.import_spells(self.db, self.dump, encounter='omor')
        with sqlite3.connect(self.db) as connection:
            connection.row_factory = sqlite3.Row
            source = json.loads(fixture.read_text())['rows']
            for table, expected in source.items():
                key = 'Id' if table == 'serverside_spell' else 'SpellID'
                self.assertEqual([dict(row) for row in connection.execute(f'SELECT * FROM tdb_{table} ORDER BY {key}')], expected)
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone()[0], 'retained')
        write_dump(self.dump, missing=('serverside_spell_effect', 32185), fixture=fixture)
        with self.assertRaisesRegex(ValueError, '32185'):
            encounter_serverside_spells.import_spells(self.db, self.dump, encounter='omor')

    def run_herald_producer(self):
        return subprocess.run(
            [sys.executable, str(Path(encounter_serverside_spells.__file__)),
             '--encounter', 'herald', '--db', str(self.db), '--source-dump', str(self.dump)],
            capture_output=True, text=True, check=False,
        )

    def read_herald_tables(self):
        with sqlite3.connect(self.db) as connection:
            connection.row_factory = sqlite3.Row
            return {
                'serverside_spell': [dict(row) for row in connection.execute(
                    'SELECT * FROM tdb_serverside_spell ORDER BY Id,DifficultyID')],
                'serverside_spell_effect': [dict(row) for row in connection.execute(
                    'SELECT * FROM tdb_serverside_spell_effect ORDER BY SpellID,EffectIndex,DifficultyID')],
            }

    def test_herald_cli_preserves_seven_source_spells_and_refuses_incomplete_input_atomically(self):
        fixture = FIXTURE.with_name('herald-serverside-spells.json')
        write_dump(self.dump, fixture=fixture)
        expected = json.loads(fixture.read_text())['rows']
        for _ in range(2):
            result = self.run_herald_producer()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(self.read_herald_tables(), expected,
                             'All seven actual spell/effect records and every field survive the real CLI')
        with sqlite3.connect(self.db) as connection:
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone(), ('retained',))
        before_failure = self.db.read_bytes()
        write_dump(self.dump, missing=('serverside_spell_effect', 30698), fixture=fixture)
        result = self.run_herald_producer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('missing authored serverside_spell_effect spells [30698]', result.stderr)
        self.assertEqual(self.db.read_bytes(), before_failure,
                         'Rejected incomplete source cannot partially mutate a prepared copy')

    def test_missing_authored_spell_refuses_without_partial_tables(self):
        write_dump(self.dump, missing=('serverside_spell_effect', 119426))
        with self.assertRaisesRegex(ValueError, '119426'):
            encounter_serverside_spells.import_spells(self.db, self.dump)
        with sqlite3.connect(self.db) as connection:
            self.assertEqual(connection.execute("SELECT name FROM sqlite_master WHERE name LIKE 'tdb_serverside%'").fetchall(), [])
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone(), ('retained',))

    def test_canonical_name_and_symlink_refused_without_writes(self):
        canonical = self.root / 'world.db'
        canonical.write_bytes(self.db.read_bytes())
        before = canonical.read_bytes()
        with self.assertRaisesRegex(ValueError, 'independent'):
            encounter_serverside_spells.import_spells(canonical, self.dump)
        self.assertEqual(canonical.read_bytes(), before)
        linked = self.root / 'linked.sqlite'
        linked.symlink_to(self.db)
        with self.assertRaisesRegex(ValueError, 'independent'):
            encounter_serverside_spells.import_spells(linked, self.dump)


if __name__ == '__main__':
    unittest.main()
