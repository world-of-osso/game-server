"""Native adapter contracts, using real Git snapshots and a child fake transport."""

import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

ADAPTER = Path(__file__).resolve().parents[1] / "build-host.py"
COMMON = Path(os.environ.get("BUILD_HOST_SCRIPTS", ADAPTER.parents[2] / "game-engine/scripts"))
FAKE_NATIVE = '''import json
import os
from pathlib import Path
import subprocess
import sys

CHILD = r"""
import json, os
from pathlib import Path
request = json.loads(os.environ['FAKE_REQUEST'])
context = Path(request['context'])
request['files'] = {str(p.relative_to(context)): p.read_bytes().hex()
                    for p in context.rglob('*') if p.is_file()}
request['project_cwd'] = str(Path.cwd())
status = int(os.environ.get('FAKE_STATUS', '0'))
request['ran'] = status == 0 and request['runtime_args'] is not None
request['runtime_observed_cwd'] = request['runtime_cwd'] if request['ran'] else None
Path(os.environ['FAKE_RECORD']).write_text(json.dumps(request))
raise SystemExit(status)
"""

def execute(context, checkout_key, project_name, cargo_args, host,
            runtime_args=None, binary=None, release=False, environment=None,
            root=None, runtime_cwd=None):
    request = dict(context=str(context), key=checkout_key, project=project_name,
                   cargo_args=cargo_args, host=host, runtime_args=runtime_args,
                   binary=binary, release=release, environment=environment,
                   root=str(root), runtime_cwd=str(runtime_cwd) if runtime_cwd else None)
    env = dict(os.environ, FAKE_REQUEST=json.dumps(request))
    cwd = root if host == 'local' else context / project_name
    return subprocess.run([sys.executable, '-c', CHILD], cwd=cwd, env=env).returncode
'''


CONCURRENT_NATIVE = '''import json, os, subprocess, sys
from pathlib import Path

CHILD = """
import json, os, sys, time
from pathlib import Path
context, marker, release = map(Path, sys.argv[1:])
marker.write_text(json.dumps(dict(context=str(context), pid=os.getpid())))
while not release.exists():
    time.sleep(0.02)
"""

def execute(context, checkout_key, project_name, cargo_args, host, **kwargs):
    marker = Path(os.environ['CONCURRENT_MARKER'])
    if kwargs.get('runtime_args') is None:
        marker.write_text(json.dumps(dict(context=str(context))))
        return 0
    return subprocess.run([sys.executable, '-c', CHILD, str(context),
                           str(marker), os.environ['CONCURRENT_RELEASE']]).returncode
'''


