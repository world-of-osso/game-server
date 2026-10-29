import importlib.util
import sqlite3
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('journal', Path(__file__).with_name('log_population_journal.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class JournalTest(unittest.TestCase):
    def test_matches_observed_actor_by_name_and_authored_display_without_claiming_boss(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'JournalEncounter.csv').write_text('ID,DungeonEncounterID\n20,2609\n')
            (root / 'JournalEncounterCreature.csv').write_text(
                'ID,JournalEncounterID,Name_lang,CreatureDisplayInfoID,Description_lang\n'
                '31,20,Other,56,"two\nlines"\n30,20,Observed Add,55,\n')
            db = root / 'world.db'
            with sqlite3.connect(db) as conn:
                conn.execute('CREATE TABLE tdb_creature_template_model(CreatureID,CreatureDisplayID)')
                conn.execute('INSERT INTO tdb_creature_template_model VALUES(123,55)')
            runs = [{'map': 2521, 'encounters': [{'id': 2609, 'name': 'Boss', 'success': True,
                'known_boss_npc_ids': [], 'units': {'g': {'npc_id': 123, 'name': 'Observed Add'}},
                'casts': [{'guid': 'g', 'spell_id': 456, 'event': 'SPELL_CAST_START',
                           'seconds_from_pull': 2.0, 'known_boss': False}]}]}]
            module.annotate(runs, module.journal_actors(root), db)
        encounter = runs[0]['encounters'][0]
        self.assertEqual(encounter['journal_actor_evidence'][0]['candidate_npc_ids'], [123])
        self.assertEqual(encounter['journal_actor_evidence'][0]['actor_line'], 4)
        self.assertTrue(encounter['casts'][0]['journal_actor'])
        self.assertFalse(encounter['casts'][0]['known_boss'])
        summary = module.census.summarize_encounters(runs)
        self.assertEqual(summary[0]['spells'][0]['actor_identification'], 'journal_actor_candidate')


if __name__ == '__main__':
    unittest.main()
