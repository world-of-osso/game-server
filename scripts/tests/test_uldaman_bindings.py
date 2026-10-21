"""Actual TDB/70448 copy; local Lua bindings cannot rewrite source actors."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class UldamanBindingsTests(unittest.TestCase):
    def test_lost_dwarves_source_rows_and_shared_credit_without_new_stats_or_spawns(self):
        with tempfile.TemporaryDirectory() as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri()+'?mode=ro', uri=True) as source, sqlite3.connect(Path(directory)/'copy.sqlite') as db:
                source.backup(db)
                population = db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall()
                stats = db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN(184580,184581,184582)').fetchall()
                before = db.execute('SELECT * FROM content_creature_template WHERE entry IN(184580,184581,184582) ORDER BY entry').fetchall()
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                script = columns.index('ScriptName')
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_lost_dwarves(db), [184580,184581,184582])
                    expected = [list(row) for row in before]
                    for row in expected:
                        row[script] = 'boss_lost_dwarves'
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry IN(184580,184581,184582) ORDER BY entry').fetchall(), [tuple(row) for row in expected])
                    self.assertEqual(db.execute('SELECT * FROM content_creature WHERE map=2451').fetchall(), population)
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template_difficulty WHERE entry IN(184580,184581,184582)').fetchall(), stats)
                    self.assertEqual(db.execute('SELECT creature_entry,encounter_id,bit,journal_creature_id FROM raid_encounter_creature WHERE map_id=2451 AND creature_entry IN(184580,184581,184582) ORDER BY creature_entry').fetchall(), [(184580,2555,0,5349),(184581,2555,0,5348),(184582,2555,0,5347)])
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=369791 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native'):
                    encounter_summons.import_lost_dwarves(db)
