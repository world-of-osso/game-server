"""Opt-in real process/mount boundary; no native gameplay or canonical DB writes."""
import hashlib
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from dps_parity_native import build_native_command


@unittest.skipUnless(os.environ.get("DPS_PARITY_WORLD_DB"), "opt-in native parity database view")
class NativeSourceViewTests(unittest.TestCase):
    def test_process_reads_frozen_database_without_mutating_host_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            live, frozen = root / "world.db", root / "prepared.db"
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
    except sqlite3.OperationalError:
        print('readonly', flush=True)
    else:
        raise SystemExit('startup database was writable')
"""
            command = build_native_command(live, frozen, [sys.executable, "-c", code, str(live)])
            result = subprocess.run(command, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["frozen", "readonly"])
            self.assertEqual(hashlib.sha256(live.read_bytes()).hexdigest(), before)
            self.assertEqual(hashlib.sha256(frozen.read_bytes()).hexdigest(), frozen_before)
            with sqlite3.connect(live) as db:
                self.assertEqual(db.execute("SELECT value FROM source_marker").fetchall(), [("mutable",)])


if __name__ == "__main__":
    unittest.main()
