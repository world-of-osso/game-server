import datetime as dt
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "refresh_mythic_plus_week.py"


def ranking(code, defense, *, region="US", level=8):
    return {
        "server": {"region": region}, "hardModeLevel": level,
        "startTime": int(dt.datetime.fromisoformat("2026-11-04T10:00:00-08:00").timestamp() * 1000),
        "affixes": [160, defense], "report": {"code": code, "fightID": 2},
    }


class RefreshWeekTest(unittest.TestCase):
    def test_pacific_reset_spans_the_fall_dst_transition(self):
        week_bounds = runpy.run_path(str(SCRIPT))["week_bounds"]
        before_reset = dt.datetime.fromisoformat("2026-11-03T07:59:59-08:00")
        after_reset = dt.datetime.fromisoformat("2026-11-03T08:00:00-08:00")
        self.assertEqual(week_bounds(before_reset), (1793113200, 1793721600))
        self.assertEqual(week_bounds(after_reset), (1793721600, 1794326400))

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.policy = self.root / "weeks.json"
        self.policy.write_text('{"weeks": []}\n')
        self.fixture = self.root / "rankings.json"
        self.command = self.root / "wcl"
        self.command.write_text("#!/usr/bin/env python3\nimport os\nprint(open(os.environ['WCL_FIXTURE']).read())\n")
        self.command.chmod(0o755)
        self.env = dict(os.environ, PATH=f"{self.root}:{os.environ['PATH']}", WCL_FIXTURE=str(self.fixture))

    def invoke(self, rows):
        self.fixture.write_text(json.dumps({"characterRankings": {"rankings": rows, "hasMorePages": False}}))
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--policy", str(self.policy), "--as-of", "2026-11-04T12:00:00-08:00"],
            env=self.env, text=True, capture_output=True,
        )

    def test_appends_observed_current_week_and_is_idempotent(self):
        rows = [ranking("ReportA", 9), ranking("ReportB", 9), ranking("OtherRegion", 10, region="EU")]
        result = self.invoke(rows)
        self.assertEqual(result.returncode, 0, result.stderr)
        week = json.loads(self.policy.read_text())["weeks"][0]
        self.assertEqual((week["start_unix"], week["end_unix"]), (1793721600, 1794326400))
        self.assertEqual((week["bargain"], week["defense"]), (160, 9))
        self.assertIn("ReportA#fight=2", week["citation"])
        self.assertIn("ReportB#fight=2", week["defense_citation"])
        before = self.policy.read_bytes()
        result = self.invoke(rows)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Confirmed", result.stdout)
        self.assertEqual(self.policy.read_bytes(), before)

    def test_missing_runtime_file_is_initialized_from_seed(self):
        self.policy.unlink()
        result = self.invoke([ranking("ReportA", 9), ranking("ReportB", 9)])
        self.assertEqual(result.returncode, 0, result.stderr)
        weeks = json.loads(self.policy.read_text())["weeks"]
        self.assertEqual(len(weeks), 12)
        self.assertEqual((weeks[-1]["bargain"], weeks[-1]["defense"]), (160, 9))

    def test_conflicting_reports_leave_table_unchanged(self):
        before = self.policy.read_bytes()
        result = self.invoke([ranking("ReportA", 9), ranking("ReportB", 10)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Conflicting WCL affixes", result.stderr)
        self.assertEqual(self.policy.read_bytes(), before)

    def test_wcl_error_is_reported_without_writing(self):
        self.command.write_text("#!/usr/bin/env python3\nimport sys\nprint('authentication failed', file=sys.stderr)\nsys.exit(1)\n")
        before = self.policy.read_bytes()
        result = self.invoke([])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("authentication failed", result.stderr)
        self.assertEqual(self.policy.read_bytes(), before)

    def test_existing_week_conflict_does_not_overwrite(self):
        original = {"weeks": [{
            "start_unix": 1793721600, "end_unix": 1794326400,
            "bargain": 148, "defense": 9,
        }]}
        self.policy.write_text(json.dumps(original))
        before = self.policy.read_bytes()
        result = self.invoke([ranking("ReportA", 9), ranking("ReportB", 9)])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("conflict with existing week", result.stderr)
        self.assertEqual(self.policy.read_bytes(), before)

    def test_one_distinct_report_does_not_publish(self):
        second_fight = ranking("ReportA", 9)
        second_fight["report"]["fightID"] = 3
        result = self.invoke([ranking("ReportA", 9), second_fight])
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(json.loads(self.policy.read_text())["weeks"], [])


if __name__ == "__main__":
    unittest.main()
