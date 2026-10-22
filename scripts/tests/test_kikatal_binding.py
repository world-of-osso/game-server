"""Ki'katal binding-only producer on an independent real-source copy."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 215407


class KikatalBindingTests(unittest.TestCase):
    def test_binding_preserves_source_and_native_data(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'kikatal.sqlite')) as db:
                    source.backup(db)
                    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')
                               if row[1] != 'ScriptName']
                    queries = ['SELECT ' + ','.join(columns) + ' FROM content_creature_template WHERE entry=215407',
                               'SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('tdb_creature_template', 'entry=215407'),
                        ('tdb_creature', 'id=215407'),
                        ('content_creature_template_model', 'CreatureID=215407'),
                        ('content_creature_template_addon', 'entry=215407'),
                        ('content_creature_template_difficulty', 'entry=215407'),
                        ('raid_encounter_creature', 'creature_entry!=215407'),
                        ('spell_effect', 'SpellID IN(432117,432130,432227,432031,447504)'),
                        ('spell_misc', 'SpellID IN(432117,432130,432227,432031,447504)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_kikatal(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT AIName,ScriptName,unit_flags FROM content_creature_template WHERE entry=215407').fetchone(),
                                         ('', 'littlewigs_kikatal', 64))
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=215407').fetchall(),
                                         [(2660, 215407, 2901, 0, 2585, 5632, 117473)])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=215407")
                    with self.assertRaisesRegex(ValueError, 'Kikatal runtime'):
                        encounter_summons.import_kikatal(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=215407').fetchone(),
                                     ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
