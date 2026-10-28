"""The gap report reads each spec's measurement from the newest master landing."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "dps_parity_gaps", Path(__file__).resolve().parents[1] / "dps_parity_gaps.py")
GAPS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GAPS)


class LatestOutputsTests(unittest.TestCase):
    def write_result(self, root, entry_id, status, spec, mtime):
        output = root / entry_id
        output.mkdir()
        report = output / f"{spec}.native.1.json"
        report.write_text("{}")
        os.utime(report, (mtime, mtime))
        (root / "results" / f"{entry_id}.json").write_text(json.dumps({
            "status": status,
            "measurements": [f"{spec} single NATIVE_DPS 1"],
            "measure": [{"spec": spec, "output": str(output)}],
        }))
        return output

    def test_late_measurement_of_older_landing_does_not_replace_newer_landing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "results").mkdir()
            newer = self.write_result(root, "newer", "landed in 8bbc870000", 269, mtime=1000)
            self.write_result(root, "older", "landed in 9a5ae1b735; measurement FAILED (1)", 269, mtime=2000)
            self.write_result(root, "probe", "probe passed", 269, mtime=3000)
            GAPS.RESULTS = root / "results"

            outputs = GAPS.latest_outputs(order={"9a5ae1b735": 1, "8bbc870000": 2})

            self.assertEqual(outputs, {269: (newer, "landed in 8bbc870000")})

    def test_archived_reports_win_over_a_shared_output_dir_overwritten_later(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "results").mkdir()
            self.write_result(root, "landed", "landed in 8bbc870000", 260, mtime=1000)
            archive = root / "logs" / "landed.reports"
            archive.mkdir(parents=True)
            (archive / "260.native.1.json").write_text("{}")
            GAPS.RESULTS = root / "results"

            outputs = GAPS.latest_outputs(order={"8bbc870000": 1})

            self.assertEqual(outputs, {260: (archive, "landed in 8bbc870000")})


if __name__ == "__main__":
    unittest.main()
