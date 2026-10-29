import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'ruby_log_extract.py'
spec = importlib.util.spec_from_file_location('ruby_log_extract', SCRIPT)
ruby = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ruby)


class RubyLogExtractTests(unittest.TestCase):
    def test_only_successful_encounter_and_selected_actor_casts(self):
        lines = [
            '1/1/2026 00:00:00.000-5  ENCOUNTER_START,2623,x,8,5,2521',
            '1/1/2026 00:00:02.000-5  SPELL_CAST_START,Vehicle-0-0-0-0-190484-a,x,0,0,0,0,0,0,381525,x',
            '1/1/2026 00:00:03.000-5  SPELL_CAST_START,Creature-0-0-0-0-999999-a,x,0,0,0,0,0,0,381525,x',
            '1/1/2026 00:00:05.000-5  SPELL_CAST_START,Vehicle-0-0-0-0-190484-a,x,0,0,0,0,0,0,381525,x',
            '1/1/2026 00:00:05.500-5  SPELL_AURA_APPLIED,Creature-0-0-0-0-190485-a,x,0,0,Creature-0-0-0-0-190485-a,x,0,0,181089,x',
            '1/1/2026 00:00:06.000-5  ENCOUNTER_END,2623,x,8,5,1',
            '1/1/2026 00:01:00.000-5  ENCOUNTER_START,2623,x,8,5,2521',
            '1/1/2026 00:01:01.000-5  SPELL_CAST_START,Vehicle-0-0-0-0-190484-a,x,0,0,0,0,0,0,381525,x',
            '1/1/2026 00:01:02.000-5  ENCOUNTER_END,2623,x,8,5,0',
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'log.txt'
            path.write_text('\n'.join(lines))
            result = ruby.extract([path])
        self.assertEqual(result['kills']['2623'], 1)
        self.assertEqual(result['stage_events'][0]['aura_181089'], 5.5)
        timer = result['timers']['190484:381525:SPELL_CAST_START']
        self.assertEqual(timer['first']['median'], 2)
        self.assertEqual(timer['repeat']['median'], 3)

    def test_advanced_position_and_health_are_observed_not_interpreted_as_altitude(self):
        advanced = ['Vehicle-0-0-0-0-190484-a', 'owner', '49', '100'] + ['0'] * 10 + ['1787', '28', '2094', '4.2']
        record = ['SPELL_DAMAGE', 'source', 'name', '0', '0', 'target', 'name', '0', '0', '1', 'spell', '1'] + advanced
        lines = [
            '1/1/2026 00:00:00.000-5  ENCOUNTER_START,2623,x,8,5,2521',
            '1/1/2026 00:00:01.000-5  ' + ','.join(record),
            '1/1/2026 00:00:02.000-5  ENCOUNTER_END,2623,x,8,5,1',
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'log.txt'
            path.write_text('\n'.join(lines))
            result = ruby.extract([path])
        self.assertEqual(result['positions']['190484'], [{'x': 1787, 'y': 28, 'logged_map_id': 2094, 'orientation': 4.2}])
        self.assertEqual(result['stage_events'][0]['first_observed_health_50'], {190484: 1.0})


if __name__ == '__main__':
    unittest.main()
