"""Preserving source-backed bindings; no canonical writes or spell/stat changes."""
from contextlib import closing
from pathlib import Path
import sqlite3
import tempfile
import unittest

import encounter_summons

SOURCE = Path(__file__).resolve().parents[2] / 'data/world.db'
PROFILES = (
    dict(entry=189340, importer='import_chargath', script='boss_chargath',
         binding=(2613, 3, 2490, 5365, 108248), spells='373742,373733,373735,373424,375056'),
    dict(entry=189901, importer='import_sargha', script='boss_sargha',
         binding=(2611, 1, 2501, 5394, 107029), spells='377204,377017,377018,377473,377477,376780'),
)


class NeltharusInitialBindingTests(unittest.TestCase):
    def assert_preserved(self, profile):
        importer = getattr(encounter_summons, profile['importer'])
        entry = profile['entry']
        with tempfile.TemporaryDirectory() as directory:
            with closing(sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True)) as source, closing(sqlite3.connect(Path(directory) / 'copy.sqlite')) as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                actor = list(db.execute('SELECT * FROM content_creature_template WHERE entry=?', (entry,)).fetchone())
                actor[columns.index('ScriptName')] = profile['script']
                preserved = [
                    'SELECT * FROM content_creature WHERE map=2519 ORDER BY guid',
                    f'SELECT * FROM creature_template WHERE entry={entry}',
                    f'SELECT * FROM content_creature_template_difficulty WHERE entry={entry}',
                    f'SELECT * FROM content_creature_template_model WHERE CreatureID={entry}',
                    f'SELECT * FROM raid_encounter_creature WHERE creature_entry<>{entry} ORDER BY map_id,creature_entry',
                    f'SELECT * FROM spell_effect WHERE SpellID IN({profile["spells"]}) ORDER BY SpellID,DifficultyID,EffectIndex',
                ]
                before = [db.execute(query).fetchall() for query in preserved]
                for _ in range(2):
                    self.assertEqual(importer(db), [entry])
                    self.assertEqual(db.execute('SELECT * FROM content_creature_template WHERE entry=?', (entry,)).fetchone(), tuple(actor))
                    self.assertEqual([db.execute(query).fetchall() for query in preserved], before)
                    self.assertEqual(db.execute('SELECT encounter_id,bit,journal_encounter_id,journal_creature_id,display_id FROM raid_encounter_creature WHERE map_id=2519 AND creature_entry=?', (entry,)).fetchone(), profile['binding'])
                db.execute("UPDATE content_creature_template SET ScriptName='foreign' WHERE entry=?", (entry,))
                with self.assertRaisesRegex(ValueError, 'binding'):
                    importer(db)
                self.assertEqual(db.execute('SELECT ScriptName FROM content_creature_template WHERE entry=?', (entry,)).fetchone(), ('foreign',))

    def test_chargath_preservation_idempotence_and_conflict(self):
        self.assert_preserved(PROFILES[0])

    def test_sargha_preservation_idempotence_and_conflict(self):
        self.assert_preserved(PROFILES[1])
