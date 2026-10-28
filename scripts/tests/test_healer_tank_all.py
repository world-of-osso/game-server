import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'healer_tank_replay.py'


class AllReplayTest(unittest.TestCase):
    def test_all_runs_all_specs_and_writes_details(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for spec in ('66', '73', '104', '250', '268', '581'):
                (root / f'{spec}.simc-trace.log').write_text(spec)
            executable = root / 'replay'
            executable.write_text('''#!/usr/bin/env python3
import json, os
spec = os.environ['HEALER_TANK_SPEC']
trace_spec = '66' if os.environ['HEALER_TANK_MODE'] == 'healer' else spec
assert open(os.environ['HEALER_TANK_TIMELINE']).read() == trace_spec
assert os.environ['HEALER_TANK_DURATION'] == '300'
assert os.environ['HEALER_TANK_SEED'] == '70448'
if os.environ['HEALER_TANK_MODE'] == 'healer':
    report = {'spec': spec, 'metrics': {'effective_hps': 10, 'raw_hps': 20, 'overheal_pct': 50, 'mana_min': 30, 'mana_curve': [{'current': 40}], 'first_death_seconds': None, 'tank_health_min': 60, 'idle_gcd_pct': 12.5, 'spell_healing': {'123': {'effective_hps': 10}}}}
else:
    healed = os.environ['HEALER_TANK_EXTERNAL_HEALING'] == '1'
    report = {'spec': spec, 'mitigation_pct': 50, 'self_heal_per_second': 10, 'absorb_per_second': 5, 'time_to_death': None if healed else 42, 'active_mitigation_uptime': 0.5, 'largest_spike': 900, 'self_healing_by_spell': {'123': 100}}
print('HEALER_TANK_REPORT=' + json.dumps(report))
''')
            executable.chmod(0o755)
            output = root / 'results'
            result = subprocess.run(['python3', str(SCRIPT), '--all', '--trace-dir', str(root),
                                     '--profile-dir', str(root), '--executable', str(executable),
                                     '--output-dir', str(output), '--jobs', '3'],
                                    text=True, capture_output=True, env=os.environ)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('| 105 | 10.00 | 20.00 | 50.00 | 30.00 | 40.00 | N/A | 60.00 | 12.50 |', result.stdout)
            self.assertIn('| 581 | 50.00 | 10.00 | 5.00 | 42.00 | 50.00 | 900.00 |', result.stdout)
            self.assertEqual(len(list(output.glob('*.json'))), 19)
            self.assertEqual(json.loads((output / 'healer-105-unhealed.json').read_text())['metrics']['spell_healing']['123']['effective_hps'], 10)
            self.assertEqual(json.loads((output / 'tank-581-unhealed.json').read_text())['time_to_death'], 42)


if __name__ == '__main__':
    unittest.main()
