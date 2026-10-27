"""Per-test --measure selection with a leading @SPEC token, without the live train."""
import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)


def measure(spec_id, actor):
    return {"spec": spec_id, "targets": "1,3", "world_db": f"db-{spec_id}", "output": f"out-{spec_id}",
            "actor": actor}


class TestSpecSelection(unittest.TestCase):
    entry = {"env": {}, "measure": [measure("252", "player"), measure("266", "follower")]}

    def test_at_spec_uses_that_specs_inputs_and_is_not_passed_to_cargo(self):
        test = ["@266", "-p", "server-simulation", "--lib", "--", "demonology_", "--ignored"]
        env = train.test_env(self.entry, test)
        self.assertEqual((env["DPS_PARITY_SPEC"], env["DPS_PARITY_OUTPUT"], env["DPS_PARITY_ACTOR"]),
                         ("266", "out-266", "follower"))
        cargo_args, _, binary_args = train.split_test(test)
        self.assertEqual(cargo_args, ["-p", "server-simulation", "--lib"])
        self.assertEqual(binary_args, ["demonology_", "--ignored"])

    def test_plain_test_keeps_first_measure(self):
        env = train.test_env(self.entry, ["-p", "server-simulation", "--lib", "--", "dk252_"])
        self.assertEqual(env["DPS_PARITY_SPEC"], "252")


class MeasurementFailureStatus(unittest.TestCase):
    def test_failed_measurement_is_named_in_status(self):
        import tempfile
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            entry = {"id": "e1", "status": "probe passed",
                     "measurements": ["NATIVE_DPS spec64 targets1: FAILED, see log", "NATIVE_DPS spec64 targets3: 1.0"]}
            with patch.object(train, "RESULTS", root / "results"), patch.object(train, "RUNNING", root / "running"):
                train.write_results([entry])
            self.assertEqual(entry["status"], "probe passed; measurement FAILED (1)")


if __name__ == "__main__":
    unittest.main()
