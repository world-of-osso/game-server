import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "run-keystone-simulator.py"


class KeystoneRunnerCliTests(unittest.TestCase):
    def test_passes_explicit_inputs_to_native_test_executable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "fixture"
            executable.write_text(
                f"#!{sys.executable}\n"
                "import json, os, pathlib, sys\n"
                "keys = ['DUNGEON_SIMULATOR_MAP', 'KEYSTONE_SIMULATOR_LEVEL', "
                "'KEYSTONE_SIMULATOR_PARTY', 'KEYSTONE_SIMULATOR_SEED', 'KEYSTONE_SIMULATOR_SCHEDULE_EPOCH']\n"
                "pathlib.Path(os.environ['KEYSTONE_SIMULATOR_OUTPUT']).write_text("
                "json.dumps({'environment': {k: os.environ[k] for k in keys}, 'args': sys.argv[1:]}))\n"
            )
            executable.chmod(0o755)
            output = root / "record.json"
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--test-executable", str(executable),
                 "--dungeon", "2515", "--key-level", "2", "--party", "66,105,262,63,253",
                 "--seed", "7", "--schedule-epoch", "1792454400", "--output", str(output)],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            record = json.loads(output.read_text())
            self.assertEqual(record["environment"], {
                "DUNGEON_SIMULATOR_MAP": "2515", "KEYSTONE_SIMULATOR_LEVEL": "2",
                "KEYSTONE_SIMULATOR_PARTY": "66,105,262,63,253", "KEYSTONE_SIMULATOR_SEED": "7",
                "KEYSTONE_SIMULATOR_SCHEDULE_EPOCH": "1792454400",
            })
            self.assertEqual(record["args"], [
                "dungeon_simulator::keystone::tests::run_headless_keystone", "--exact", "--ignored", "--nocapture",
            ])

    def test_rejects_incomplete_party(self):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--test-executable", sys.executable, "--dungeon", "2515",
             "--key-level", "2", "--party", "66,105", "--output", "unused.json"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("tank, healer and three damage", result.stderr)


if __name__ == "__main__":
    unittest.main()
