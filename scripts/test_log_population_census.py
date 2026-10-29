import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('log_population_census', Path(__file__).with_name('log_population_census.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
census = module.census


class CensusTest(unittest.TestCase):
    def test_advanced_position_belongs_to_described_unit(self):
        fields = ['SPELL_DAMAGE', 'Creature-0-1-2521-42-123-ABCD', 'Boss', '0', '0',
                  'Creature-0-1-2521-42-456-DCBA', 'Add', '0', '0', '1', 'Hit', '1',
                  'Creature-0-1-2521-42-456-DCBA', '0', '100', '100', '0', '0', '0',
                  '0', '0', '0', '0', '0', '0', '0', '1433.58', '-155.52', '2095',
                  '5.1539', '90', '1']
        pos = module.advanced_position(fields)
        self.assertEqual(pos['guid'], 'Creature-0-1-2521-42-456-DCBA')
        self.assertEqual((pos['x'], pos['y'], pos['ui_map_id'], pos['level']),
                         (1433.58, -155.52, 2095, 90))
        fields[12] = 'Creature-0-1-2521-42-789-AAAA'
        self.assertIsNone(module.advanced_position(fields))
        fields[0] = 'SPELL_AURA_APPLIED'
        self.assertIsNone(module.advanced_position(fields))

    def test_runs_creatures_and_summon_destinations(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'combat.txt'
            path.write_text(
                '1  CHALLENGE_MODE_START,"Ruby Life Pools",2521,399,15,[]\n'
                '2  SPELL_DAMAGE,Player-1,"Player",0,0,Creature-0-1-2521-42-123-ABCD,"Mob",0,0,1,"Hit",1\n'
                '3  SPELL_SUMMON,Creature-0-1-2521-42-123-ABCD,"Mob",0,0,Creature-0-1-2521-42-456-DCBA,"Add",0,0,2,"Summon",1\n'
                '4  CHALLENGE_MODE_END,"Ruby Life Pools",2521\n'
                '5  SPELL_DAMAGE,Player-1,"Player",0,0,Creature-0-1-2521-42-789-FFFF,"Excluded",0,0,1,"Hit",1\n'
            )
            runs = census([path], {2521})
        self.assertEqual(len(runs), 1)
        creatures = runs[0]['creatures']
        self.assertEqual(len(creatures), 2)
        self.assertEqual(creatures['Creature-0-1-2521-42-123-ABCD']['instance'], '42')
        self.assertEqual(creatures['Creature-0-1-2521-42-123-ABCD']['first_seen'], '2')
        self.assertTrue(creatures['Creature-0-1-2521-42-456-DCBA']['summoned'])
        self.assertFalse(creatures['Creature-0-1-2521-42-123-ABCD']['summoned'])


if __name__ == '__main__':
    unittest.main()
