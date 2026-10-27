"""Measure-only grouping against private, temporary Git repositories."""
from contextlib import ExitStack
import importlib.util
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train_grouping", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)


class GroupingTests(unittest.TestCase):
    def test_same_resolved_tree_builds_once_and_preserves_entry_inputs(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)

            def git(*args):
                return subprocess.run(["git", *args], cwd=root, text=True, capture_output=True, check=True).stdout.strip()

            git("init", "-q")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.test")
            (root / "Cargo.toml").write_text("[workspace]\n")
            git("add", "Cargo.toml")
            git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            git("update-ref", "refs/remotes/origin/master", base)
            git("commit", "--allow-empty", "-qm", "same tree, different commit")
            same = git("rev-parse", "HEAD")
            (root / "change.txt").write_text("different tree")
            git("add", "change.txt")
            git("commit", "-qm", "different tree")
            different = git("rev-parse", "HEAD")
            entries = [
                {"id": "first", "commit": base, "env": {"DPS_PARITY_SEEDS": "1"}, "tests": [], "measure": [{"spec": "62"}]},
                {"id": "different", "commit": different, "env": {}, "tests": [], "measure": [{"spec": "64"}]},
                {"id": "same", "commit": same, "env": {"DPS_PARITY_SEEDS": "2"}, "tests": [], "measure": [{"spec": "263"}]},
                {"id": "profile", "commit": base, "env": {"CARGO_PROFILE_TEST_OPT_LEVEL": "1"}, "tests": [], "measure": [{"spec": "259"}]},
                {"id": "feature", "commit": base, "env": {"CARGO_FEATURE_SPECIAL": "1"}, "tests": [], "measure": [{"spec": "260"}]},
            ]
            for name in ("HOME", "TREE", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name in ("HOME", "TREE") else root / name.lower()))
            (root / "running").mkdir()
            (root / "queue").mkdir()
            for entry in entries:
                (root / "running" / f"{entry['id']}.json").write_text("{}")
            builds, measures = [], []

            def build(group, log):
                builds.append([entry["id"] for entry in group])
                return group, []

            def measure(group, log):
                measures.append([(entry["id"], entry["measure"], entry["env"], entry["status"]) for entry in group])

            stack.enter_context(patch.object(train, "reset_tree"))
            stack.enter_context(patch.object(train, "build_and_test", side_effect=build))
            stack.enter_context(patch.object(train, "measure_then_write", side_effect=measure))
            stack.enter_context(patch.object(train, "write_results"))
            train.run_probes(entries, [], root / "cycle.log")
            self.assertEqual(builds, [["first"], ["different"], ["profile"]])
            self.assertEqual([[item[0] for item in group] for group in measures],
                             [["first", "same"], ["different"], ["profile"]])
            self.assertEqual(measures[0][0][1:3], (entries[0]["measure"], {"DPS_PARITY_SEEDS": "1"}))
            self.assertEqual(measures[0][1][1:3], (entries[2]["measure"], {"DPS_PARITY_SEEDS": "2"}))
            self.assertTrue(all(item[3] == "probe passed" for group in measures for item in group))
            self.assertTrue((root / "queue/feature.json").exists())
            self.assertFalse((root / "running/feature.json").exists())

    def test_probe_waits_while_its_owner_or_descendant_probe_shares_the_tree(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)

            def git(*args):
                return subprocess.run(["git", *args], cwd=root, text=True, capture_output=True, check=True).stdout.strip()

            def commit(name, parent):
                git("checkout", "-q", "--detach", parent)
                (root / f"{name}.rs").write_text(name)
                git("add", f"{name}.rs")
                git("commit", "-qm", name)
                return git("rev-parse", "HEAD")

            git("init", "-q")
            git("config", "user.name", "Fixture")
            git("config", "user.email", "fixture@example.test")
            git("commit", "--allow-empty", "-qm", "base")
            base = git("rev-parse", "HEAD")
            red = commit("enlightened_test", base)
            green = commit("enlightened_fix", red)
            sibling_red = commit("rapidity_test", base)
            other = commit("hati_fix", base)
            unowned_red = commit("chakram_test", base)
            unowned_green = commit("chakram_fix", unowned_red)

            def probe(name, sha, owner=None):
                return {"id": name, "commit": sha, "owner": owner, "tests": ["dps_parity::"], "measure": []}

            entries = [
                probe("red", red, "parity-62"),
                probe("green", green, "parity-62"),
                probe("sibling-red", sibling_red, "parity-62"),
                probe("other", other, "parity-253"),
                probe("unowned-red", unowned_red),
                probe("unowned-green", unowned_green),
            ]
            for name in ("HOME", "TREE", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name in ("HOME", "TREE") else root / name.lower()))
            (root / "running").mkdir()
            (root / "queue").mkdir()
            for entry in entries:
                (root / "running" / f"{entry['id']}.json").write_text("{}")
            builds = []

            def build(group, log):
                builds.append([entry["id"] for entry in group])
                return group, []

            stack.enter_context(patch.object(train, "build_and_test", side_effect=build))
            stack.enter_context(patch.object(train, "measure_then_write"))
            stack.enter_context(patch.object(train, "write_results"))
            train.run_probes(entries, [], root / "cycle.log")
            # A RED probe built beside its owner's fix (or any probe containing it) would pass.
            self.assertEqual(builds, [["red", "other", "unowned-red"]])
            for deferred in ("green", "sibling-red", "unowned-green"):
                self.assertTrue((root / "queue" / f"{deferred}.json").exists(), deferred)
                self.assertFalse((root / "running" / f"{deferred}.json").exists(), deferred)

    def test_failed_and_deferred_representatives_propagate_to_group(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            root = Path(directory)
            for name in ("HOME", "TREE", "RESULTS", "QUEUE", "RUNNING", "LOGS"):
                stack.enter_context(patch.object(train, name, root if name in ("HOME", "TREE") else root / name.lower()))
            (root / "queue").mkdir()
            (root / "running").mkdir()
            failed = {"id": "failed", "status": "rejected", "reason": "check failed"}
            sibling = {"id": "sibling"}
            saved = []
            stack.enter_context(patch.object(train, "write_results", side_effect=lambda entries: saved.extend(entries)))
            stack.enter_context(patch.object(train, "git", return_value=SimpleNamespace(stdout="base\n")))
            with patch.object(train, "build_and_test", return_value=([], [failed])):
                passed, rejected = train.build_isolated_group([failed, sibling], root / "cycle.log", "base")
            self.assertEqual(passed, [])
            self.assertEqual([entry["id"] for entry in rejected], ["failed", "sibling"])
            self.assertEqual((sibling["status"], sibling["reason"]), ("rejected", "check failed"))
            self.assertEqual(saved, [sibling])
            deferred = {"id": "deferred"}
            follower = {"id": "follower"}
            (root / "running/follower.json").write_text("{}")
            with patch.object(train, "build_and_test", return_value=([], [])):
                self.assertEqual(train.build_isolated_group([deferred, follower], root / "cycle.log", "base"), ([], []))
            self.assertTrue((root / "queue/follower.json").exists())
            changed = {"id": "changed"}
            (root / "running/changed.json").write_text("{}")
            with patch.object(train, "build_and_test", return_value=([deferred], [])):
                self.assertEqual(train.build_isolated_group([deferred, changed], root / "cycle.log", "old-base"),
                                 ([deferred], []))
            self.assertTrue((root / "queue/changed.json").exists())


if __name__ == "__main__":
    unittest.main()
