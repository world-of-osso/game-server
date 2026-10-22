"""Dailcry binding-only producer; no invented difficulty205 population."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 207946


class DailcryBindingTests(unittest.TestCase):
    def test_binding_preserves_source_and_native_data(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'dailcry.sqlite')) as db:
                    source.backup(db)
                    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')
                               if row[1] != 'ScriptName']
                    queries = ['SELECT ' + ','.join(columns) + ' FROM content_creature_template WHERE entry=207946',
                               'SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('tdb_creature_template', 'entry IN(207946,211289,211290,211291,207939,207940)'),
                        ('tdb_creature', 'map=2649'),
                        ('content_creature_template_model', 'CreatureID=207946'),
                        ('content_creature_template_addon', 'entry=207946'),
                        ('content_creature_template_difficulty', 'entry=207946'),
                        ('raid_encounter_creature', 'creature_entry!=207946'),
                        ('spell_effect', 'SpellID IN(424414,1238780,424419,447439,447443)'),
                        ('spell_misc', 'SpellID IN(424414,1238780,424419,447439,447443)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_dailcry(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT AIName,ScriptName,unit_flags FROM content_creature_template WHERE entry=207946').fetchone(),
                                         ('', 'littlewigs_dailcry', 64))
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=207946').fetchall(),
                                         [(2649, 207946, 2847, 1, 2571, 5714, 118879)])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=207946")
                    with self.assertRaisesRegex(ValueError, 'Dailcry runtime'):
                        encounter_summons.import_dailcry(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=207946').fetchone(),
                                     ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
