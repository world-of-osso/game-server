"""Benk source binding; preserve live120557 rather than journal preview117818."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 218002


class BenkBindingTests(unittest.TestCase):
    def test_binding_preserves_actual_model_source_and_native_data(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'benk.sqlite')) as db:
                    source.backup(db)
                    self.assertEqual(db.execute('SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=218002 ORDER BY Idx').fetchall(), [(120557,)])
                    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')
                               if row[1] != 'ScriptName']
                    queries = ['SELECT ' + ','.join(columns) + ' FROM content_creature_template WHERE entry=218002',
                               'SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('tdb_creature_template', 'entry IN(218002,218016)'),
                        ('tdb_creature', 'map=2661'),
                        ('content_creature_template_model', 'CreatureID=218002'),
                        ('content_creature_template_addon', 'entry=218002'),
                        ('content_creature_template_difficulty', 'entry=218002'),
                        ('raid_encounter_creature', 'creature_entry!=218002'),
                        ('spell_effect', 'SpellID IN(438025,439524,440134,440141,438971)'),
                        ('spell_misc', 'SpellID IN(438025,439524,440134,440141,438971)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_benk(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT AIName,ScriptName,unit_flags FROM content_creature_template WHERE entry=218002').fetchone(),
                                         ('', 'littlewigs_benk', 64))
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=218002').fetchall(),
                                         [(2661, 218002, 2931, 3, 2588, 5697, 117818)])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=218002")
                    with self.assertRaisesRegex(ValueError, 'Benk runtime'):
                        encounter_summons.import_benk(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=218002').fetchone(),
                                     ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
