"""Process-boundary qualification for the seeded simulator runner."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest

RUNNER = Path(__file__).resolve().parents[1] / "run-dungeon-simulator.py"

WORKER = r'''
import json, os, sys, time
from pathlib import Path
out = Path(os.environ['DUNGEON_SIMULATOR_OUTPUT'])
seed = int(os.environ['DUNGEON_SIMULATOR_SEED'])
assert os.environ['DUNGEON_SIMULATOR_RUNS'] == '1'
started = time.monotonic()
time.sleep(0.25)
(out / f'interval-{seed}.json').write_text(json.dumps([started, time.monotonic()]))
if seed == int(os.environ.get('FIXTURE_MISSING_SEED', '-1')):
    sys.exit(3)
report = {'seed': seed, 'status': 'stalled' if seed == 2 else 'cleared',
          'failed_pack': 20 if seed == 2 else None,
          'pulls': [{'pack': 10}, {'pack': 20}]}
(out / f'run-{seed}.json').write_text(json.dumps(report))
(out / 'summary.json').write_text(json.dumps({'runs': 1, 'seeds': [seed]}))
sys.exit(101 if seed == 2 else 0)
'''


class RunnerProcesses(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="dungeon-runner-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.executable = self.root / "native-test"
        self.executable.write_text(f"#!{sys.executable}\n" + textwrap.dedent(WORKER))
        self.executable.chmod(0o755)

    def run_batch(self, jobs, output, environment=None):
        return subprocess.run(
            [sys.executable, str(RUNNER), "--test-executable", str(self.executable),
             "--output", str(output), "--runs", "4", "--seed", "1", "--jobs", str(jobs)],
            capture_output=True, text=True, env=environment, check=False,
        )

    def test_parallel_seeds_overlap_and_merge_failures_without_changing_reports(self):
        parallel = self.root / "parallel"
        result = self.run_batch(2, parallel)
        self.assertEqual(result.returncode, 101, result.stderr)
        summary = json.loads((parallel / "summary.json").read_text())
        self.assertEqual(summary["seeds"], [1, 2, 3, 4])
        self.assertEqual((summary["runs"], summary["cleared"], summary["stalled"], summary["failed_runs"]), (4, 3, 1, 1))
        self.assertEqual(summary["packs"]["10"]["runs_reaching"], 4)
        self.assertEqual(summary["packs"]["20"]["failures"], 1)
        intervals = [json.loads((parallel / f"interval-{seed}.json").read_text()) for seed in range(1, 5)]
        self.assertTrue(any(max(a[0], b[0]) < min(a[1], b[1]) for i, a in enumerate(intervals) for b in intervals[i + 1:]), "real seed processes must overlap")
        serial = self.root / "serial"
        self.assertEqual(self.run_batch(1, serial).returncode, 101)
        for seed in range(1, 5):
            self.assertEqual(json.loads((parallel / f"run-{seed}.json").read_text()), json.loads((serial / f"run-{seed}.json").read_text()))

    def test_missing_process_report_cannot_reuse_stale_seed_success(self):
        import os
        output = self.root / "missing"
        output.mkdir()
        (output / "run-3.json").write_text(json.dumps({"seed": 3, "status": "cleared", "failed_pack": None, "pulls": []}))
        result = self.run_batch(2, output, os.environ | {"FIXTURE_MISSING_SEED": "3"})
        self.assertNotEqual(result.returncode, 0)
        summary = json.loads((output / "summary.json").read_text())
        self.assertEqual(summary["missing_seeds"], [3])
        self.assertEqual(summary["requested_runs"], 4)
        self.assertEqual(summary["runs"], 3)
        self.assertIn("3", result.stderr)


if __name__ == "__main__":
    unittest.main()
