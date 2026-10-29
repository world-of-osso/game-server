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

    def test_pressure_sweep_ranks_from_first_failing_grid_scale(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            specs = ('66', '73', '104', '250', '268', '581')
            for spec in specs:
                (root / f'{spec}.simc-trace.log').write_text(spec)
            executable = root / 'replay'
            executable.write_text('''#!/usr/bin/env python3
import json, os
spec = os.environ['HEALER_TANK_SPEC']
scale = float(os.environ['HEALER_TANK_PRESSURE_SCALE'])
seed = int(os.environ['HEALER_TANK_SEED'])
assert os.environ['HEALER_TANK_EXTERNAL_HEALING'] == '0'
threshold = {'66': 1.0, '73': 1.25, '104': 1.5, '250': 2.0, '268': 2.5, '581': 3.0}[spec]
# 250 survives again at 3.0: a later recovery must not raise its survival scale.
survives = (scale <= threshold or spec == '250' and scale == 3.0) and seed < 70453
report = {'time_to_death': None if survives else 100.0, 'duration_seconds': 300.0 if survives else 100.0,
          'damage_taken': (3000.0 if survives else 1000.0) * scale, 'self_heal_per_second': 3.0, 'pressure_scale': scale}
print('HEALER_TANK_REPORT=' + json.dumps(report))
''')
            executable.chmod(0o755)
            output = root / 'results'
            result = subprocess.run(['python3', str(SCRIPT), '--pressure-sweep', '--trace-dir', str(root),
                                     '--profile-dir', str(root), '--executable', str(executable),
                                     '--output-dir', str(output), '--jobs', '8'], text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            summary = json.loads((output / 'summary.json').read_text())
            self.assertEqual(len(summary['seeds']), 10)
            self.assertEqual(len(list(output.glob('tank-*.json'))), 6 * 17 * 10)
            ranking = [(row['spec'], row['survival_scale']) for row in summary['results']]
            self.assertEqual(ranking, [('581', 3.0), ('268', 2.5), ('250', 2.0), ('104', 1.5), ('73', 1.25), ('66', 1.0)])
            self.assertEqual(summary['results'][0]['net_damage_per_second'], 7.0)
            self.assertIn('| 3.00 | 0/10, 100 s | 0/10, 100 s | 0/10, 100 s | 5/10, 200 s | 0/10, 100 s | 5/10, 200 s |',
                          result.stdout)
            self.assertIn('| 581 | 3.00 | 7.00 |', result.stdout)

    def test_multiseed_aggregates_real_cli_outputs_and_keeps_seed_details(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for spec in ('66', '73', '104', '250', '268', '581'):
                (root / f'{spec}.simc-trace.log').write_text(spec)
            executable = root / 'replay'
            executable.write_text('''#!/usr/bin/env python3
import json, os
spec = os.environ['HEALER_TANK_SPEC']
seed = int(os.environ['HEALER_TANK_SEED'])
if os.environ['HEALER_TANK_MODE'] == 'healer':
    assert seed == 70448
    report = {'metrics': {'effective_hps': 10, 'raw_hps': 20, 'overheal_pct': 50, 'mana_min': 30, 'mana_curve': [{'current': 40}], 'first_death_seconds': None, 'tank_health_min': 60, 'idle_gcd_pct': 12}}
else:
    assert seed in (70448, 70449)
    healed = os.environ['HEALER_TANK_EXTERNAL_HEALING'] == '1'
    report = {'spec': spec, 'external_healing': healed, 'mitigation_pct': 40 + 20 * (seed - 70448), 'time_to_death': None if healed or seed == 70448 else 20, 'active_mitigation_uptime': 0.2 + 0.2 * (seed - 70448), 'tank_casts': {'123': 2} if seed == 70448 else {}, 'aura_uptime': {'456': 0.5} if seed == 70449 else {}}
print('HEALER_TANK_REPORT=' + json.dumps(report))
''')
            executable.chmod(0o755)
            output = root / 'results'
            result = subprocess.run(['python3', str(SCRIPT), '--all', '--seeds', '2', '--trace-dir', str(root), '--profile-dir', str(root), '--executable', str(executable), '--output-dir', str(output), '--jobs', '3'], text=True, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('| 66 | 50.00 (40.00–60.00) | 30.00 (20.00–40.00) | 0/2 | 1/2 |', result.stdout)
            self.assertIn('| 123 | 1.00 (0.00–2.00) | 0.00 (0.00–0.00) |', result.stdout)
            self.assertIn('| 456 | 0.00 (0.00–0.00) | 25.00 (0.00–50.00) |', result.stdout)
            self.assertEqual(len(list(output.glob('*.json'))), 31)
            self.assertEqual(json.loads((output / 'tank-66-unhealed-seed70449.json').read_text())['time_to_death'], 20)



if __name__ == '__main__':
    unittest.main()
