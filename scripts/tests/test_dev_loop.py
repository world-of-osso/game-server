"""Process-level contracts for the watched native server entrypoint."""

import hashlib
import json
import os
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMMON = ROOT.parent / "game-engine/scripts"

HELPER = """#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
import time
sys.path.insert(0, os.environ["BUILD_HOST_SCRIPTS"])
import native_build_hosts as native

def record(kind, **fields):
    with open(os.environ["EVENTS"], "a") as output:
        output.write(json.dumps(dict(kind=kind, at=time.monotonic(), **fields)) + "\\n")

record("helper", pid=os.getpid(), argv=sys.argv[1:])
if os.environ.get("FAIL_BUILD"):
    print("fixture build failed: no runtime launched", flush=True)
    sys.exit(7)
child = [sys.executable, str(Path(__file__).with_name("runtime.py"))]
with native.lifetime() as lease:
    result = native.run_owned(child, Path.cwd(), dict(os.environ), lease=lease,
                              relay="desktop" in sys.argv)
sys.exit(result)
"""

RUNTIME = """#!/usr/bin/env python3
import hashlib
import json
import os
from pathlib import Path
import signal
import sys
import time
sys.path.insert(0, os.environ["BUILD_HOST_SCRIPTS"])
import native_build_hosts as native
with native.lifetime(monitor="desktop" in os.environ.get("MODE", "")) as lease:
    with open(os.environ["EVENTS"], "a") as output:
        catalogs = {name: hashlib.sha256(Path("data", name).read_bytes()).hexdigest()
                    for name in ("trading-post-monthly-catalog.json", "quest-reward-bonus.json")}
        output.write(json.dumps(dict(kind="runtime", pid=os.getpid(), at=time.monotonic(),
                                    cwd=str(Path.cwd()), catalogs=catalogs)) + "\\n")
    marker = Path(os.environ["EVENTS"] + ".crashed")
    if os.environ.get("CRASH_ONCE") and not marker.exists():
        marker.touch()
        print("fixture runtime crashed", flush=True)
        sys.exit(23)
    lease[0].wait()
"""


class DevLoopTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="server-dev-loop-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "checkout with spaces"
        (self.root / "scripts").mkdir(parents=True)
        shutil.copy2(ROOT / "run-dev.sh", self.root / "run-dev.sh")
        loop = ROOT / "scripts/dev-loop.py"
        if loop.exists():
            shutil.copy2(loop, self.root / "scripts/dev-loop.py")
        (self.root / "scripts/build-host.py").write_text(HELPER)
        (self.root / "scripts/runtime.py").write_text(RUNTIME)
        (self.root / "data").mkdir()
        self.catalogs = {}
        for name in ("trading-post-monthly-catalog.json", "quest-reward-bonus.json"):
            source = ROOT / "content" / name
            shutil.copy2(source, self.root / "data" / name)
            self.catalogs[name] = hashlib.sha256(source.read_bytes()).hexdigest()
        self.events = self.root / "events.jsonl"
        self.environment = dict(
            os.environ, EVENTS=str(self.events), BUILD_HOST_SCRIPTS=str(COMMON)
        )
        self.processes = []
        self.addCleanup(self.cleanup_processes)

    def cleanup_processes(self):
        for process in self.processes:
            if process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=12)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            process.stdout.close()
            process.stderr.close()

    def records(self):
        if not self.events.exists():
            return []
        return [json.loads(line) for line in self.events.read_text().splitlines()]

    def wait_for(self, kind, count=1, timeout=4):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            records = [row for row in self.records() if row["kind"] == kind]
            if len(records) >= count:
                return records
            if self.processes and self.processes[-1].poll() is not None:
                break
            time.sleep(0.02)
        self.fail(f"missing {count} {kind} events: {self.records()}")

    def start_loop(self, *args, **environment):
        self.environment.update(environment)
        process = subprocess.Popen(
            [sys.executable, str(self.root / "scripts/dev-loop.py"), *args],
            cwd=self.root,
            env=self.environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            start_new_session=True,
        )
        self.processes.append(process)
        return process

    def assert_stopped(self, pid):
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return
            time.sleep(0.02)
        self.fail(f"orphan process {pid}")

    def test_watcher_serializes_selected_command_without_losing_arguments(self):
        cargo = self.root / "cargo"
        cargo.write_text(
            "#!/usr/bin/env python3\nimport json, os, sys\n"
            "open(os.environ['EVENTS'], 'w').write(json.dumps(sys.argv[1:]))\n"
        )
        cargo.chmod(0o755)
        environment = dict(
            self.environment, PATH=str(self.root) + os.pathsep + os.environ["PATH"]
        )
        args = [
            "--build-host",
            "local",
            "--release",
            "--features",
            "a,b",
            "--",
            "two words",
            "it's literal",
        ]
        result = subprocess.run(
            [str(self.root / "run-dev.sh"), *args],
            cwd=self.root,
            env=environment,
            capture_output=True,
            text=True,
            timeout=4,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        command = json.loads(self.events.read_text())
        self.assertEqual(
            command[:6], ["watch", "-w", "crates", "-w", "Cargo.toml", "-s"]
        )
        self.assertEqual(
            shlex.split(command[6]),
            [sys.executable, str(self.root / "scripts/dev-loop.py"), *args],
        )

    def test_default_does_not_override_shared_saved_host(self):
        process = self.start_loop()
        helper = self.wait_for("helper")[0]
        self.assertEqual(helper["argv"], ["--bin", "game-server", "--run"])
        self.wait_for("runtime")
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=12)

    def test_runtime_reads_staged_tracked_catalogs_from_checkout_data(self):
        process = self.start_loop("--build-host", "local")
        runtime = self.wait_for("runtime")[0]
        self.assertEqual(runtime["cwd"], str(self.root))
        self.assertEqual(runtime["catalogs"], self.catalogs)
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=12)
        self.assert_stopped(runtime["pid"])

    def test_selected_hosts_flags_and_shutdown_clean_separate_runtime_groups(self):
        for host in ("local", "desktop"):
            with self.subTest(host=host):
                self.events.unlink(missing_ok=True)
                args = [
                    "--build-host",
                    host,
                    "--release",
                    "--features",
                    "a,b",
                    "--",
                    "two words",
                ]
                process = self.start_loop(*args, MODE=host)
                helper = self.wait_for("helper")[0]
                runtime = self.wait_for("runtime")[0]
                self.assertEqual(
                    helper["argv"], ["--bin", "game-server", "--run", *args]
                )
                process.send_signal(signal.SIGTERM)
                process.wait(timeout=12)
                self.assert_stopped(helper["pid"])
                self.assert_stopped(runtime["pid"])
                self.assertEqual(len(self.records()), 2)

    def test_ctrl_c_and_disconnect_stop_without_restart(self):
        for number in (signal.SIGINT, signal.SIGHUP):
            with self.subTest(signal=number):
                self.events.unlink(missing_ok=True)
                process = self.start_loop("--build-host", "desktop", MODE="desktop")
                helper = self.wait_for("helper")[0]
                runtime = self.wait_for("runtime")[0]
                os.killpg(process.pid, number)
                process.wait(timeout=12)
                self.assert_stopped(helper["pid"])
                self.assert_stopped(runtime["pid"])
                self.assertEqual(len(self.records()), 2)

    def test_build_failure_logs_retry_but_never_launches_runtime(self):
        process = self.start_loop(FAIL_BUILD="1")
        self.wait_for("helper")
        time.sleep(0.3)
        process.send_signal(signal.SIGTERM)
        stdout, stderr = process.communicate(timeout=12)
        self.assertIn("fixture build failed", stdout)
        self.assertIn("code 7", stdout + stderr)
        self.assertIn("20s", stdout + stderr)
        self.assertFalse(any(row["kind"] == "runtime" for row in self.records()))
        self.assertEqual(len(self.records()), 1)

    def test_crash_retries_after_existing_twenty_second_deadline(self):
        process = self.start_loop(CRASH_ONCE="1")
        first = self.wait_for("runtime")[0]
        second = self.wait_for("runtime", count=2, timeout=23)[1]
        self.assertGreaterEqual(second["at"] - first["at"], 20)
        self.assertLess(second["at"] - first["at"], 22)
        process.send_signal(signal.SIGTERM)
        stdout, stderr = process.communicate(timeout=12)
        self.assertIn("code 23", stdout + stderr)
        self.assertIn("20s", stdout + stderr)
        for row in self.records():
            self.assert_stopped(row["pid"])


if __name__ == "__main__":
    unittest.main()
