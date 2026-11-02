"""Reach-audit failures name the script modules that become bisect suspects."""
import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "merge_train.py"
spec = importlib.util.spec_from_file_location("merge_train_audit_suspects", SCRIPT)
train = importlib.util.module_from_spec(spec)
spec.loader.exec_module(train)

MODULES = "crates/server-spell-scripts/src/modules"


class AuditSuspectTests(unittest.TestCase):
    def test_undeclared_and_dynamic_children_name_their_modules(self):
        text = (
            "598 literal children, 226 dynamic sites, 3 audit errors\n"
            "feral_claw_rampage.rs: child 441585 is undeclared\n"
            "('shadow_cthun.rs', 'try_summon', 'load_triggered_spell', 'id'): "
            "dynamic child needs an explicit declaration (1 site(s))\n"
        )
        self.assertEqual(
            train.audit_failing_paths(text),
            {f"{MODULES}/feral_claw_rampage.rs", f"{MODULES}/shadow_cthun.rs"},
        )

    def test_target_selection_diff_names_only_changed_sites(self):
        text = (
            '  left: {("a.rs", "f", "query"): 1, ("dir/b.rs", "g", "area"): 2}\n'
            ' right: {("a.rs", "f", "query"): 1, ("dir/b.rs", "g", "area"): 1, '
            '("c.rs", "h", "area"): 1}\n'
        )
        self.assertEqual(
            train.audit_failing_paths(text),
            {f"{MODULES}/dir/b.rs", f"{MODULES}/c.rs"},
        )

    def test_failing_paths_include_audit_modules(self):
        import tempfile
        with tempfile.TemporaryDirectory() as temp:
            log = Path(temp) / "cycle.log"
            log.write_text("x.rs: child 1 is undeclared\n")
            self.assertIn(f"{MODULES}/x.rs", train.failing_paths(log, 0))


if __name__ == "__main__":
    unittest.main()
