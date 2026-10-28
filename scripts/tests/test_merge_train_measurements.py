"""Exercise merge-train measurements without accessing the live train."""
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack
import importlib.util
import json
from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)


class MeasurementTests(unittest.TestCase):
    def test_specs_share_global_limit_preserve_order_and_forward_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "server_simulation"
            executable.write_text("fixture")
            log = root / "cycle.log"
            entries = [
                {"id": "first", "env": {"DPS_PARITY_SEEDS": "1,2,3,4"}, "measure": [
                    {"spec": str(s), "targets": "1,3", "world_db": "db", "output": f"out-{s}"}
                    for s in (3, 1, 2)
                ]},
                {"id": "second", "env": {"DPS_PARITY_SEEDS": "7,8"}, "measure": [
                    {"spec": str(s), "targets": "1,3", "world_db": "db", "output": f"out-{s}"}
                    for s in (5, 4)
                ]},
            ]
            lock = threading.Lock()
            two_active = threading.Event()
            state = {"active": 0, "maximum": 0, "env": {}}

            def fake_run(command, **kwargs):
                number = command[command.index("--spec") + 1]
                with lock:
                    state["active"] += 1
                    state["maximum"] = max(state["maximum"], state["active"])
                    if state["active"] == 2:
                        two_active.set()
                    state["env"][number] = kwargs["env"].get("DPS_PARITY_SEEDS")
                time.sleep({"3": 0.3, "1": 0.15, "2": 0.03, "5": 0.02, "4": 0.02}[number])
                with lock:
                    state["active"] -= 1
                return type("Result", (), {"stdout": f"noise\nNATIVE_DPS spec{number} targets1: {number}\nNATIVE_DPS spec{number} targets3: {number}\n"})()

            with ExitStack() as stack:
                for name in ("HOME", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                    stack.enter_context(patch.object(train, name, root if name == "HOME" else root / name.lower()))
                stack.enter_context(patch.object(train, "MEASURING", set()))
                stack.enter_context(patch.object(train, "parity_executable", return_value=str(executable)))
                stack.enter_context(patch.object(train, "tree_id", return_value="tree-123"))
                stack.enter_context(patch.object(train.subprocess, "run", side_effect=fake_run))
                # Shut the executor down before the path and subprocess patches are removed,
                # including when an assertion fails while work is still running.
                pool = stack.enter_context(ThreadPoolExecutor(max_workers=2))
                stack.enter_context(patch.object(train, "MEASURER", pool))
                train.measure_then_write([entries[0]], log)
                self.assertTrue(two_active.wait(1), "one entry must run specs concurrently")
                train.measure_then_write([entries[1]], log)
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline and not all((root / "results" / f"{e['id']}.json").exists() for e in entries):
                    time.sleep(0.01)
                self.assertTrue(all((root / "results" / f"{e['id']}.json").exists() for e in entries))
            written = {e["id"]: json.loads((root / "results" / f"{e['id']}.json").read_text())["measurements"]
                       for e in entries}
            self.assertEqual(state["maximum"], 2)
            self.assertEqual(state["env"], {"3": "1,2,3,4", "1": "1,2,3,4", "2": "1,2,3,4",
                                            "5": "7,8", "4": "7,8"})
            self.assertEqual(written["first"], [
                "NATIVE_DPS spec3 targets1: 3", "NATIVE_DPS spec3 targets3: 3",
                "NATIVE_DPS spec1 targets1: 1", "NATIVE_DPS spec1 targets3: 1",
                "NATIVE_DPS spec2 targets1: 2", "NATIVE_DPS spec2 targets3: 2",
            ])
            self.assertEqual(written["second"], [
                "NATIVE_DPS spec5 targets1: 5", "NATIVE_DPS spec5 targets3: 5",
                "NATIVE_DPS spec4 targets1: 4", "NATIVE_DPS spec4 targets3: 4",
            ])
            events = [json.loads(line.removeprefix("TIMING ")) for line in log.read_text().splitlines()
                      if line.startswith("TIMING ")]
            measured = [event for event in events if event["phase"] == "measure"]
            self.assertEqual(len(measured), 10)
            self.assertTrue(all(event["tree"] == "tree-123" for event in measured))
            self.assertEqual({event["event"] for event in measured}, {"start", "end"})
            self.assertTrue(all(isinstance(event["monotonic"], float) for event in measured))
            for span in {event["span"] for event in measured}:
                pair = [event for event in measured if event["span"] == span]
                self.assertEqual([event["event"] for event in pair], ["start", "end"])
                self.assertLessEqual(pair[0]["monotonic"], pair[1]["monotonic"])


    def test_failed_measurement_keeps_its_native_log_beyond_the_shared_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "out"
            output.mkdir()
            (output / "577.native.1.log").write_text("panicked at native item driver effect\n")
            (root / "logs").mkdir()
            failed = type("Run", (), {"stdout": "spec577 targets1: FAILED, see out/577.native.1.log\n",
                                      "stderr": "traceback tail\n"})()
            with patch.object(train, "LOGS", root / "logs"), patch.object(train, "TREE", root), \
                    patch.object(train.subprocess, "run", return_value=failed):
                lines = train.measure_spec({"id": "entry"}, {"spec": "577", "targets": "1", "world_db": "db",
                                                            "output": str(output)}, "exe", root, root / "c.log", "t")
            kept = root / "logs/entry.measure-577.log"
            self.assertEqual(lines[-1], f"FAILED details kept in {kept}")
            (output / "577.native.1.log").write_text("overwritten by the next measurement\n")
            self.assertIn("panicked at native item driver effect", kept.read_text())
            self.assertIn("traceback tail", kept.read_text())

    def test_successful_measurement_keeps_its_per_spell_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "out"
            output.mkdir()
            (output / "1480.native.1.json").write_text('{"Cull": 17722}')
            (output / "1480.native.3.json").write_text('{"Cull": 40000}')
            (output / "1480.native.1.seed-2.json").write_text("per-seed")
            (root / "logs").mkdir()
            passed = type("Run", (), {"stdout": "NATIVE_DPS spec1480 targets1: 114934.05\n", "stderr": ""})()
            with patch.object(train, "LOGS", root / "logs"), patch.object(train, "TREE", root), \
                    patch.object(train.subprocess, "run", return_value=passed):
                train.measure_spec({"id": "entry"}, {"spec": "1480", "targets": "1,3", "world_db": "db",
                                                     "output": str(output)}, "exe", root, root / "c.log", "t")
            (output / "1480.native.1.json").write_text('{"Cull": 0}')
            kept = root / "logs/entry.reports"
            self.assertEqual((kept / "1480.native.1.json").read_text(), '{"Cull": 17722}')
            self.assertEqual((kept / "1480.native.3.json").read_text(), '{"Cull": 40000}')
            self.assertFalse((kept / "1480.native.1.seed-2.json").exists())

    def test_concurrent_entries_sharing_an_output_measure_one_at_a_time(self):
        import threading as th
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "out"
            output.mkdir()
            (root / "logs").mkdir()
            active, peak = [0], [0]
            gate = th.Lock()

            def run(*args, **kwargs):
                with gate:
                    active[0] += 1
                    peak[0] = max(peak[0], active[0])
                time.sleep(0.05)
                with gate:
                    active[0] -= 1
                return type("Run", (), {"stdout": "NATIVE_DPS spec252 targets1: 1\n", "stderr": ""})()

            measurement = {"spec": "252", "targets": "1", "world_db": "db", "output": str(output)}
            with patch.object(train, "LOGS", root / "logs"), patch.object(train, "TREE", root), \
                    patch.object(train.subprocess, "run", side_effect=run):
                threads = [th.Thread(target=train.measure_spec, args=({"id": f"e{i}"}, measurement, "exe", root,
                                                                       root / "c.log", "t")) for i in range(3)]
                for thread in threads:
                    thread.start()
                for thread in threads:
                    thread.join()
            self.assertEqual(peak[0], 1)

    def test_repeated_spec_output_is_serialized_in_submission_order(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)
            executable = root / "server_simulation"
            executable.write_text("fixture")
            for name in ("HOME", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name == "HOME" else root / name.lower()))
            stack.enter_context(patch.object(train, "MEASURING", set()))
            stack.enter_context(patch.object(train, "parity_executable", return_value=str(executable)))
            stack.enter_context(patch.object(train, "tree_id", return_value="tree-abc"))
            order = []
            seeds = []

            def fake_run(command, **kwargs):
                target = command[command.index("--targets") + 1]
                seeds.append(kwargs["env"].get("DPS_PARITY_SEEDS"))
                order.append(f"start-{target}")
                time.sleep(0.05)
                order.append(f"end-{target}")
                return type("Result", (), {"stdout": f"NATIVE_DPS target {target}\n"})()

            stack.enter_context(patch.object(train.subprocess, "run", side_effect=fake_run))
            pool = stack.enter_context(ThreadPoolExecutor(max_workers=2))
            stack.enter_context(patch.object(train, "MEASURER", pool))
            entry = {"id": "duplicate", "measure": [
                {"spec": "259", "targets": target, "world_db": "db", "output": "shared"}
                for target in ("1", "3")
            ]}
            train.measure_then_write([entry], root / "cycle.log")
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline and not (root / "results/duplicate.json").exists():
                time.sleep(0.01)
            self.assertTrue((root / "results/duplicate.json").exists())
            self.assertEqual(order, ["start-1", "end-1", "start-3", "end-3"])
            self.assertEqual(seeds, ["70448,1,2,3,4,5,6,7,8,9"] * 2)
            self.assertEqual(json.loads((root / "results/duplicate.json").read_text())["measurements"],
                             ["NATIVE_DPS target 1", "NATIVE_DPS target 3"])

    def test_entries_with_equal_inputs_on_one_tree_share_a_single_measurement(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)
            executable = root / "server_simulation"
            executable.write_text("fixture")
            for name in ("HOME", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name == "HOME" else root / name.lower()))
            stack.enter_context(patch.object(train, "MEASURING", set()))
            stack.enter_context(patch.object(train, "parity_executable", return_value=str(executable)))
            stack.enter_context(patch.object(train, "tree_id", return_value="tree-abc"))
            outputs = {}
            for name, profile in (("first", "same"), ("second", "same"), ("other", "different")):
                outputs[name] = root / name
                outputs[name].mkdir()
                (outputs[name] / "252.profile.json").write_text(profile)
                (outputs[name] / "252.selected-traits-active.json").write_text("[]")
            runs = []

            def fake_run(command, **kwargs):
                output = Path(command[command.index("--output") + 1])
                runs.append(output.name)
                (output / "252.native.1.json").write_text(output.name)
                return type("Result", (), {"stdout": f"NATIVE_DPS spec252 targets1: {output.name}\n",
                                           "stderr": ""})()

            stack.enter_context(patch.object(train.subprocess, "run", side_effect=fake_run))
            pool = stack.enter_context(ThreadPoolExecutor(max_workers=2))
            stack.enter_context(patch.object(train, "MEASURER", pool))
            entries = [{"id": name, "measure": [{"spec": "252", "targets": "1", "world_db": "db",
                                                  "output": str(outputs[name])}]}
                       for name in ("first", "second", "other")]
            train.measure_then_write(entries, root / "cycle.log")
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline and not all(
                    (root / f"results/{name}.json").exists() for name in outputs):
                time.sleep(0.01)

            def result(name):
                return json.loads((root / f"results/{name}.json").read_text())["measurements"]
            self.assertEqual(sorted(runs), ["first", "other"])
            self.assertEqual(result("second"), result("first"))
            self.assertEqual(result("other"), ["NATIVE_DPS spec252 targets1: other"])
            self.assertEqual((root / "logs/second.reports/252.native.1.json").read_text(), "first")

    def test_cycle_events_cover_operation_boundaries_with_tree_ids(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)
            log = root / "cycle.log"
            for name in ("HOME", "TREE", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name in ("HOME", "TREE") else root / name.lower()))
            stack.enter_context(patch.object(train, "tree_id", return_value="tree-abc"))
            stack.enter_context(patch.object(train, "cargo", return_value=0))
            stack.enter_context(patch.object(train, "drop_union_duplicates"))
            result = type("Result", (), {"returncode": 0, "stdout": "", "stderr": ""})()
            stack.enter_context(patch.object(train, "git", return_value=result))
            stack.enter_context(patch.object(train.subprocess, "run", return_value=result))
            entry = {"id": "fixture", "commit": "commit-123", "env": {}, "measure": []}
            train.merge([entry], log)
            self.assertTrue(train.check(log))
            train.test_executables(entry, ["-p", "server-simulation", "--lib"], root / "entry.log", log, "tree-abc")
            train.run_executable(entry, ["-p", "server-simulation", "--lib"], "fixture-executable", root,
                                 root / "entry.log", log, "tree-abc")
            train.parity_executable(log)
            events = [json.loads(line.removeprefix("TIMING ")) for line in log.read_text().splitlines()
                      if line.startswith("TIMING ")]
            for phase in ("merge", "check", "fmt", "no-run-build", "test-execution", "parity-build"):
                selected = [event for event in events if event["phase"] == phase]
                self.assertGreaterEqual(len(selected), 2, phase)
                self.assertEqual([event["event"] for event in selected].count("start"),
                                 [event["event"] for event in selected].count("end"))
                self.assertTrue(all(event["tree"] == "tree-abc" for event in selected))
                self.assertTrue(all(isinstance(event["monotonic"], float) for event in selected))


if __name__ == "__main__":
    unittest.main()
