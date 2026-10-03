#!/usr/bin/env python3
"""Retry the selected native server build/run after the existing 20-second delay."""

import os
import signal
import subprocess
import sys
import threading
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent


def signal_group(process, number):
    try:
        os.killpg(process.pid, number)
    except ProcessLookupError:
        pass  # The invocation has already exited.


def run_helper(command, stopped, reason):
    process = subprocess.Popen(command, start_new_session=True)
    try:
        while process.poll() is None and not stopped.wait(0.2):
            pass
        if stopped.is_set():
            signal_group(process, reason[0])
            # The common runner closes the SSH lease first, then reaps native
            # groups. Its five-second EOF wait and two-second TERM wait must
            # finish before this parent can force termination.
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                print(
                    "Native server helper did not stop after shutdown; killing its group.",
                    file=sys.stderr,
                    flush=True,
                )
                signal_group(process, signal.SIGKILL)
        return process.wait()
    finally:
        signal_group(process, signal.SIGTERM)
        signal_group(process, signal.SIGKILL)
        process.wait()


def main(argv=None):
    stopped = threading.Event()
    reason = [0]

    def stop(number, _frame):
        reason[0] = number
        stopped.set()

    handlers = {
        number: signal.signal(number, stop)
        for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)
    }
    command = [
        sys.executable,
        str(SCRIPTS / "build-host.py"),
        "--bin",
        "game-server",
        "--run",
        *(sys.argv[1:] if argv is None else argv),
    ]
    try:
        while not stopped.is_set():
            try:
                code = run_helper(command, stopped, reason)
                message = f"Native server build/run exited with code {code}."
            except OSError as error:
                message = f"Native server build/run failed: {error}."
            if stopped.is_set():
                break
            print(f"{message} Restarting in 20s...", flush=True)
            stopped.wait(20)
        return 0
    finally:
        for number, handler in handlers.items():
            signal.signal(number, handler)


if __name__ == "__main__":
    sys.exit(main())
