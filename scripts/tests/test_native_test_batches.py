"""Real subprocess lifetime/coverage contracts for the libtest-JSON batch runner."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

RUNNER = Path(__file__).resolve().parents[1] / "native_test_batches.py"
BACKEND = r'''#!/usr/bin/env python3
import argparse
import json
import os
import time

parser = argparse.ArgumentParser()
parser.add_argument("--list", action="store_true")
parser.add_argument("--exact", action="store_true")
parser.add_argument("--test-threads")
parser.add_argument("--report-time", action="store_true")
parser.add_argument("--format")
parser.add_argument("-Z")
parser.add_argument("--skip", action="append", default=[])
parser.add_argument("filters", nargs="*")
args = parser.parse_args()
cases = [(module + "::" + case, False) for module in ["left", "right", "third"] for case in ["alpha", "beta"]]
cases += [("left::ignored", True), ("broken::failure", False)]
selected = [(name, ignored) for name, ignored in cases
            if (not args.filters or any((value == name if args.exact else value in name) for value in args.filters))
            and not any(value in name for value in args.skip)]
def emit(row):
    print(json.dumps(row), flush=True)
def record(event, **fields):
    row = {"event": event, "pid": os.getpid(), "time": time.monotonic(), **fields}
    descriptor = os.open(os.environ["NATIVE_BATCH_FIXTURE_LOG"], os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    os.write(descriptor, (json.dumps(row) + "\n").encode())
    os.close(descriptor)
if args.list:
    for name, ignored in selected:
        emit({"type": "test", "event": "discovered", "name": name, "ignore": ignored})
else:
    record("start")
    time.sleep(0.15)
    owner = None
    failed = False
    for name, ignored in selected:
        if ignored:
            emit({"type": "test", "event": "ignored", "name": name})
            continue
        module = name.split("::")[0]
        owner = owner or module
        passed = owner == module and module != "broken"
        failed = failed or not passed
        record("case", name=name, cached_owner=owner)
        emit({"type": "test", "event": "ok" if passed else "failed", "name": name, "exec_time": 0.01})
    record("end")
    raise SystemExit(1 if failed else 0)
'''


class NativeBatchContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        binary = self.root / "test-backend"
        binary.write_text(BACKEND)
        binary.chmod(0o755)
        self.artifacts = self.root / "artifacts.jsonl"
        self.artifacts.write_text(json.dumps({
            "reason": "compiler-artifact", "profile": {"test": True},
            "target": {"name": "lifetime-fixture"}, "executable": str(binary),
        }) + "\n")
        self.events = self.root / "process-events.jsonl"

    def run_batches(self, *filters):
        output = self.root / "output"
        result = subprocess.run(
            [sys.executable, str(RUNNER), "--artifacts", str(self.artifacts), "--output", str(output), *filters],
            env=dict(os.environ, NATIVE_BATCH_FIXTURE_LOG=str(self.events)),
            capture_output=True, text=True,
        )
        summary = json.loads((output / "summary.json").read_text())
        events = [json.loads(line) for line in self.events.read_text().splitlines()]
        return result, summary, events

    def test_module_cache_lifetimes_reset_without_losing_cases_or_ignores(self):
        result, summary, events = self.run_batches("left::", "right::", "third::")
        self.assertEqual(result.returncode, 0, result.stderr)
        cases = [event for event in events if event["event"] == "case"]
        self.assertEqual({event["name"] for event in cases}, {
            module + "::" + case for module in ["left", "right", "third"] for case in ["alpha", "beta"]
        })
        self.assertEqual(len(cases), 6)
        self.assertEqual(summary["passed"], 6)
        self.assertEqual(summary["ignored"], 1)
        self.assertEqual(summary["missing"], [])
        owners = {}
        for event in cases:
            module = event["name"].split("::")[0]
            owners.setdefault(module, set()).add(event["pid"])
            self.assertEqual(event["cached_owner"], module)
        self.assertTrue(all(len(pids) == 1 for pids in owners.values()))
        self.assertEqual(len(set.union(*owners.values())), 3)
        active = peak = 0
        for event in sorted(events, key=lambda row: row["time"]):
            if event["event"] == "start":
                active += 1
                peak = max(peak, active)
            elif event["event"] == "end":
                active -= 1
        self.assertEqual((peak, active), (2, 0))
        self.assertTrue(all(group["cpu_seconds"] > 0 for group in summary["groups"]))

    def test_failure_is_reported_without_dropping_other_module_coverage(self):
        result, summary, events = self.run_batches("broken::", "right::")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((summary["passed"], summary["failed"]), (2, 1))
        self.assertEqual(summary["missing"], [])
        self.assertEqual({event["name"] for event in events if event["event"] == "case"},
                         {"broken::failure", "right::alpha", "right::beta"})


if __name__ == "__main__":
    unittest.main()
