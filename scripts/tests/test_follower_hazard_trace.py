"""Concrete trace timelines, not source-shape assertions."""
import importlib.util
import json
from pathlib import Path
import unittest

PATH = Path(__file__).resolve().parents[1] / "follower_hazard_trace.py"
SPEC = importlib.util.spec_from_file_location("follower_hazard_trace", PATH)
TRACE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TRACE)


def event(name, simulation_s, monotonic_ns, **data):
    return {"schema": 1, "event": name, "simulation_s": simulation_s,
            "monotonic_ns": monotonic_ns, "actor": 4294966902,
            "data": {"field": 4294966901, "map_id": 2515, **data}}


def timeline(publication=10.1, movement=10.25):
    return [
        event("detected", 10.0, 1_000_000_000, react_at=10.25, position=[0, 0, 0], center=[0, 0, 0], radius_current=5.0),
        event("query_submitted", None, 1_010_000_000, request_id=11),
        event("hold", 10.05, 1_050_000_000, reason="reaction", position=[0, 0, 0]),
        event("query_published", publication, 1_120_000_000, request_id=11, status="ready"),
        event("corridor_adopted", None, 1_125_000_000, request_id=11),
        event("move", movement, 1_310_000_000, position=[1, 0, 0], **{"from": [0, 0, 0], "to": [1, 0, 0]}),
        event("exit", 10.75, 2_000_000_000, physical_clear=True, position=[6, 0, 0]),
    ]


class FollowerHazardTraceTests(unittest.TestCase):
    def test_native_prefix_and_three_latency_distributions_keep_both_clocks(self):
        records = timeline()
        lines = ["ordinary log\n"] + ["DEBUG FOLLOWER_HAZARD_TRACE " + json.dumps(record) for record in records]
        result = TRACE.analyze(TRACE.read_records(lines))
        self.assertEqual(len(result["episodes"]), 1)
        self.assertEqual(result["policy_violations"], [])
        for name, simulation_ms, wall_ms in [("publication", 100.0, 120.0), ("first_move", 250.0, 310.0), ("physical_exit", 750.0, 1000.0)]:
            distribution = result["distributions"][name]
            self.assertEqual(distribution["simulation_ms"]["n"], 1)
            self.assertAlmostEqual(distribution["simulation_ms"]["p50"], simulation_ms)
            self.assertEqual(distribution["wall_ms"]["p95"], wall_ms)

    def test_late_publication_accepts_pending_holds_but_not_an_extra_ready_tick(self):
        records = timeline(publication=10.6, movement=10.6)
        records[3:3] = [event("hold", 10.25, 1_070_000_000, reason="pending", position=[0, 0, 0])]
        self.assertEqual(TRACE.analyze(records)["policy_violations"], [])
        records.insert(6, event("hold", 10.6, 1_200_000_000, reason="pending", position=[0, 0, 0]))
        violations = TRACE.analyze(records)["policy_violations"]
        self.assertIn("late_start_after_ready", [item["kind"] for item in violations])

    def test_early_motion_and_actual_hold_displacement_are_reported(self):
        records = timeline(movement=10.2)
        records[2]["data"]["position"] = [0.25, 0, 0]
        kinds = [item["kind"] for item in TRACE.analyze(records)["policy_violations"]]
        self.assertIn("movement_before_reaction", kinds)
        self.assertIn("hold_displacement", kinds)

    def test_source_cancellation_and_missing_exit_are_not_physical_escape(self):
        records = timeline()[:-1] + [event("cancel", 10.75, 2_000_000_000, reason="field_unavailable_or_expiring")]
        result = TRACE.analyze(records)
        self.assertEqual(result["cancelled"], 1)
        self.assertEqual(result["distributions"]["physical_exit"]["simulation_ms"]["n"], 0)
        self.assertIsNone(result["episodes"][0]["physical_exit"])

    def test_malformed_opt_in_record_and_unknown_schema_fail_explicitly(self):
        with self.assertRaisesRegex(ValueError, "line 2"):
            TRACE.read_records(["normal log", "FOLLOWER_HAZARD_TRACE {broken"])
        with self.assertRaisesRegex(ValueError, "unsupported schema"):
            TRACE.read_records(['FOLLOWER_HAZARD_TRACE {"schema":2}'])


if __name__ == "__main__":
    unittest.main()
