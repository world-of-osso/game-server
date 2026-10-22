"""TC Skarmorak primary import on a fresh independent canonical copy."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 210156
BINDING = (2652, ENTRY, 2880, 1, 2579, 5608, 118714)


class SkarmorakBindingTests(unittest.TestCase):
    def test_binding_preserves_source_population_and_native_data(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'skarmorak.sqlite')) as db:
                    source.backup(db)
                    queries = ['SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('content_creature_template', 'entry=210156'),
                        ('tdb_creature_template', 'entry=210156'),
                        ('tdb_creature', 'id=210156'),
                        ('content_creature_template_model', 'CreatureID=210156'),
                        ('content_creature_template_addon', 'entry=210156'),
                        ('content_creature_template_difficulty', 'entry=210156'),
                        ('raid_encounter_creature', 'creature_entry!=210156'),
                        ('spell_effect', 'SpellID IN(422233,423538,439749,429976,423200,423324)'),
                        ('spell_misc', 'SpellID IN(422233,423538,439749,429976,423200,423324)'),
                        ('spell_power', 'SpellID IN(422233,423538,439749,429976,423200,423324)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    db.execute('DELETE FROM raid_encounter_creature WHERE creature_entry=?', (ENTRY,))
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_skarmorak(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=?', (ENTRY,)).fetchall(), [BINDING])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=?", (ENTRY,))
                    with self.assertRaisesRegex(ValueError, 'Skarmorak runtime'):
                        encounter_summons.import_skarmorak(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=?', (ENTRY,)).fetchone(), ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
