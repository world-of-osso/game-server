"""A superseding entry keeps the gates of the entry it replaces."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)


def entry(entry_id, commit, submitted, tests, specs):
    return {"id": entry_id, "owner": "parity-260", "commit": commit, "land": True,
            "submitted": submitted, "tests": tests,
            "measure": [{"spec": s, "output": f"out-{entry_id}-{s}"} for s in specs]}


class SupersedeTests(unittest.TestCase):
    def test_descendant_inherits_the_ancestor_tests_and_measured_specs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            poison = entry("poison", "aaa", 1, ["-p server-combat poison_copy"], ["259", "260", "261"])
            fazed = entry("fazed", "bbb", 2, ["-p server-combat fazed"], ["260"])
            ancestor = subprocess.CompletedProcess([], 0)
            with patch.object(train, "RUNNING", root / "running"), \
                    patch.object(train, "RESULTS", root / "results"), \
                    patch.object(train, "git", return_value=ancestor):
                (root / "running").mkdir()
                (root / "results").mkdir()
                kept = train.drop_superseded([poison, fazed])

                self.assertEqual([e["id"] for e in kept], ["fazed"])
                self.assertEqual(fazed["tests"], ["-p server-combat fazed", "-p server-combat poison_copy"])
                self.assertEqual([(m["spec"], m["output"]) for m in fazed["measure"]],
                                 [("260", "out-fazed-260"), ("259", "out-poison-259"), ("261", "out-poison-261")])
                self.assertEqual(json.loads((root / "running" / "fazed.json").read_text()), fazed)
                self.assertEqual(json.loads((root / "results" / "poison.json").read_text())["reason"],
                                 "superseded by fazed")


if __name__ == "__main__":
    unittest.main()
