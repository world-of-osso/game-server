"""Aldryr binding-only producer; preserve source/native population."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 210271


class AldryrBindingTests(unittest.TestCase):
    def test_binding_preserves_source_and_native_data(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'aldryr.sqlite')) as db:
                    source.backup(db)
                    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')
                               if row[1] != 'ScriptName']
                    queries = ['SELECT ' + ','.join(columns) + ' FROM content_creature_template WHERE entry=210271',
                               'SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('tdb_creature_template', 'entry=210271'),
                        ('tdb_creature', 'map=2661'),
                        ('content_creature_template_model', 'CreatureID=210271'),
                        ('content_creature_template_addon', 'entry=210271'),
                        ('content_creature_template_difficulty', 'entry=210271'),
                        ('raid_encounter_creature', 'creature_entry!=210271'),
                        ('spell_effect', 'SpellID IN(432229,432179,432198,442525,454459)'),
                        ('spell_misc', 'SpellID IN(432229,432179,432198,442525,454459)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_aldryr(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT AIName,ScriptName,unit_flags FROM content_creature_template WHERE entry=210271').fetchone(),
                                         ('', 'littlewigs_aldryr', 64))
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=210271').fetchall(),
                                         [(2661, 210271, 2900, 0, 2586, 5619, 116429)])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=210271")
                    with self.assertRaisesRegex(ValueError, 'Aldryr runtime'):
                        encounter_summons.import_aldryr(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=210271').fetchone(),
                                     ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
