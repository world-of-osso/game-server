import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

import encounter_serverside_spells

FIXTURE = Path(__file__).parent / 'fixtures/adarogg-serverside-spells.json'


def write_dump(path, missing=None):
    source = json.loads(FIXTURE.read_text())
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
            self.assertEqual(connection.execute('SELECT count(*) FROM tdb_serverside_spell').fetchone(), (2,))
            self.assertEqual(connection.execute('SELECT count(*) FROM tdb_serverside_spell_effect').fetchone(), (2,))
            self.assertEqual(connection.execute('SELECT Id,DurationIndex,RangeIndex,SpellName FROM tdb_serverside_spell ORDER BY Id').fetchall(),
                             [(119297, 28, 5, 'Inferno Charge'), (119298, 0, 5, 'Inferno Charge')])
            self.assertEqual(connection.execute('SELECT SpellID,Effect,EffectMiscValue1,EffectMiscValue2,ImplicitTarget1 FROM tdb_serverside_spell_effect ORDER BY SpellID').fetchall(),
                             [(119297, 28, 61409, 491, 53), (119298, 96, 0, 0, 46)])
            self.assertEqual(connection.execute('SELECT EffectTriggerSpell FROM tdb_serverside_spell_effect WHERE SpellID=119298').fetchone(), (119299,))
            self.assertEqual(connection.execute('SELECT value FROM unrelated').fetchone(), ('retained',))
            connection.row_factory = sqlite3.Row
            expected = json.loads(FIXTURE.read_text())['rows']
            for table, source_rows in expected.items():
                id_column = 'Id' if table == 'serverside_spell' else 'SpellID'
                actual = [dict(row) for row in connection.execute(f'SELECT * FROM tdb_{table} ORDER BY {id_column}')]
                self.assertEqual(actual, source_rows, 'Every actual source field must survive unchanged')

    def test_missing_authored_spell_refuses_without_partial_tables(self):
        write_dump(self.dump, missing=('serverside_spell', 119298))
        with self.assertRaisesRegex(ValueError, '119298'):
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
