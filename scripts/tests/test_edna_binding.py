"""TC EDNA binding preserves source immunity, inherited StringIds and population."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
ENTRY = 210108
BINDING = (2652, ENTRY, 2854, 0, 2572, 5586, 120054)


class EdnaBindingTests(unittest.TestCase):
    def test_binding_preserves_source_population_immunity_and_native_data(self):
        parent = Path('/var/tmp/osso')
        parent.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as directory:
            with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'edna.sqlite')) as db:
                    source.backup(db)
                    queries = ['SELECT * FROM content_creature ORDER BY guid']
                    for table, where in [
                        ('content_creature_template', 'entry IN(210108,210109)'),
                        ('tdb_creature_template', 'entry IN(210108,210109)'),
                        ('tdb_creature', 'id IN(210108,210109)'),
                        ('content_creature_template_model', 'CreatureID IN(210108,210109)'),
                        ('content_creature_template_addon', 'entry IN(210108,210109)'),
                        ('content_creature_template_difficulty', 'entry IN(210108,210109)'),
                        ('raid_encounter_creature', 'creature_entry!=210108'),
                        ('spell_effect', 'SpellID IN(45313,424903,452738,424888)'),
                        ('spell_misc', 'SpellID IN(45313,424903,452738,424888)'),
                        ('spell_power', 'SpellID IN(45313,424903,452738,424888)'),
                    ]:
                        queries.append(f'SELECT * FROM {table} WHERE {where}')
                    snapshots = {query: db.execute(query).fetchall() for query in queries}
                    db.execute('DELETE FROM raid_encounter_creature WHERE creature_entry=?', (ENTRY,))
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_edna(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=?', (ENTRY,)).fetchall(), [BINDING])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET unit_flags=64 WHERE entry=?", (ENTRY,))
                    with self.assertRaisesRegex(ValueError, 'EDNA runtime'):
                        encounter_summons.import_edna(db)
                    self.assertEqual(db.execute('SELECT unit_flags FROM content_creature_template WHERE entry=?', (ENTRY,)).fetchone(), (64,))


if __name__ == '__main__':
    unittest.main()
