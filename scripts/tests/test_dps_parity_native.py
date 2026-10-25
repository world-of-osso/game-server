"""Opt-in real process/mount boundary; no native gameplay or canonical DB writes."""
import hashlib
import os
from pathlib import Path
import sqlite3
import shutil
import subprocess
import sys
import tempfile
import unittest


@unittest.skipUnless(
    os.environ.get("DPS_PARITY_WORLD_DB"), "opt-in native parity database view"
)
class NativeSourceViewTests(unittest.TestCase):
    def test_process_reads_frozen_database_without_mutating_host_source(self):
        self.assert_source_view(shared_target=False)

    def test_process_accepts_explicit_shared_cargo_target_without_mutating_host_source(self):
        self.assert_source_view(shared_target=True)

    def assert_source_view(self, *, shared_target):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "data").mkdir()
            live, frozen = root / "data/world.db", root / "prepared.db"
            for path, value in [(live, "mutable"), (frozen, "frozen")]:
                with sqlite3.connect(path) as db:
                    db.execute("CREATE TABLE source_marker(value TEXT)")
                    db.execute("INSERT INTO source_marker VALUES (?)", (value,))
            before = hashlib.sha256(live.read_bytes()).hexdigest()
            frozen_before = hashlib.sha256(frozen.read_bytes()).hexdigest()
            code = """
import sqlite3, sys
with sqlite3.connect(sys.argv[1]) as db:
    print(db.execute('SELECT value FROM source_marker').fetchone()[0], flush=True)
    try:
        db.execute("INSERT INTO source_marker VALUES ('forbidden')")
    except sqlite3.OperationalError as error:
        if 'readonly' not in str(error).lower():
            raise
        print('readonly', flush=True)
    else:
        raise SystemExit('startup database was writable')
"""
            script = root / "scripts/dps_parity_native.py"
            script.parent.mkdir()
            shutil.copy2(Path(__file__).resolve().parents[1] / script.name, script)
            target = root / ("shared-target" if shared_target else "target")
            executable = target / "debug/deps/server_simulation-fixture"
            executable.parent.mkdir(parents=True)
            executable.write_text(f"#!{sys.executable}\n" + code)
            executable.chmod(0o755)
            command = [sys.executable, str(script), str(executable), str(live)]
            environment = os.environ | {"DPS_PARITY_WORLD_DB": str(frozen)}
            environment.pop("CARGO_TARGET_DIR", None)
            if shared_target:
                environment["CARGO_TARGET_DIR"] = str(target)
            result = subprocess.run(
                command, capture_output=True, text=True, timeout=30,
                env=environment,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["frozen", "readonly"])
            self.assertEqual(hashlib.sha256(live.read_bytes()).hexdigest(), before)
            self.assertEqual(hashlib.sha256(frozen.read_bytes()).hexdigest(), frozen_before)
            if shared_target:
                unselected = root / "unselected-target"
                unselected.mkdir()
                refused = subprocess.run(
                    command, capture_output=True, text=True, timeout=30,
                    env=environment | {"CARGO_TARGET_DIR": str(unselected)},
                )
                self.assertNotEqual(refused.returncode, 0)
                self.assertIn("selected Cargo target", refused.stderr)
            with sqlite3.connect(live) as db:
                self.assertEqual(
                    db.execute("SELECT value FROM source_marker").fetchall(),
                    [("mutable",)],
                )


if __name__ == "__main__":
    unittest.main()
