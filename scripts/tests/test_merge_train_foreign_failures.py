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


if __name__ == "__main__":
    unittest.main()
