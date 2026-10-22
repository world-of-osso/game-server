"""Anub'zekt binding-only producer on an independent real-source copy."""
import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

import encounter_summons

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'
ENTRY = 215405


class AnubzektBindingTests(unittest.TestCase):
    def test_binding_preserves_source_actor_native_spells_and_population(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / 'target') as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True)) as source:
                with closing(sqlite3.connect(Path(directory) / 'anubzekt.sqlite')) as db:
                    source.backup(db)
                    columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')
                               if row[1] != 'ScriptName']
                    actor_query = 'SELECT ' + ','.join(columns) + ' FROM content_creature_template WHERE entry=215405'
                    snapshots = {
                        actor_query: db.execute(actor_query).fetchall(),
                        'SELECT * FROM content_creature ORDER BY guid': db.execute('SELECT * FROM content_creature ORDER BY guid').fetchall(),
                    }
                    for table, where in [
                        ('tdb_creature_template', 'entry=215405'),
                        ('tdb_creature', 'id=215405'),
                        ('content_creature_template_model', 'CreatureID=215405'),
                        ('content_creature_template_addon', 'entry=215405'),
                        ('content_creature_template_difficulty', 'entry=215405'),
                        ('raid_encounter_creature', 'creature_entry!=215405'),
                        ('spell_effect', 'SpellID IN(435012,439506,433740,433766,434408)'),
                        ('spell_misc', 'SpellID IN(435012,439506,433740,433766,434408)'),
                    ]:
                        query = f'SELECT * FROM {table} WHERE {where}'
                        snapshots[query] = db.execute(query).fetchall()
                    for _ in range(2):
                        self.assertEqual(encounter_summons.import_anubzekt(db), [ENTRY])
                        self.assertEqual(db.execute('SELECT AIName,ScriptName,unit_flags FROM content_creature_template WHERE entry=215405').fetchone(),
                                         ('', 'littlewigs_anubzekt', 64))
                        self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE creature_entry=215405').fetchall(),
                                         [(2660,215405,2906,1,2584,5726,115723)])
                        for query, expected in snapshots.items():
                            self.assertEqual(db.execute(query).fetchall(), expected, query)
                    db.execute("UPDATE content_creature_template SET ScriptName='conflicting_script' WHERE entry=215405")
                    with self.assertRaisesRegex(ValueError, 'Anubzekt runtime'):
                        encounter_summons.import_anubzekt(db)
                    self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=215405').fetchone(),
                                     ('conflicting_script',))


if __name__ == '__main__':
    unittest.main()
