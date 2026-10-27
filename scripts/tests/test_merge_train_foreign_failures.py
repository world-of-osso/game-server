"""Foreign-failure attribution against private, temporary Git repositories."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train_foreign", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)

TEST_FILE = "crates/sim/src/pet_tests.rs"
OTHER = "#[test]\nfn other_entry_test() {\n    assert!(true);\n}\n"


class ForeignFailureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.test")
        self.write(OTHER)
        self.master = self.commit("base")
        self.git("update-ref", "refs/remotes/origin/master", self.master)
        tree = patch.object(train, "TREE", self.root)
        tree.start()
        self.addCleanup(tree.stop)

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, text=True, check=True,
                              capture_output=True).stdout.strip()

    def write(self, text):
        path = self.root / TEST_FILE
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def combine(self, text):
        """Check out a combined tree on top of master whose test file holds `text`."""
        self.git("checkout", "-q", "--detach", self.master)
        self.write(text)
        self.commit("combined tree")

    def test_own_new_test_reformatted_by_train_fmt_stays_this_entrys_failure(self):
        # 2026-10-27: 59741a5cf2 added this test; the train's cargo fmt turned
        # `else { continue }` into `else { continue; }` and the failure was ignored.
        own = "#[test]\nfn new_entry_test() {\n    let Some(x) = None::<u8> else { continue };\n}\n"
        self.write(OTHER + own)
        entry = self.commit("entry adds test")
        self.combine(OTHER + own.replace("{ continue }", "{ continue; }"))
        self.assertFalse(train.foreign_failures_only(
            {"commit": entry}, {"sim::pet_tests::new_entry_test"}))

    def test_test_changed_only_by_another_entry_is_foreign(self):
        self.write(OTHER + "\n// unrelated entry change\n")
        entry = self.commit("entry leaves the test alone")
        self.combine(OTHER.replace("assert!(true)", "assert!(false)") + "\n// unrelated entry change\n")
        self.assertTrue(train.foreign_failures_only(
            {"commit": entry}, {"sim::pet_tests::other_entry_test"}))


    def test_panic_only_in_code_another_probe_changed_is_foreign(self):
        # 2026-10-27: 16956's Arcanoweave probe panicked midnight_stat_procs.rs:40 inside the
        # shared probe tree and three other owners' probes were rejected for it.
        procs = "crates/server-combat/src/auras/midnight_stat_procs.rs"
        self.write(OTHER + "\n// probe edits only its test file\n")
        entry = self.commit("probe")
        self.git("checkout", "-q", "--detach", self.master)
        (self.root / procs).parent.mkdir(parents=True, exist_ok=True)
        (self.root / procs).write_text("fn driver() { panic!(\"native item driver effect\") }\n")
        self.write(OTHER + "\n// probe edits only its test file\n")
        self.commit("combined tree with another probe's production change")
        foreign = "thread 'x' panicked at server-combat/src/auras/midnight_stat_procs.rs:40:9:\nnative item driver effect"
        own = "thread 'x' panicked at crates/sim/src/pet_tests.rs:3:5:\nassertion failed"
        self.assertTrue(train.foreign_panics_only({"commit": entry}, [foreign]))
        self.assertFalse(train.foreign_panics_only({"commit": entry}, [foreign, own]))
        self.assertFalse(train.foreign_panics_only({"commit": entry}, ["error: test failed, no panic"]))


class RequeueTests(unittest.TestCase):
    def test_deferred_check_and_test_entries_return_to_the_queue(self):
        # 2026-10-27: a helper named like the queue-reload requeue() was shadowed by it and
        # crash-looped the service on the first deferred entry.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "running").mkdir()
            (root / "queue").mkdir()
            for name in ("check", "probe", "landed"):
                (root / "running" / f"{name}.json").write_text("{}")
            check = {"id": "check", "defer": True, "reason": "conflicts with queued entry"}
            probe = {"id": "probe", "defer": True, "reason": "foreign panic"}
            landed = {"id": "landed"}
            with patch.object(train, "RUNNING", root / "running"), \
                    patch.object(train, "QUEUE", root / "queue"), \
                    patch.object(train, "write_results"), \
                    patch.object(train, "build_checked", return_value=([probe, landed], [check])), \
                    patch.object(train, "run_tests", return_value=([landed], [probe])):
                passed, rejected = train.build_and_test([check, probe, landed], root / "cycle.log")
            self.assertEqual((passed, rejected), ([landed], []))
            self.assertEqual(sorted(p.name for p in (root / "queue").iterdir()), ["check.json", "probe.json"])

if __name__ == "__main__":
    unittest.main()
