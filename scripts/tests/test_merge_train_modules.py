"""Submission and E0583 isolation against private, temporary Git repositories."""
import argparse
from contextlib import ExitStack, redirect_stdout
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train_modules", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)


class ModuleGuardTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.run_git("init", "-q")
        self.run_git("config", "user.name", "Fixture")
        self.run_git("config", "user.email", "fixture@example.test")
        self.write("src/lib.rs", "mod parent;\n")
        self.write("src/parent.rs", "// parent\n")
        self.write("src/registry.in", "// declarations\n")
        self.commit("base")
        self.base = self.run_git("rev-parse", "HEAD")
        self.run_git("update-ref", "refs/remotes/origin/master", self.base)
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        for name in ("HOME", "TREE", "QUEUE", "RUNNING", "RESULTS", "LOGS"):
            self.stack.enter_context(patch.object(train, name, self.root if name in ("HOME", "TREE") else self.root / name.lower()))
        old_cwd = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, old_cwd)

    def run_git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, text=True, check=True, capture_output=True).stdout.strip()

    def write(self, path, text):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text)

    def commit(self, message):
        self.run_git("add", "-u")
        self.run_git("add", "src")
        self.run_git("commit", "-qm", message)
        return self.run_git("rev-parse", "HEAD")

    def submit(self, commit):
        args = argparse.Namespace(commit=commit, owner="fixture", land=False, red=None,
                                  env=[], test=[], measure=[])
        with redirect_stdout(io.StringIO()):
            train.submit(args)

    def test_submit_rejects_untracked_module_from_commit(self):
        self.write("src/registry.in", "mod missing;\n")
        self.run_git("add", "src/registry.in")
        self.run_git("commit", "-qm", "declaration")
        commit = self.run_git("rev-parse", "HEAD")
        self.write("src/missing.rs", "// untracked, not in commit\n")
        with self.assertRaisesRegex(SystemExit, r"registry\.in.*missing.*missing\.rs"):
            self.submit(commit)
        self.assertFalse(list((self.root / "queue").glob("*.json")))

    def test_submit_accepts_committed_nested_and_explicit_path_modules(self):
        self.write("src/parent.rs", '#[path = "alternative.rs"]\nmod named;\nmod child;\nmod inside {\n    #[path = "special.rs"]\n    mod actual;\n}\n')
        self.write("src/parent/inside/special.rs", "// inline explicit path\n")
        self.write("src/alternative.rs", "// explicit sibling\n")
        self.write("src/parent/child/mod.rs", "// default nested module\n")
        self.write("src/lib.rs", "mod parent;\nmod outer { mod nested; }\n")
        self.write("src/outer/nested.rs", "// inline nested module\n")
        self.write("src/registry.in", '/*\nmod not_a_module;\n*/\nconst TEXT: &str = r#"\nmod also_not_a_module;\n"#;\nmod registered;\n')
        self.write("src/registered.rs", "// included sibling\n")
        self.submit(self.commit("valid modules"))
        self.assertEqual(len(list((self.root / "queue").glob("*.json"))), 1)

    def test_e0583_prioritizes_declarer_over_shared_touched_file(self):
        self.write("src/registry.in", "mod culprit;\n")
        offender = self.commit("missing declaration")
        branches = [{"id": "bad", "commit": offender}]
        for index in range(4):
            self.run_git("reset", "-q", "--hard", self.base)
            self.write("src/registry.in", f"// unrelated {index}\n")
            branches.append({"id": f"good-{index}", "commit": self.commit(f"good {index}")})
        log = self.root / "cycle.log"
        state = {"entries": [], "checks": []}

        def merge(entries, _log):
            state["entries"] = entries[:]
            return entries[:], []

        def check(_log):
            selected = tuple(e["id"] for e in state["entries"])
            state["checks"].append(selected)
            if "bad" not in selected:
                return True
            with log.open("a") as out:
                out.write("error[E0583]: file not found for module `culprit`\n --> src/registry.in:1:1\n")
            return False

        with patch.object(train, "reset_tree", side_effect=lambda: state.update(entries=[])), \
             patch.object(train, "merge", side_effect=merge), patch.object(train, "check", side_effect=check):
            passed, rejected = train.build_checked(branches, log)
        self.assertEqual([e["id"] for e in passed], [f"good-{i}" for i in range(4)])
        self.assertEqual([e["id"] for e in rejected], ["bad"])
        self.assertEqual(state["checks"][2], tuple(f"good-{i}" for i in range(4)))
        self.assertEqual(len(state["checks"]), 4)


if __name__ == "__main__":
    unittest.main()
