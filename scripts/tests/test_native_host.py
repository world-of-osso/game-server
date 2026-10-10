"""Native server adapters execute Cargo in the checkout with its real data."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SERVER_SCRIPT = Path(__file__).resolve().parents[1] / "build-host.py"
SCRIPTS = Path(os.environ.get("BUILD_HOST_SCRIPTS", SERVER_SCRIPT.parents[2] / "game-engine/scripts"))


class ServerNativeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / "game-server"
        self.root.mkdir()
        (self.root / "scripts").mkdir()
        (self.root / "scripts/build-host.py").write_bytes(SERVER_SCRIPT.read_bytes())
        (self.base / "game-engine").mkdir()
        (self.base / "game-engine/scripts").symlink_to(SCRIPTS, target_is_directory=True)
        (self.root / "data").mkdir()
        (self.root / "data/world.db").write_text("real checkout rows")
        self.bin = self.base / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
Path(os.environ['RECORD']).write_text(json.dumps(dict(args=sys.argv[1:], cwd=str(Path.cwd()), target=os.environ['CARGO_TARGET_DIR'], jobs=os.environ.get('CARGO_BUILD_JOBS'), data=Path('data/world.db').read_text())))
print('test result: ok. 1 passed; 0 failed; 0 ignored')
raise SystemExit(int(os.environ.get('CARGO_STATUS', '0')))
''')
        cargo.chmod(0o755)
        self.record = self.base / "record.json"
        self.env = dict(os.environ, HOME=str(self.base), XDG_CACHE_HOME=str(self.base / "cache"), PATH=str(self.bin) + os.pathsep + os.environ['PATH'], RECORD=str(self.record), CARGO_BUILD_JOBS="8")
        self.env.pop('BUILD_HOST_SCRIPTS', None)
        setting = self.base / '.config/game-engine/build-host'
        setting.parent.mkdir(parents=True)
        setting.write_text('native\n')

    def invoke(self, adapter, *args):
        script = SCRIPTS / 'desktop-server-build.py' if adapter == 'export' else self.root / 'scripts/build-host.py'
        return subprocess.run([sys.executable, str(script), '--root', str(self.root), *args], env=self.env, capture_output=True, text=True)

    def test_saved_native_tests_forward_arguments_and_exit_status(self):
        self.env['CARGO_STATUS'] = '7'
        for adapter in ('dev',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--test', '-p', 'server', 'fixture', '--', '--nocapture')
                self.assertEqual(result.returncode, 7, result.stderr)
                observed = json.loads(self.record.read_text())
                self.assertEqual(observed['args'], ['test', '--locked', '-p', 'server', 'fixture', '--', '--nocapture'])
                self.assertEqual(observed['cwd'], str(self.root))
                self.assertEqual(observed['target'], str(self.root / 'target'))
                self.assertIsNone(observed['jobs'])
                self.assertEqual(observed['data'], 'real checkout rows')

    def test_explicit_native_build_selects_one_binary(self):
        for adapter in ('dev',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--build-host', 'native', '--bin', 'game-cli')
                self.assertEqual(result.returncode, 0, result.stderr)
                observed = json.loads(self.record.read_text())
                self.assertEqual(observed['args'], ['build', '--locked', '-p', 'client', '--bin', 'game-cli'])
                self.assertNotIn('-j8', observed['args'])

    def test_native_release_fails_before_cargo(self):
        for adapter in ('dev',):
            with self.subTest(adapter=adapter):
                result = self.invoke(adapter, '--build-host', 'native', '--release')
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn('bookworm container', result.stderr)
                self.assertFalse(self.record.exists())


if __name__ == '__main__':
    unittest.main()
