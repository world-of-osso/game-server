"""Deployment requires graphs for every selected baked navigation map."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_navigation_graphs import check_graphs


class NavigationGraphDeploymentTest(unittest.TestCase):
    def test_selected_baked_map_requires_a_nonempty_graph(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            selected = root / "2519"
            selected.mkdir()
            (selected / "30_30.nav").write_bytes(b"tile")
            with self.assertRaisesRegex(ValueError, "navigation graph missing or empty"):
                check_graphs(root, {2519})
            graph = selected / "portal-graph.json"
            graph.write_bytes(b"")
            with self.assertRaisesRegex(ValueError, "navigation graph missing or empty"):
                check_graphs(root, {2519})
            graph.write_text("{}")
            unselected = root / "2520"
            unselected.mkdir()
            (unselected / "30_30.nav").write_bytes(b"tile")
            check_graphs(root, {2519})
            with self.assertRaisesRegex(ValueError, "navigation graph missing or empty"):
                check_graphs(root, {2519, 2520})

    def test_missing_navigation_directory_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "navigation directory missing"):
                check_graphs(Path(temporary) / "absent", {2519})


if __name__ == "__main__":
    unittest.main()
