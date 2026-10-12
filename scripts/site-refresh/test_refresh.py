import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("refresh.py")


class RefreshTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.events = self.root / "events.jsonl"
        self.env = os.environ | {
            "PATH": f"{self.bin}:{os.environ['PATH']}",
            "TEST_EVENTS": str(self.events),
        }
        self.source = self.root / "source.redb"
        self.source.write_bytes(b"private database")
        self.optouts = self.root / "opt-outs.json"
        self.optouts.write_text("[]")
        self.exporter = self.bin / "exporter"
        self.exporter.write_text('''#!/usr/bin/env python3
import json, os, pathlib, shutil, sys
args = sys.argv[1:]
with open(os.environ['TEST_EVENTS'], 'a') as f: f.write(json.dumps(['exporter', *args]) + '\\n')
if args[0] == '--repair-copy':
    shutil.copyfile(args[1], args[2]); sys.exit(0)
if os.environ.get('TEST_EXPORT_FAIL'): sys.exit(9)
out = pathlib.Path(args[args.index('--output') + 1]); out.mkdir()
base = {'schema_version': 1, 'realm': 'world-of-osso', 'as_of': 123, 'gaps': []}
for name in ['index.json', 'rankings/mythic-plus.json', 'rankings/raids.json', 'characters/1.json']:
    path = out / name; path.parent.mkdir(exist_ok=True)
    doc = base | ({'characters': [{'url': 'characters/1.json'}]} if name == 'index.json' else {})
    if os.environ.get('TEST_BAD_REF'): doc['icon_url'] = 'icons/missing.png'
    path.write_text(json.dumps(doc))
''')
        self.exporter.chmod(0o700)
        for name in ["ssh", "scp"]:
            path = self.bin / name
            path.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
with open(os.environ['TEST_EVENTS'], 'a') as f: f.write(json.dumps([pathlib.Path(sys.argv[0]).name, *sys.argv[1:]]) + '\\n')
if os.environ.get('TEST_TRANSFER_FAIL'): sys.exit(10)
if 'cat /var/lib/game-server/data/game.redb' in ' '.join(sys.argv): sys.stdout.buffer.write(b'private database')
''')
            path.chmod(0o700)

    def refresh(self, *extra):
        return subprocess.run([
            sys.executable, str(SCRIPT), "--exporter", str(self.exporter),
            "--state-dir", str(self.root / "state"), "--opt-outs", str(self.optouts),
            "--world-db", str(self.root / "world.db"), "--textures", str(self.root),
            *extra,
        ], env=self.env, capture_output=True, text=True)

    def recorded(self):
        return [json.loads(line) for line in self.events.read_text().splitlines()]

    def test_dry_run_exports_fresh_local_snapshot_and_prunes_only_old_runs(self):
        for _ in range(4):
            result = self.refresh("--dry-run", "--source-redb", str(self.source))
            self.assertEqual(result.returncode, 0, result.stderr)
        runs = list((self.root / "state" / "runs").iterdir())
        self.assertEqual(len(runs), 3)
        self.assertTrue(all((p / "snapshot/index.json").is_file() for p in runs))
        self.assertEqual(self.source.read_bytes(), b"private database")
        self.assertTrue(all(event[0] == "exporter" for event in self.recorded()))
        self.assertIn("DRY RUN", result.stdout)

    def test_export_failure_never_stages_or_publishes(self):
        self.env["TEST_EXPORT_FAIL"] = "1"
        result = self.refresh("--source-redb", str(self.source))
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(all(event[0] == "exporter" for event in self.recorded()))
        self.assertIn("failed", result.stderr)

    def test_missing_snapshot_asset_never_stages_or_publishes(self):
        self.env["TEST_BAD_REF"] = "1"
        result = self.refresh("--source-redb", str(self.source))
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(all(event[0] == "exporter" for event in self.recorded()))

    def test_failed_source_transfer_never_repairs_or_exports(self):
        self.env["TEST_TRANSFER_FAIL"] = "1"
        result = self.refresh("--dry-run")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([event[0] for event in self.recorded()], ["ssh"])

    def test_failed_stage_never_swaps(self):
        self.env["TEST_TRANSFER_FAIL"] = "1"
        result = self.refresh("--source-redb", str(self.source))
        self.assertNotEqual(result.returncode, 0)
        events = self.recorded()
        self.assertEqual([event[0] for event in events], ["exporter", "exporter", "ssh"])

    def test_complete_snapshot_only_is_transferred_before_swap(self):
        result = self.refresh("--source-redb", str(self.source))
        self.assertEqual(result.returncode, 0, result.stderr)
        events = self.recorded()
        self.assertEqual([event[0] for event in events], ["exporter", "exporter", "ssh", "scp", "ssh"])
        self.assertIn("/snapshot/.", events[3][-2])
        self.assertNotIn("redb", events[3][-2])
        self.assertIn("--exchange --no-copy", events[4][-1])
        self.assertIn("data.old", events[4][-1])


if __name__ == "__main__":
    unittest.main()
