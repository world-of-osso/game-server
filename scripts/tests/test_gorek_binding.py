"""Real canonical snapshot; binding-only import never mutates source actor data."""
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'


class GorekBindingTests(unittest.TestCase):
    def test_gorek_preservation_idempotence_and_conflict(self):
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory) / 'copy.sqlite')) as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                actor = list(db.execute('SELECT * FROM content_creature_template WHERE entry=189478').fetchone())
                actor[columns.index('ScriptName')] = 'boss_gorek'
                preserved = [
                    'SELECT * FROM content_creature WHERE map=2519 ORDER BY guid',
                    'SELECT * FROM content_creature_template_difficulty WHERE entry=189478',
                    'SELECT * FROM content_creature_template_model WHERE CreatureID=189478',
                    'SELECT * FROM raid_encounter_creature WHERE creature_entry<>189478 ORDER BY map_id,creature_entry',
                    'SELECT * FROM spell_effect WHERE SpellID IN(374635,374842,374533,374969) ORDER BY SpellID,DifficultyID,EffectIndex',
                ]
                before = [db.execute(query).fetchall() for query in preserved]
                for _ in range(2):
                    self.assertEqual(encounter_summons.import_gorek(db), [189478])
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=189478').fetchone(), tuple(actor))
                    self.assertEqual([db.execute(query).fetchall() for query in preserved], before)
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2519 AND creature_entry=189478').fetchone(), (2612,2,2489,5363,107189))
                db.execute("UPDATE content_creature_template SET ScriptName='foreign' WHERE entry=189478")
                with self.assertRaisesRegex(ValueError, 'binding'):
                    encounter_summons.import_gorek(db)
                self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=189478').fetchone(), ('foreign',))
