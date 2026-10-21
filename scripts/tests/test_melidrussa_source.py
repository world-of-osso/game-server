"""TC/70448 primary plus explicit local Retail-Lua binding; independent copy."""
import sqlite3
import tempfile
import unittest
from pathlib import Path

import encounter_cpp_bindings
from encounter_summons import import_melidrussa

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'data/world.db'
TC = Path('/home/osso/.worktrees/logs/achievements-criteria-2026-10-12/tc-source-a352b1fa/TrinityCore-a352b1fa/src/server/scripts')
DB2 = Path('/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.5.70448')


class MelidrussaSourceTests(unittest.TestCase):
    def test_authentic_primary_local_binding_preserves_all_other_inputs(self):
        primary, sources = encounter_cpp_bindings.PROFILES['melidrussa']
        encounter_cpp_bindings.validate_source_files(TC, sources)
        encounter_cpp_bindings.validate_journal(DB2 / 'JournalEncounter.csv', DB2 / 'JournalEncounterCreature.csv', primary)
        fixtures = ROOT / 'target/encounter-fixtures'
        fixtures.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=fixtures) as directory:
            with sqlite3.connect(SOURCE.resolve().as_uri() + '?mode=ro', uri=True) as source, sqlite3.connect(Path(directory) / 'melidrussa.sqlite') as db:
                source.backup(db)
                columns = [row[1] for row in db.execute('PRAGMA table_info(content_creature_template)')]
                boss = dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=188252').fetchone()))
                queries = ["SELECT * FROM content_creature_template_difficulty WHERE entry=188252 ORDER BY DifficultyID",
                           "SELECT * FROM content_creature_template_model WHERE CreatureID=188252",
                           "SELECT * FROM content_creature_template_addon WHERE entry=188252",
                           "SELECT count(*) FROM content_creature",
                           "SELECT * FROM raid_encounter_creature WHERE creature_entry!=188252 ORDER BY map_id,creature_entry"]
                before = [db.execute(query).fetchall() for query in queries]
                self.assertEqual(import_melidrussa(db), [188252])
                self.assertEqual(db.execute('SELECT * FROM raid_encounter_creature WHERE map_id=2521 AND creature_entry=188252').fetchone(),
                                 (2521,188252,2609,1,2488,5358,106891))
                self.assertEqual(dict(zip(columns, db.execute('SELECT * FROM content_creature_template WHERE entry=188252').fetchone())),
                                 dict(boss, ScriptName='boss_melidrussa_chillworn'))
                self.assertEqual([db.execute(query).fetchall() for query in queries], before)
                import_melidrussa(db)
                self.assertEqual(db.execute('SELECT count(*) FROM raid_encounter_creature WHERE creature_entry=188252').fetchone()[0], 1)
                db.execute('UPDATE spell_effect SET EffectTriggerSpell=0 WHERE SpellID=1307297 AND DifficultyID=0 AND EffectIndex=0')
                with self.assertRaisesRegex(ValueError, 'native wrapper1307297 child changed'):
                    import_melidrussa(db)


if __name__ == '__main__':
    unittest.main()
