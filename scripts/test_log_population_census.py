import tempfile
import unittest
from pathlib import Path

from log_population_census import census


class CensusTest(unittest.TestCase):
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
