"""Real canonical snapshot; binding-only import never mutates source actor data."""
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class IrideusBindingTests(unittest.TestCase):
    def test_irideus_preservation_idempotence_and_conflict(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory) / 'copy.sqlite')) as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                actor = list(db.execute('SELECT * FROM content_creature_template WHERE entry=189719').fetchone())
                actor[columns.index('ScriptName')] = 'boss_irideus'
                preserved = [
                    'SELECT * FROM content_creature WHERE map=2527 ORDER BY guid',
                    'SELECT * FROM content_creature_template_difficulty WHERE entry=189719',
                    'SELECT * FROM content_creature_template_model WHERE CreatureID=189719',
                    'SELECT * FROM raid_encounter_creature WHERE creature_entry<>189719 ORDER BY map_id,creature_entry',
                    'SELECT * FROM spell_effect WHERE SpellID IN(384524,384014,384015,389179,384351,383840,389446) ORDER BY SpellID,DifficultyID,EffectIndex',
                ]
                before = [db.execute(query).fetchall() for query in preserved]
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_irideus(db), [189719])
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=189719').fetchone(), tuple(actor))
                    self.assertEqual([db.execute(query).fetchall() for query in preserved], before)
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2527 AND creature_entry=189719').fetchone(), (2615,0,2504,5395,106801))
                db.execute("UPDATE content_creature_template SET ScriptName='foreign' WHERE entry=189719")
                with self.assertRaisesRegex(ValueError, 'binding'):
                    encounter_summons.import_irideus(db)
                self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=189719').fetchone(), ('foreign',))
