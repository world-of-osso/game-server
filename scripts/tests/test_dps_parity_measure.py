"""Parity builds must return artifacts from the explicitly selected shared target."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / "dps_parity_measure.py"
SPEC = importlib.util.spec_from_file_location("dps_parity_measure", SOURCE)
MEASURE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MEASURE)


class SharedTargetTests(unittest.TestCase):
    def test_build_uses_explicit_shared_target_instead_of_native_checkout_default(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            scripts = root / "scripts"
            scripts.mkdir()
            # Model native build-host's checkout target default. Explicit Cargo
            # --target-dir must win even though that helper overwrites its env.
            (scripts / "build-host.py").write_text(
                "import json, pathlib, sys\n"
                "target = pathlib.Path(__file__).resolve().parents[1] / 'target'\n"
                "if '--target-dir' in sys.argv:\n"
                "    target = pathlib.Path(sys.argv[sys.argv.index('--target-dir') + 1])\n"
                "artifact = target / 'debug/deps/server_simulation-fixture'\n"
                "artifact.parent.mkdir(parents=True, exist_ok=True)\n"
                "artifact.write_text('fixture executable')\n"
                "print(json.dumps({'reason': 'compiler-artifact', 'executable': str(artifact), "
                "'target': {'name': 'server_simulation'}}))\n"
            )
            shared = root / "shared-target"
            with patch.object(MEASURE, "ROOT", root), patch.dict(
                os.environ, {"CARGO_TARGET_DIR": str(shared)}
            ):
                executable = Path(MEASURE.build_executable(None))
            self.assertEqual(executable, shared / "debug/deps/server_simulation-fixture")
            self.assertTrue(executable.is_file())
            self.assertFalse((root / "target").exists())


if __name__ == "__main__":
    unittest.main()
