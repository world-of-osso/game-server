"""Current Retail Kokia primary/local binding preserves native world inputs."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

import encounter_cpp_bindings
from encounter_summons import import_kokia

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'
TC = Path('/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/src/server/scripts')
DB2 = Path('/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.5.70448')


class KokiaSourceTests(unittest.TestCase):
    def test_actual_primary_only_binding_and_source_effect_drift_guard(self):
        primary, sources = encounter_cpp_bindings.PROFILES['kokia']
        encounter_cpp_bindings.validate_source_files(TC, sources)
        encounter_cpp_bindings.validate_journal(DB2 / 'JournalEncounter.csv', DB2 / 'JournalEncounterCreature.csv', primary)
        fixtures = ROOT / 'target/encounter-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'kokia.sqlite') as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                boss = dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=189232').fetchone()))
                queries = ['SELECT * FROM content_creature_template_difficulty WHERE entry=189232 ORDER BY DifficultyID',
                           'SELECT * FROM content_creature_template_model WHERE CreatureID=189232',
                           'SELECT * FROM content_creature_template_addon WHERE entry=189232',
                           'SELECT count(*) FROM content_creature',
                           'SELECT * FROM raid_encounter_creature WHERE creature_entry!=189232 ORDER BY map_id,creature_entry']
                before = [db.execute(query).fetchall() for query in queries]
                self.assertEqual(import_kokia(db), [189232])
                self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=189232').fetchone(),
                                 (2521,189232,2606,0,2485,5354,106851))
                self.assertEqual(dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=189232').fetchone())),
                                 dict(boss, ScriptName='boss_kokia_blazehoof'))
                self.assertEqual([db.execute(query).fetchall() for query in queries], before)
                import_kokia(db)
                self.assertEqual(db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=189232').fetchone()[0], 1)
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=372110 AND DifficultyID=0 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native current effect/child372110 changed'):
                    import_kokia(db)


if __name__ == '__main__':
    unittest.main()