class BuildHostTests(unittest.TestCase):
    def test_los_bake_build_is_explicit_and_does_not_change_default_server_tools(self):
        result = self.invoke("--bin", "los-bake", "--features", "bake")
        self.assertEqual(result.returncode, 0, result.stderr)
        request = json.loads(self.record.read_text())
        self.assertEqual(request["binary"], "los-bake")
        self.assertIn("los", request["cargo_args"])
        self.assertIn("bake", request["cargo_args"])
        self.assertFalse(request["ran"])
        result = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        request = json.loads(self.record.read_text())
        self.assertNotIn("los-bake", request["cargo_args"])
        self.assertIn("game-server", request["cargo_args"])

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base / "game-server"
        self.protocol = self.base / "shared-protocol"
        for repo in (self.root, self.protocol):
            repo.mkdir()
            self.git(repo, "init", "-q")
        self.files = {
            "Cargo.toml": b'[workspace]\nmembers = ["crates/server", "crates/client"]\n',
            "Cargo.lock": b"lock fixture\n",
            "crates/server/Cargo.toml": b'[package]\nname = "server"\n',
            "crates/server/src/main.rs": b'fn main() { println!("tracked"); }\n',
            "crates/server/src/ground_gaps.tsv": b"map\tx\ty\n0\t1\t2\n",
            "crates/client/Cargo.toml": b'[package]\nname = "client"\n',
            "vendor/bevy_replicon/Cargo.toml": b'[package]\nname = "bevy_replicon"\n',
            "vendor/bevy_replicon/src/lib.rs": b"pub fn vendored() {}\n",
            ".gitignore": b"ignored.rs\ndata/\ntarget/\n",
        }
        for name, content in self.files.items():
            self.put(self.root, name, content)
        self.put(self.protocol, "Cargo.toml", b'[package]\nname = "shared"\n')
        self.put(self.protocol, "src/lib.rs", b"pub fn protocol() {}\n")
        for repo in (self.root, self.protocol):
            self.git(repo, "add", ".")
            self.git(
                repo,
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            )
        self.put(
            self.root,
            "crates/server/src/main.rs",
            b'fn main() { println!("working bytes"); }\n',
        )
        self.put(self.root, "crates/server/src/new.rs", b"pub fn owned_new() {}\n")
        self.put(self.protocol, "src/new.rs", b"pub fn protocol_new() {}\n")
        self.put(
            self.root,
            ".cargo/config.toml",
            b'[build]\nrustflags = ["-C", "debuginfo=1"]\n',
        )
        for name in (
            "data/game.redb",
            "data/world.db",
            "data/live.rs",
            "target/generated.rs",
            ".env",
            ".cargo/credentials.toml",
            "ignored.rs",
            ".userstate/state.rs",
        ):
            self.put(self.root, name, b"LIVE PRIVATE BYTES")
        self.common = self.base / "common"
        self.common.mkdir()
        for name in ("depot-build.py", "build_hosts.py", "prepare_ui_icons.py", "db2_source.py", "native_cargo.py"):
            shutil.copyfile(COMMON / name, self.common / name)
        (self.common / "native_build_hosts.py").write_text(FAKE_NATIVE)
        self.record = self.base / "record.json"
        self.home = self.base / "home"
        self.home.mkdir()
        self.env = dict(os.environ)
        for name in list(self.env):
            if name.startswith(("GAME_SERVER_", "DEPOT_SIBLING_")):
                del self.env[name]
        self.env.update(
            HOME=str(self.home),
            XDG_CACHE_HOME=str(self.base / "cache"),
            BUILD_HOST_SCRIPTS=str(self.common),
            FAKE_RECORD=str(self.record),
            SECRET_TOKEN="do-not-forward",
            GAME_SERVER_PRIVATE_SECRET="do-not-forward",
        )

    def git(self, repo, *args):
        return subprocess.run(
            ["git", "-C", str(repo), *args], check=True, capture_output=True
        )

    def put(self, repo, name, content):
        path = repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)

    def invoke(self, *args, host="desktop"):
        command = [sys.executable, str(ADAPTER), "--root", str(self.root)]
        if host is not None:
            command += ["--build-host", host]
        return subprocess.run(
            command + list(args),
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )

    def observed(self, result):
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(self.record.read_text())

    def test_running_app_does_not_block_check_or_delete_another_context(self):
        (self.common / "native_build_hosts.py").write_text(CONCURRENT_NATIVE)
        for host in ("desktop", "local"):
            with self.subTest(host=host):
                self.exercise_concurrent_helpers(host)

    def exercise_concurrent_helpers(self, host):
        command = [
            sys.executable,
            str(ADAPTER),
            "--root",
            str(self.root),
            "--build-host",
            host,
        ]
        processes = []
        releases = []
        try:
            first, first_marker, first_release = self.start_running_helper(
                command, host + "-first", processes, releases
            )
            first_request = self.wait_for_runtime(first, first_marker)
            check_marker = self.base / (host + "-check.json")
            try:
                check = subprocess.run(
                    command + ["--check"],
                    env=dict(self.env, CONCURRENT_MARKER=str(check_marker)),
                    capture_output=True,
                    text=True,
                    timeout=3,
                    check=False,
                )
            except subprocess.TimeoutExpired:
                self.fail("check blocked behind a running app's caller lock")
            self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
            self.assertIsNone(first.poll())
            os.kill(first_request["pid"], 0)
            second, second_marker, _ = self.start_running_helper(
                command, host + "-second", processes, releases
            )
            second_request = self.wait_for_runtime(second, second_marker)
            first_context = Path(first_request["context"])
            second_context = Path(second_request["context"])
            self.assertNotEqual(first_context, second_context)
            sentinel = second_context / "runtime-sentinel"
            sentinel.write_bytes(b"second context still owned")
            first_release.touch()
            stdout, stderr = first.communicate(timeout=3)
            self.assertEqual(first.returncode, 0, stdout + stderr)
            self.assertFalse(first_context.exists())
            self.assertEqual(sentinel.read_bytes(), b"second context still owned")
            self.assertIsNone(second.poll())
            os.kill(second_request["pid"], 0)
        finally:
            for release in releases:
                release.touch()
            for process in processes:
                try:
                    process.communicate(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.communicate(timeout=3)

    def start_running_helper(self, command, name, processes, releases):
        marker = self.base / (name + ".json")
        release = self.base / (name + ".release")
        releases.append(release)
        process = subprocess.Popen(
            command + ["--run"],
            env=dict(
                self.env, CONCURRENT_MARKER=str(marker), CONCURRENT_RELEASE=str(release)
            ),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True,
        )
        processes.append(process)
        return process, marker, release

    def wait_for_runtime(self, process, marker):
        deadline = time.monotonic() + 3
        while not marker.exists():
            if process.poll() is not None:
                stdout, stderr = process.communicate()
                self.fail("helper exited before runtime: " + stdout + stderr)
            if time.monotonic() >= deadline:
                self.fail("runtime did not start within bounded timeout")
            time.sleep(0.02)
        return json.loads(marker.read_text())

    def test_site_export_tool_build_does_not_launch_server(self):
        record = self.observed(self.invoke("--bin", "game-server-site-export"))
        self.assertEqual(record["cargo_args"], ["build", "-p", "server", "--bin", "game-server-site-export"])
        self.assertFalse(record["ran"])
        self.assertIsNone(record["runtime_args"])

    def test_adapter_exists(self):
        self.assertTrue(ADAPTER.is_file(), "native game-server adapter is missing")

    def test_desktop_snapshot_uses_working_and_owned_source_not_live_state(self):
        result = self.observed(self.invoke())
        files = result["files"]
        for name in self.files:
            if name != ".gitignore":
                self.assertEqual(
                    bytes.fromhex(files["game-server/" + name]),
                    (self.root / name).read_bytes(),
                )
        for name in ("crates/server/src/new.rs", ".cargo/config.toml"):
            self.assertEqual(
                bytes.fromhex(files["game-server/" + name]),
                (self.root / name).read_bytes(),
            )
        self.assertEqual(
            bytes.fromhex(files["shared-protocol/src/new.rs"]),
            b"pub fn protocol_new() {}\n",
        )
        self.assertEqual(
            set(files),
            {"game-server/" + name for name in self.files if name != ".gitignore"}
            | {
                "game-server/crates/server/src/new.rs",
                "game-server/.cargo/config.toml",
                "shared-protocol/Cargo.toml",
                "shared-protocol/src/lib.rs",
                "shared-protocol/src/new.rs",
            },
        )
        self.assertFalse(result["ran"])
        self.assertEqual(
            result["cargo_args"],
            [
                "build",
                "-p",
                "server",
                "-p",
                "client",
                "--bin",
                "game-server",
                "--bin",
                "game-server-admin",
                "--bin",
                "game-cli",
            ],
        )
        self.assertEqual(
            result["project_cwd"], str(Path(result["context"]) / "game-server")
        )
        self.assertFalse((self.root / "target/debug/game-server").exists())
        self.assertEqual(
            (self.root / "data/game.redb").read_bytes(), b"LIVE PRIVATE BYTES"
        )

    def test_default_run_has_persistent_checkout_state_and_asset_cwd(self):
        result = self.observed(self.invoke("--run", "--", "--port", "5100", "a b"))
        key = (
            "server-"
            + hashlib.sha256(os.fsencode(self.root.resolve())).hexdigest()[:20]
        )
        self.assertEqual(result["key"], key)
        state = "/home/osso-test/.local/share/game-server/" + key
        self.assertEqual(result["runtime_args"], ["--port", "5100", "a b"])
        self.assertEqual(result["binary"], "game-server")
        self.assertEqual(
            result["environment"],
            {
                "GAME_SERVER_GROUND_DIR": "/home/osso-test/game-engine-trial/ground",
                "GAME_SERVER_DB_PATH": state + "/game.redb",
                "GAME_SERVER_ADMIN_SOCKET": state + "/admin.sock",
            },
        )
        self.assertEqual(
            result["runtime_observed_cwd"],
            "/home/osso-test/.local/share/game-server/assets",
        )
        again = self.observed(self.invoke("--run"))
        self.assertEqual(again["key"], key)
        self.assertNotEqual(again["context"], result["context"])
        self.assertFalse(Path(again["context"]).exists())
        self.assertFalse(Path(result["context"]).exists())
        self.assertEqual(again["environment"], result["environment"])

    def test_release_exports_through_container_adapter(self):
        (self.common / "desktop-server-build.py").write_text(
            "import json, os\nfrom pathlib import Path\n"
            "def build(root, host, release, binary, features=None):\n"
            "    Path(os.environ['FAKE_RECORD']).write_text(json.dumps(dict("
            "root=str(root), host=host, release=release, binary=binary, features=features)))\n"
        )
        for host in ("local", "desktop"):
            result = self.observed(self.invoke("--release", "--bin", "game-cli", host=host))
            self.assertEqual(
                result,
                dict(root=str(self.root), host=host, release=True, binary="game-cli", features=None),
            )
        result = self.observed(
            self.invoke("--release", "--bin", "game-server", "--features", "system-timings", host="local")
        )
        self.assertEqual(result["features"], "system-timings")
        for options in (("--test",), ("--check",), ("--run",)):
            result = self.invoke("--release", *options, host="local")
            self.assertEqual(result.returncode, 1)
            self.assertIn("container release exports", result.stderr)

    def test_selected_binaries_features_and_explicit_environment(self):
        self.env.update(
            GAME_SERVER_DB_PATH="/custom/game.redb",
            GAME_SERVER_ADMIN_SOCKET="/custom/admin.sock",
            GAME_SERVER_GROUND_DIR="/custom/ground",
            GAME_SERVER_PORT="5101",
            GAME_SERVER_BIND_IP="127.0.0.1",
            GAME_SERVER_DAILY_RESET_HOUR_UTC="16",
        )
        for binary, package in (
            ("game-server", "server"),
            ("game-server-admin", "server"),
            ("game-cli", "client"),
        ):
            with self.subTest(binary=binary):
                result = self.observed(
                    self.invoke(
                        "--bin",
                        binary,
                        "--features",
                        "a,b",
                        "--run",
                        "--",
                        "ping",
                    )
                )
                self.assertEqual(
                    result["cargo_args"],
                    ["build", "-p", package, "--bin", binary, "--features", "a,b"],
                )
                self.assertFalse(result["release"])
                self.assertEqual(result["binary"], binary)
                self.assertEqual(result["runtime_args"], ["ping"])
                self.assertEqual(
                    result["environment"],
                    {
                        k: v
                        for k, v in self.env.items()
                        if k
                        in {
                            "GAME_SERVER_DB_PATH",
                            "GAME_SERVER_ADMIN_SOCKET",
                            "GAME_SERVER_GROUND_DIR",
                            "GAME_SERVER_PORT",
                            "GAME_SERVER_BIND_IP",
                            "GAME_SERVER_DAILY_RESET_HOUR_UTC",
                        }
                    },
                )

    def test_local_uses_original_paths_and_does_not_install_desktop_defaults(self):
        self.env["GAME_SERVER_DB_PATH"] = str(self.root / "data/game.redb")
        result = self.observed(self.invoke("--run", "--", "serve", host="local"))
        self.assertEqual(result["project_cwd"], str(self.root))
        self.assertEqual(result["runtime_observed_cwd"], str(self.root))
        self.assertEqual(
            result["environment"],
            {"GAME_SERVER_DB_PATH": str(self.root / "data/game.redb")},
        )
        self.assertEqual(result["root"], str(self.root))
        self.assertEqual(result["files"], {})

    def test_test_tail_preserves_literal_separator_and_package_filter(self):
        result = self.observed(
            self.invoke(
                "--test",
                "-p",
                "server",
                "ground_data",
                "--",
                "--exact",
                "--nocapture",
            )
        )
        self.assertEqual(
            result["cargo_args"],
            ["test", "-p", "server", "ground_data", "--", "--exact", "--nocapture"],
        )
        self.assertFalse(result["release"])
        self.assertIsNone(result["runtime_cwd"])
        self.assertIsNone(result["runtime_args"])
        self.assertEqual(result["environment"], {})
        default = self.observed(self.invoke("--test", "--", "--nocapture"))
        self.assertEqual(
            default["cargo_args"], ["test", "--workspace", "--", "--nocapture"]
        )

    def test_check_default_and_package_filter(self):
        result = self.observed(self.invoke("--check"))
        self.assertEqual(result["cargo_args"], ["check", "--workspace"])
        result = self.observed(self.invoke("--check", "-p", "client", "--all-targets"))
        self.assertEqual(
            result["cargo_args"], ["check", "-p", "client", "--all-targets"]
        )
        self.assertIsNone(result["runtime_cwd"])

    def test_failure_returns_status_and_never_runs(self):
        self.env["FAKE_STATUS"] = "37"
        result = self.invoke("--run")
        self.assertEqual(result.returncode, 37, result.stderr)
        self.assertFalse(json.loads(self.record.read_text())["ran"])

    def test_save_only_and_saved_selection(self):
        saved = self.invoke("--save-build-host", "local", host=None)
        self.assertEqual(saved.returncode, 0, saved.stderr)
        self.assertFalse(self.record.exists())
        result = self.observed(self.invoke(host=None))
        self.assertEqual(result["host"], "local")
        invalid = self.invoke("--save-build-host", "desktop", "--run", host=None)
        self.assertNotEqual(invalid.returncode, 0)

    def test_invalid_modes_and_runtime_tail_fail_before_transport(self):
        for args in (("--check", "--run"), ("--check", "--test"), ("--", "ping")):
            with self.subTest(args=args):
                self.assertNotEqual(self.invoke(*args).returncode, 0)
                self.assertFalse(self.record.exists())

    def test_source_symlink_is_rejected_before_transport(self):
        source = self.root / "crates/server/src/new.rs"
        source.unlink()
        source.symlink_to(self.root / "data/game.redb")
        result = self.invoke("--run")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symlink", result.stderr)
        self.assertFalse(self.record.exists())


if __name__ == "__main__":
    unittest.main()
