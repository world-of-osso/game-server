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
        timer = result['timers']['190484:381525:SPELL_CAST_START']
        self.assertEqual(timer['first']['median'], 2)
        self.assertEqual(timer['repeat']['median'], 3)


if __name__ == '__main__':
    unittest.main()
